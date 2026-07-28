#!/usr/bin/env python3
"""AIL capability ladder runner.

Computes the single scalar that measures whether AIL can express, compile, and
execute real programs. See docs/ail/32-capability-ladder-and-improvement-loop.md.

A rung passes only when, for every case:

    reference output == VM final state == native final state

The reference oracle is an independent Python program. Expected outputs are
recomputed from it on every run and never stored, so a score cannot be moved by
editing an expectation. Rung inputs are fingerprinted into bench/SCORE.lock, so
a score cannot be moved by editing a rung.

Usage:
    python3 scripts/ail_bench.py                     # score every rung
    python3 scripts/ail_bench.py --rung R001-add     # one rung, verbose
    python3 scripts/ail_bench.py --update-lock       # record a new high score
    python3 scripts/ail_bench.py --bless-ladder      # accept edited rung inputs
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / "bench"
LOCK = BENCH / "SCORE.lock"

# A runtime state line: dotted lowercase key, '=', value. Excludes 'trace='.
STATE_LINE = re.compile(r"^(?!trace=)([a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*)=(.*)$")

CASE_TIMEOUT = 60
BUILD_TIMEOUT = 300


class RungError(Exception):
    """A rung is malformed. Distinct from a rung that merely fails."""


def ail_binary() -> Path:
    """Locate the ail binary, preferring an existing build over a fresh one."""
    for candidate in (ROOT / "target/debug/ail", ROOT / "target/release/ail"):
        if candidate.is_file():
            return candidate
    subprocess.run(
        ["cargo", "build"], cwd=ROOT, check=True, capture_output=True, timeout=BUILD_TIMEOUT
    )
    return ROOT / "target/debug/ail"


def read_metadata(path: Path) -> dict[str, str]:
    """Read leading 'key: value' lines from a rung's intent.md."""
    metadata: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            break
        key, value = line.split(":", 1)
        if " " in key.strip():
            break
        metadata[key.strip()] = value.strip()
    return metadata


