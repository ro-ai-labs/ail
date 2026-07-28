#!/usr/bin/env python3
"""AIL self-hosting bootstrap chain runner.

AIL should bootstrap the way Rust did: Rust builds the first AIL compiler
(stage0); the binary of that first compiler compiles the next compiler,
written in AIL (stage1); stage1 compiles the same source again (stage2); and
stage1's output must equal stage2's output byte for byte (the fixpoint). That
byte-identical fixpoint is the same check rustc runs, and it is the real
definition of self-hosting -- not "a compiler exists" but "the language can
reproduce its own toolchain."

    stage0    target/debug/ail                   Rust-written, exists today
    stage1    stage0 compiles compiler.ail   ->   ail-stage1
    stage2    stage1 compiles compiler.ail   ->   ail-stage2
    FIXPOINT  stage1 output == stage2 output, byte for byte

This is NOT `cargo run -- ail-bootstrap`. That existing command composes a
compiler *pass* over an agent package inside a single Rust-built binary --
useful, but a fixed-point check on one pass, not a chain of compiler
generations. This script measures the chain described in
docs/ail/13-bootstrap-self-hosting.md: whether stage0 can produce a stage1
that can reproduce itself.

CRITICAL, and the entire point of this script: AIL today cannot express a
compiler. It cannot add two runtime values, has no loops reachable from
source, no recursive data structures, and function calls do not return
values. So `bootstrap/compiler.ail` cannot exist yet, and this script reports
that truthfully -- it does not fabricate a passing chain, and it does not
quietly succeed by doing something trivial. See `bootstrap/README.md` for the
stage model and --status output below for exactly what blocks stage1 today,
in the order the capability ladder plans to close it.

Usage:
    python3 scripts/ail_bootstrap_chain.py --status   # how far can the chain get today
    python3 scripts/ail_bootstrap_chain.py --run      # attempt the full chain

Exit codes:
    0  fixpoint reached (stage1 == stage2, byte for byte)
    1  chain incomplete but scaffolding healthy (the expected state today)
    2  scaffolding or precondition error (e.g. no stage0 binary at all)
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BOOTSTRAP = ROOT / "bootstrap"
COMPILER_SOURCE = BOOTSTRAP / "compiler.ail"
BUILD_DIR = BOOTSTRAP / "build"

STAGE0_CANDIDATES = (ROOT / "target/debug/ail", ROOT / "target/release/ail")

CHECK_TIMEOUT = 60
BUILD_TIMEOUT = 300

# The compiler-pass "action" ail-compile is invoked with, once compiler.ail
# exists. A forward-looking convention, not yet exercised: read `action:`
# from compiler.ail/ail-package.md if present, else fall back to this.
DEFAULT_ACTION = "Compile"

TARGET = "linux-x86_64-elf"


@dataclass
class Blocker:
    """One concrete language capability standing between AIL and stage1."""

    order: int
    phase: str
    capability: str
    why: str
    reference: str


# Ordered by the phase in which docs/ail/32-capability-ladder-and-improvement-loop.md
# plans to close it (Phase 1 computation, Phase 2 data, Phase 4 effects/IO, then
# the Stage 3 self-hosting boundary from docs/ail/13) -- i.e. the order in which
# the improvement loop is actually expected to unblock these, not a ranking of
# which one feels most essential to a compiler in the abstract.
ORDERED_BLOCKERS: tuple[Blocker, ...] = (
    Blocker(
        1,
        "Phase 1 -- computation",
        "typed expression IR with runtime (non-literal) operands, plus general "
        "arithmetic/comparison opcodes",
        "AIL cannot add two runtime values today. ADD_INT_FIELD is the only "
        "arithmetic opcode and its delta must be a compile-time literal; a "
        "variable operand silently degrades to WRITE_FIELD (English prose "
        "stored as text, no diagnostic).",
        "bench/R001-add, R002-arithmetic, R003-max, R004-abs-sign",
    ),
    Blocker(
        2,
        "Phase 1 -- computation",
        "function calls that return values, with real call frames",
        "A compiler is built from functions that consume input and produce a "
        "result (tokenize, parse, check, lower). FUNCTION_CALL/FUNCTION_RETURN "
        "currently push a trace line and fall through; factorial(5) returns "
        "nothing.",
        "bench/R005-factorial, R006-fibonacci",
    ),
    Blocker(
        3,
        "Phase 1 -- computation",
        "loops reachable from source",
        "Scanning source text and walking a token/AST list needs unbounded "
        "iteration. The native x86-64 emitter explicitly rejects backward "
        "jumps and REPEAT_ACTION is unrolled at compile time -- every native "
        "AIL binary is loop-free by construction.",
        "bench/R007-sum-to-n, R008-gcd, R009-collatz, R010-is-prime",
    ),
    Blocker(
        4,
        "Phase 2 -- data",
        "recursive algebraic data types with a real tail",
        "A token stream and an AST are both exactly this shape "
        "(Cons(head, tail) / Node(left, right)). List<T> is declared "
        "Empty | Items(values: T) today -- no tail -- which independently "
        "rules out self-hosting.",
        "capability ladder Tier 1 rung R019 (not yet materialized under bench/)",
    ),
    Blocker(
        5,
        "Phase 2 -- data",
        "string processing: indexing, slicing, comparison, concatenation, "
        "character classification",
        "A lexer is a string-processing loop. No string opcode exists today "
        "beyond storing prose as an opaque text payload.",
        "capability ladder Tier 1 rung R020 (not yet materialized under bench/)",
    ),
    Blocker(
        6,
        "Phase 4 -- effects and I/O",
        "file I/O from AIL source itself: read the compiler's own source "
        "text, write the next stage's bytecode/ELF bytes",
        "Stage1 must read compiler.ail and write stage2's bytes. No "
        "file-read/write effect exists in the language yet.",
        "capability ladder Tier 3, rungs R031 onward",
    ),
    Blocker(
        7,
        "Stage 3 -- self-hosting boundary",
        "an AIL-expressible code-generation target (bytecode lowering and "
        "native emission as AIL computation, not merely invoked by AIL on "
        "Rust's behalf)",
        "Even once compiler.ail exists and runs, its output only counts as "
        "self-hosted if code generation is itself AIL computation over AIL "
        "data. Otherwise stage1 is AIL wrapping a hidden Rust cross-compile "
        "step.",
        "docs/ail/13-bootstrap-self-hosting.md, Stage 3",
    ),
)


@dataclass
class StageResult:
    stage: str
    status: str  # ready | blocked | built | error
    detail: str
    fingerprint: str | None = None
    path: Path | None = None
    blockers: tuple[Blocker, ...] = field(default_factory=tuple)


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def fingerprint_bytes(data: bytes) -> str:
    return f"sha256:{hashlib.sha256(data).hexdigest()}"


def run_cmd(command: list[str], timeout: int) -> tuple[int, str]:
    """Run a command, returning its exit code and combined output."""
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return 124, f"timeout after {timeout}s"
    except OSError as error:
        return 127, f"failed to execute {command[0]}: {error}"
    return completed.returncode, completed.stdout + completed.stderr


def last_line(output: str) -> str:
    stripped = output.strip()
    return stripped.splitlines()[-1] if stripped else "(no output)"


def read_action(source: Path) -> str:
    """Read an 'action: X' line from source/ail-package.md, else the default."""
    manifest = source / "ail-package.md"
    if manifest.is_file():
        for line in manifest.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.lower().startswith("action:"):
                value = line.split(":", 1)[1].strip()
                if value:
                    return value
    return DEFAULT_ACTION


def eval_stage0() -> StageResult:
    for candidate in STAGE0_CANDIDATES:
        if candidate.is_file():
            data = candidate.read_bytes()
            fp = fingerprint_bytes(data)
            return StageResult(
                stage="stage0",
                status="ready",
                detail=f"{rel(candidate)} ({len(data)} bytes, Rust-written)",
                fingerprint=fp,
                path=candidate,
            )
    return StageResult(
        stage="stage0",
        status="error",
        detail=(
            "no stage0 binary found at target/debug/ail or target/release/ail. "
            "This script does not build the Rust toolchain -- run `cargo build` "
            "first; the bootstrap chain has no premise without it."
        ),
    )


def eval_stage1(stage0: StageResult, execute: bool) -> StageResult:
    if stage0.status != "ready":
        return StageResult(
            stage="stage1", status="blocked", detail="stage0 is unavailable"
        )

    if not COMPILER_SOURCE.is_dir():
        return StageResult(
            stage="stage1",
            status="blocked",
            detail=(
                f"{rel(COMPILER_SOURCE)} does not exist: AIL cannot yet express "
                "a compiler"
            ),
            blockers=ORDERED_BLOCKERS,
        )

    assert stage0.path is not None
    ail = stage0.path
    code, output = run_cmd(
        [str(ail), "ail-check", str(COMPILER_SOURCE)], timeout=CHECK_TIMEOUT
    )
    if code != 0 or "ail diagnostics: none" not in output:
        return StageResult(
            stage="stage1",
            status="blocked",
            detail=f"ail-check rejected {rel(COMPILER_SOURCE)}: {last_line(output)}",
        )

    if not execute:
        return StageResult(
            stage="stage1",
            status="ready-to-build",
            detail=(
                f"{rel(COMPILER_SOURCE)} checks clean; rerun with --run to "
                "attempt the native build"
            ),
        )

    action = read_action(COMPILER_SOURCE)
    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    out_path = BUILD_DIR / "ail-stage1"
    code, output = run_cmd(
        [
            str(ail),
            "ail-compile",
            str(COMPILER_SOURCE),
            "--action",
            action,
            "--target",
            TARGET,
            "--out",
            str(out_path),
        ],
        timeout=BUILD_TIMEOUT,
    )
    if code != 0 or not out_path.is_file():
        return StageResult(
            stage="stage1",
            status="error",
            detail=f"native build failed: {last_line(output)}",
        )

    data = out_path.read_bytes()
    return StageResult(
        stage="stage1",
        status="built",
        detail=f"{rel(out_path)} built via action {action!r}",
        fingerprint=fingerprint_bytes(data),
        path=out_path,
    )


def eval_stage2(stage1: StageResult, execute: bool) -> StageResult:
    if stage1.status != "built":
        reason = (
            "stage1 was not built"
            if stage1.status in ("blocked", "error")
            else "stage1 was not attempted (rerun with --run)"
        )
        return StageResult(
            stage="stage2",
            status="blocked",
            detail=f"blocked transitively: {reason}",
        )
    if not execute:
        return StageResult(
            stage="stage2",
            status="blocked",
            detail="not attempted (rerun with --run)",
        )

    assert stage1.path is not None
    stage1_binary = stage1.path
    action = read_action(COMPILER_SOURCE)
    out_path = BUILD_DIR / "ail-stage2"
    # This assumes ail-stage1 implements the same ail-compile CLI surface as
    # stage0 -- the whole premise of "AIL bootstraps the way Rust did" is that
    # it does. If it doesn't, that failure is itself informative and is
    # reported here rather than papered over.
    code, output = run_cmd(
        [
            str(stage1_binary),
            "ail-compile",
            str(COMPILER_SOURCE),
            "--action",
            action,
            "--target",
            TARGET,
            "--out",
            str(out_path),
        ],
        timeout=BUILD_TIMEOUT,
    )
    if code != 0 or not out_path.is_file():
        return StageResult(
            stage="stage2",
            status="error",
            detail=(
                f"ail-stage1 failed to compile itself a second time: "
                f"{last_line(output)}"
            ),
        )

    data = out_path.read_bytes()
    return StageResult(
        stage="stage2",
        status="built",
        detail=f"{rel(out_path)} built via action {action!r}",
        fingerprint=fingerprint_bytes(data),
        path=out_path,
    )


def eval_fixpoint(stage1: StageResult, stage2: StageResult) -> StageResult:
    if stage1.status != "built" or stage2.status != "built":
        return StageResult(
            stage="fixpoint",
            status="blocked",
            detail="not reached: stage1 and/or stage2 artifacts are unavailable",
        )
    assert stage1.path is not None and stage2.path is not None
    stage1_bytes = stage1.path.read_bytes()
    stage2_bytes = stage2.path.read_bytes()
    if stage1_bytes == stage2_bytes:
        return StageResult(
            stage="fixpoint",
            status="reached",
            detail=(
                f"stage1 == stage2, byte for byte ({len(stage1_bytes)} bytes, "
                f"{stage1.fingerprint})"
            ),
        )
    return StageResult(
        stage="fixpoint",
        status="diverged",
        detail=(
            f"stage1 ({stage1.fingerprint}) != stage2 ({stage2.fingerprint}): "
            "not self-hosting yet, even though both stages built"
        ),
    )


MARKERS = {
    "ready": "READY",
    "ready-to-build": "READY*",
    "built": "BUILT",
    "reached": "FIXED",
    "blocked": "BLOCKED",
    "diverged": "DIVERGED",
    "error": "ERROR",
}


def print_stage(result: StageResult) -> None:
    marker = MARKERS.get(result.status, result.status.upper())
    print(f"{marker:>8}  {result.stage:<10} {result.detail}")
    if result.fingerprint:
        print(f"{'':>8}  {'':<10} fingerprint {result.fingerprint}")


def print_blockers(blockers: tuple[Blocker, ...]) -> None:
    print()
    print(
        "blocked-on (ordered; stage1 cannot be attempted until all of these "
        "close):"
    )
    for blocker in blockers:
        print(f"  {blocker.order}. [{blocker.phase}] {blocker.capability}")
        print(f"     why: {blocker.why}")
        print(f"     see: {blocker.reference}")


def evaluate_chain(execute: bool) -> tuple[StageResult, StageResult, StageResult, StageResult]:
    stage0 = eval_stage0()
    stage1 = eval_stage1(stage0, execute=execute)
    stage2 = eval_stage2(stage1, execute=execute)
    fixpoint = eval_fixpoint(stage1, stage2)
    return stage0, stage1, stage2, fixpoint


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Report on / attempt the AIL self-hosting bootstrap chain."
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--status",
        action="store_true",
        help="report how far the chain can currently get, without building anything",
    )
    mode.add_argument(
        "--run",
        action="store_true",
        help="attempt the full chain, stopping at the first stage that cannot be built",
    )
    args = parser.parse_args()

    execute = bool(args.run)
    stage0, stage1, stage2, fixpoint = evaluate_chain(execute=execute)

    mode_label = "run" if execute else "status"
    print(f"AIL self-hosting bootstrap chain -- {mode_label}")
    print()
    for result in (stage0, stage1, stage2, fixpoint):
        print_stage(result)
    if stage1.blockers:
        print_blockers(stage1.blockers)

    built = sum(1 for r in (stage0, stage1, stage2) if r.status in ("ready", "built"))
    print()
    print(
        f"bootstrap chain: {built}/3 stages ready or built; "
        f"fixpoint {fixpoint.status}"
    )

    if stage0.status != "ready":
        print()
        print("SCAFFOLDING ERROR: stage0 precondition not met.", file=sys.stderr)
        return 2

    if fixpoint.status == "reached":
        return 0

    print()
    print(
        "chain incomplete -- expected today; see blocked-on list above for what "
        "closes it."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
