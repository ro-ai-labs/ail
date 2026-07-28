#!/usr/bin/env python3
"""Differential test: the bytecode VM and the native backend must agree.

For every package and every compilable action in the corpus, this runs the same
inputs through both execution paths and compares the resulting runtime state and
failure outcome. Nothing else in the repository does this, so a codegen
regression that changes what a native binary computes — while leaving every
report, manifest, and fingerprint identical — is currently invisible.

One such divergence is already latent: the native integer-delta annotator resets
its accumulator on LABEL/BRANCH/JUMP, so once the spec compiler emits a branch
followed by an add, native and VM results will differ silently. This test is the
guard for that class of bug.

Usage:
    python3 scripts/ail_differential.py
    python3 scripts/ail_differential.py --package examples/stateful_counter.ail
    python3 scripts/ail_differential.py --verbose
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TIMEOUT = 60

# 'trace=' and 'failure=' are outcome markers the VM prints, not runtime state.
# The native backend prints the same information in a different shape
# ('failure NotFound'), so comparing them as state would report a divergence
# where the two backends actually agree.
STATE_LINE = re.compile(r"^(?!trace=|failure=)([a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*)=(.*)$")
FAILURE_LINE = re.compile(r"^(?:failure[= ]|ail-run failed)")


def ail_binary() -> Path:
    for candidate in (ROOT / "target/debug/ail", ROOT / "target/release/ail"):
        if candidate.is_file():
            return candidate
    raise SystemExit("no ail binary found; run 'cargo build' first")


def run(command: list[str]) -> tuple[int, str]:
    try:
        completed = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, timeout=TIMEOUT
        )
    except subprocess.TimeoutExpired:
        return 124, "timeout"
    return completed.returncode, completed.stdout + completed.stderr


def observable(text: str, seeds: dict[str, str]) -> tuple[tuple[str, ...], bool]:
    """Reduce output to what both backends are required to agree on.

    Returns (sorted state mutations, whether a failure was reported).

    Two things are deliberately excluded. Trace prose, because the backends
    format traces differently by design. And unchanged input fields, because the
    VM echoes the whole final state while the native binary prints only the
    fields it wrote — comparing raw state would report a divergence on every
    seeded field where the two backends actually agree. What both must agree on
    is what the program *changed*, and whether it succeeded.
    """
    # Last value wins per key. A native binary with an unrolled REPEAT_ACTION
    # prints every intermediate value (8, 9, 10) while the VM prints only the
    # final one; they agree on the result, so only the final value is compared.
    final: dict[str, str] = {}
    failed = False
    for line in text.splitlines():
        line = line.strip()
        match = STATE_LINE.match(line)
        if match:
            final[match.group(1)] = match.group(2)
        if FAILURE_LINE.match(line) or line.startswith("failure "):
            failed = True
    mutations = [f"{k}={v}" for k, v in final.items() if seeds.get(k) != v]
    return tuple(sorted(mutations)), failed


FIELD_NODE = re.compile(r"^node Field ([A-Za-z0-9_.]+)\s*:\s*(.+?)(?:\s*\[|$)")


def seed_state(ail: Path, package: Path) -> list[str]:
    """Synthesize a runtime state so actions reach their computation.

    With an empty state most actions fail their existence requirements and never
    execute anything, so the two backends would agree only about failing. Seeding
    declared fields is what makes the comparison exercise real work.
    """
    code, output = run([str(ail), "ail-core", str(package)])
    if code != 0:
        return []
    seeds = []
    for line in output.splitlines():
        match = FIELD_NODE.match(line.strip())
        if not match:
            continue
        qualified, type_name = match.group(1), match.group(2).strip()
        if "." not in qualified:
            continue
        thing, field = qualified.rsplit(".", 1)
        key = f"{thing.lower()}.{field}"
        if type_name.startswith("Secret") or type_name.startswith("List"):
            continue
        if type_name == "Int":
            value = "7"
        elif type_name.startswith("State<"):
            variants = type_name[len("State<") : -1].split(",")
            value = variants[0].strip() if variants else "Open"
        elif type_name == "Bool":
            value = "true"
        else:
            value = "seed"
        seeds.append(f"{key}={value}")
    return seeds


def actions_for(ail: Path, package: Path) -> list[str]:
    """List the actions a package lowers to bytecode."""
    code, output = run([str(ail), "ail-lower", str(package)])
    if code != 0:
        return []
    try:
        program = json.loads(output)
    except json.JSONDecodeError:
        return []
    return [a["action"] for a in program.get("actions", []) if a.get("action")]


def compare(
    ail: Path, package: Path, action: str, workspace: Path, seeds: list[str]
) -> dict[str, object]:
    result: dict[str, object] = {
        "package": package.name,
        "action": action,
        "status": "agree",
        "detail": "",
    }
    binary = workspace / f"{package.name}-{action}"
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
            str(binary),
        ]
    )
    if code != 0 or not binary.is_file():
        result["status"] = "skipped"
        result["detail"] = "not compilable to linux-x86_64-elf"
        return result

    _, vm_output = run([str(ail), "ail-run", str(package), "--action", action, *seeds])
    _, native_output = run([str(binary), *seeds])

    seed_map = dict(s.split("=", 1) for s in seeds if "=" in s)
    vm_state, vm_failed = observable(vm_output, seed_map)
    native_state, native_failed = observable(native_output, seed_map)

    if vm_state != native_state:
        result["status"] = "DIVERGED"
        result["detail"] = f"mutations vm={list(vm_state)} native={list(native_state)}"
    elif vm_failed != native_failed:
        result["status"] = "DIVERGED"
        result["detail"] = f"outcome vm_failed={vm_failed} native_failed={native_failed}"
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="VM vs native differential test.")
    parser.add_argument("--package", help="restrict to one package directory")
    parser.add_argument("--verbose", action="store_true", help="list agreeing pairs too")
    args = parser.parse_args()

    ail = ail_binary()
    if args.package:
        packages = [Path(args.package)]
    else:
        packages = sorted(p for p in (ROOT / "examples").iterdir() if p.suffix == ".ail")

    workspace = Path(tempfile.mkdtemp(prefix="ail-differential-"))
    agreed = 0
    skipped = 0
    diverged = []
    try:
        for package in packages:
            seeds = seed_state(ail, package)
            for action in actions_for(ail, package):
                result = compare(ail, package, action, workspace, seeds)
                if result["status"] == "DIVERGED":
                    diverged.append(result)
                    print(f"DIVERGED  {package.name}::{action}  {result['detail']}")
                elif result["status"] == "skipped":
                    skipped += 1
                    if args.verbose:
                        print(f"  skipped  {package.name}::{action}  {result['detail']}")
                else:
                    agreed += 1
                    if args.verbose:
                        print(f"    agree  {package.name}::{action}")
    finally:
        shutil.rmtree(workspace, ignore_errors=True)

    print()
    print(f"vm/native differential: {agreed} agree, {skipped} not natively compilable, {len(diverged)} diverged")
    return 1 if diverged else 0


if __name__ == "__main__":
    raise SystemExit(main())