def ladder_fingerprint(rung_dir: Path) -> str:
    """Fingerprint every rung input except the AIL implementation.

    program.ail/ is deliberately excluded: it is the thing under test and is
    expected to change. Everything else defines the rung and is frozen.
    """
    digest = hashlib.sha256()
    for path in sorted(rung_dir.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(rung_dir)
        if relative.parts and relative.parts[0] == "program.ail":
            continue
        digest.update(str(relative).encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()[:32]}"


def state_lines(text: str) -> list[str]:
    """Extract sorted runtime state assignments from toolchain or binary output."""
    found = []
    for line in text.splitlines():
        match = STATE_LINE.match(line.strip())
        if match:
            found.append(f"{match.group(1)}={match.group(2)}")
    return sorted(set(found))


def run(command: list[str], cwd: Path | None = None) -> tuple[int, str]:
    """Run a command, returning its exit code and combined output."""
    try:
        completed = subprocess.run(
            command,
            cwd=cwd or ROOT,
            capture_output=True,
            text=True,
            timeout=CASE_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return 124, f"timeout after {CASE_TIMEOUT}s"
    return completed.returncode, completed.stdout + completed.stderr


def discover_rungs() -> list[Path]:
    if not BENCH.is_dir():
        return []
    return sorted(p for p in BENCH.iterdir() if p.is_dir() and re.match(r"^R\d{3}-", p.name))


def load_cases(rung_dir: Path) -> list[tuple[str, list[str]]]:
    """Load each case as (name, args). Args are 'key=value' lines."""
    cases_dir = rung_dir / "cases"
    if not cases_dir.is_dir():
        raise RungError("missing cases/ directory")
    cases = []
    for path in sorted(cases_dir.glob("*.args")):
        args = [
            line.strip()
            for line in path.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.strip().startswith("#")
        ]
        cases.append((path.stem, args))
    if not cases:
        raise RungError("cases/ contains no .args files")
    return cases


def evaluate_rung(rung_dir: Path, ail: Path, verbose: bool) -> dict[str, object]:
    """Score one rung. Returns a result record; never raises on mere failure."""
    name = rung_dir.name
    result: dict[str, object] = {
        "rung": name,
        "status": "fail",
        "detail": "",
        "fingerprint": ladder_fingerprint(rung_dir),
    }

    intent = rung_dir / "intent.md"
    if not intent.is_file():
        result["detail"] = "missing intent.md"
        return result
    metadata = read_metadata(intent)
    action = metadata.get("action")
    if not action:
        result["detail"] = "intent.md does not declare an 'action:'"
        return result
    targets = [t.strip() for t in metadata.get("targets", "vm,native").split(",") if t.strip()]

    try:
        cases = load_cases(rung_dir)
    except RungError as error:
        result["detail"] = str(error)
        return result

    oracle = rung_dir / "reference.py"
    if not oracle.is_file():
        result["detail"] = "missing reference.py oracle"
        return result

    package = rung_dir / "program.ail"
    if not package.is_dir():
        result["status"] = "unimplemented"
        result["detail"] = f"no program.ail (blocked on: {metadata.get('blocked-on', 'unknown')})"
        return result

    code, output = run([str(ail), "ail-check", str(package)])
    if code != 0 or "ail diagnostics: none" not in output:
        result["status"] = "check-failed"
        result["detail"] = next(
            (line for line in output.splitlines() if line.startswith("AIL-")),
            output.strip().splitlines()[-1] if output.strip() else "ail-check failed",
        )
        return result

    native_binary = None
    workspace = None
    if "native" in targets:
        workspace = Path(tempfile.mkdtemp(prefix=f"ail-bench-{name}-"))
        native_binary = workspace / "program"
        code, output = run(
            [
                str(ail),
                "ail-compile",
                str(package),
                "--action",
                action,
                "--target",
                "linux-x86_64-elf",
                "--out",
                str(native_binary),
            ]
        )
        if code != 0 or not native_binary.is_file():
            shutil.rmtree(workspace, ignore_errors=True)
            result["status"] = "compile-failed"
            result["detail"] = output.strip().splitlines()[-1] if output.strip() else "no output"
            return result

    try:
        for case_name, args in cases:
            code, oracle_output = run([sys.executable, str(oracle), *args])
            if code != 0:
                result["detail"] = f"{case_name}: oracle failed: {oracle_output.strip()}"
                return result
            expected = state_lines(oracle_output)
            if not expected:
                result["detail"] = f"{case_name}: oracle produced no state lines"
                return result

            if "vm" in targets:
                _, vm_output = run(
                    [str(ail), "ail-run", str(package), "--action", action, *args]
                )
                actual = state_lines(vm_output)
                if actual != expected:
                    result["detail"] = (
                        f"{case_name}: vm mismatch expected={expected} actual={actual}"
                    )
                    if verbose:
                        print(f"    vm output:\n{vm_output}", file=sys.stderr)
                    return result

            if native_binary is not None:
                _, native_output = run([str(native_binary), *args])
                actual = state_lines(native_output)
                if actual != expected:
                    result["detail"] = (
                        f"{case_name}: native mismatch expected={expected} actual={actual}"
                    )
                    if verbose:
                        print(f"    native output:\n{native_output}", file=sys.stderr)
                    return result
    finally:
        if workspace is not None:
            shutil.rmtree(workspace, ignore_errors=True)

    result["status"] = "pass"
    result["detail"] = f"{len(cases)} cases on {'+'.join(targets)}"
    return result


def read_lock() -> dict[str, str]:
    if not LOCK.is_file():
        return {}
    entries = {}
    for line in LOCK.read_text(encoding="utf-8").splitlines():
        # rung <name> <pass|fail> ladder-fingerprint <sha256:...>
        parts = line.split()
        if len(parts) >= 5 and parts[0] == "rung" and parts[3] == "ladder-fingerprint":
            entries[parts[1]] = parts[4]
    return entries


def read_lock_best() -> int:
    if not LOCK.is_file():
        return -1
    for line in LOCK.read_text(encoding="utf-8").splitlines():
        if line.startswith("best-score "):
            return int(line.split()[1])
    return -1


def read_lock_passing() -> set[str]:
    if not LOCK.is_file():
        return set()
    passing = set()
    for line in LOCK.read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[0] == "rung" and parts[2] == "pass":
            passing.add(parts[1])
    return passing


def write_lock(results: list[dict[str, object]], score: int) -> None:
    lines = [
        "ail-bench-lock v1",
        "# Highest score ever achieved, and the exact rung set that achieved it.",
        "# The loop may never commit a change that drops a rung out of this set.",
        f"best-score {score}",
        f"best-total {len(results)}",
    ]
    for result in results:
        status = "pass" if result["status"] == "pass" else "fail"
        lines.append(
            f"rung {result['rung']} {status} ladder-fingerprint {result['fingerprint']}"
        )
    LOCK.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the AIL capability ladder.")
    parser.add_argument("--rung", help="evaluate a single rung by directory name")
    parser.add_argument(
        "--update-lock", action="store_true", help="record the current score as the new high score"
    )
    parser.add_argument(
        "--bless-ladder",
        action="store_true",
        help="accept modified rung inputs (deliberate human action)",
    )
    parser.add_argument("--verbose", action="store_true", help="show failing command output")
    args = parser.parse_args()

    rungs = discover_rungs()
    if args.rung:
        rungs = [r for r in rungs if r.name == args.rung]
        if not rungs:
            print(f"no such rung: {args.rung}", file=sys.stderr)
            return 2
    if not rungs:
        print("no rungs found under bench/", file=sys.stderr)
        return 2

    ail = ail_binary()
    locked = read_lock()
    results = []
    tampered = []

    for rung_dir in rungs:
        result = evaluate_rung(rung_dir, ail, args.verbose)
        results.append(result)
        expected_fingerprint = locked.get(result["rung"])
        if (
            expected_fingerprint
            and expected_fingerprint != result["fingerprint"]
            and not args.bless_ladder
        ):
            tampered.append(result["rung"])
        marker = {
            "pass": "PASS",
            "unimplemented": "----",
            "check-failed": "CHECK",
            "compile-failed": "COMPILE",
        }.get(str(result["status"]), "FAIL")
        print(f"{marker:>7}  {result['rung']:<28} {result['detail']}")

    score = sum(1 for r in results if r["status"] == "pass")
    total = len(results)
    print()
    print(f"ail-bench score {score}/{total}")

    if tampered:
        print(file=sys.stderr)
        print(
            f"LADDER TAMPERED: rung inputs changed for {', '.join(tampered)}.\n"
            "Rungs are frozen once committed. Re-run with --bless-ladder only if a\n"
            "human deliberately revised the rung definition.",
            file=sys.stderr,
        )
        return 3

    if args.rung:
        return 0 if score == total else 1

    # A rung recorded in the lock but absent from the tree shrinks the denominator
    # and inflates the ratio. Removing a rung is a deliberate human act, never a
    # side effect of a loop iteration.
    present = {r.name for r in rungs}
    removed = sorted(set(locked) - present)
    if removed and not args.bless_ladder:
        print(file=sys.stderr)
        print(
            f"LADDER INCOMPLETE: rungs recorded in SCORE.lock are missing from bench/: "
            f"{', '.join(removed)}.\n"
            "Deleting a failing rung raises the ratio without improving the language.",
            file=sys.stderr,
        )
        return 3

    best = read_lock_best()
    previously_passing = read_lock_passing()
    now_passing = {str(r["rung"]) for r in results if r["status"] == "pass"}
    regressed = sorted(previously_passing - now_passing)

    if regressed:
        print(file=sys.stderr)
        print(f"RATCHET BROKEN: previously passing rungs now fail: {', '.join(regressed)}", file=sys.stderr)
        return 4

    if args.update_lock:
        if score < best:
            print(f"refusing to lower best-score from {best} to {score}", file=sys.stderr)
            return 4
        write_lock(results, score)
        print(f"SCORE.lock updated: best-score {score}/{total}")
        return 0

    if best >= 0 and score > best:
        print(f"new high score ({best} -> {score}); re-run with --update-lock to record it")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
