#!/usr/bin/env python3
"""Driver for the AIL capability loop.

Gives each unattended iteration a crisp contract, so an agent never has to guess
what to work on or whether it is allowed to commit.

    python3 scripts/ail_loop.py --next     what to work on now
    python3 scripts/ail_loop.py --verify   may this change be committed?

--next reports the current score and the lowest-numbered failing rung, together
with the language capability that rung is blocked on. That capability is the
task. Writing a document about it is not.

--verify runs every gate and prints ALLOW or BLOCK. It is the commit decision,
and it is deterministic: the loop does not get to talk itself into a commit.

Exit codes:
    0  --next found work / --verify says ALLOW
    1  --verify says BLOCK
    2  a precondition failed (dirty tree, missing tooling)
    3  the ladder is complete
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / "bench"
AUDIT_TIMEOUT = 900


def run(command: list[str], timeout: int = 300) -> tuple[int, str]:
    try:
        done = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return 124, f"timed out after {timeout}s"
    except FileNotFoundError as error:
        return 127, str(error)
    return done.returncode, done.stdout + done.stderr


def metadata(rung: Path) -> dict[str, str]:
    intent = rung / "intent.md"
    if not intent.is_file():
        return {}
    fields = {}
    for line in intent.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line or " " in line.split(":", 1)[0].strip():
            break
        key, value = line.split(":", 1)
        fields[key.strip()] = value.strip()
    return fields


def bench_state() -> tuple[int, int, list[tuple[str, str]]]:
    """Return (score, total, [(failing rung, detail)])."""
    code, output = run([sys.executable, "scripts/ail_bench.py"])
    score, total = 0, 0
    failing = []
    for line in output.splitlines():
        match = re.match(r"^ail-bench score (\d+)/(\d+)$", line.strip())
        if match:
            score, total = int(match.group(1)), int(match.group(2))
            continue
        parts = line.split(None, 2)
        if len(parts) >= 2 and parts[0] in {"----", "FAIL", "CHECK", "COMPILE"}:
            failing.append((parts[1], parts[2] if len(parts) > 2 else ""))
    if code == 3 or code == 4:
        print(output, file=sys.stderr)
    return score, total, failing


def show_next() -> int:
    code, dirty = run(["git", "status", "--porcelain"])
    if dirty.strip():
        print("PRECONDITION FAILED: working tree is dirty.")
        print("Commit or discard the previous iteration before starting a new one.")
        print(dirty.strip())
        return 2

    score, total, failing = bench_state()
    print(f"score        {score}/{total}")

    if not failing:
        print("state        ladder complete")
        print("next         extend bench/LADDER.md with the next tier")
        return 3

    rung_name, detail = failing[0]
    rung = BENCH / rung_name
    fields = metadata(rung)
    intent = (rung / "intent.md").read_text(encoding="utf-8") if rung.is_dir() else ""
    body = intent.split("\n#", 1)[-1].strip() if "\n#" in intent else intent.strip()

    print(f"next-rung    {rung_name}")
    print(f"tier         {fields.get('tier', '?')}")
    print(f"action       {fields.get('action', '?')}")
    print(f"blocked-on   {fields.get('blocked-on', 'unknown')}")
    print(f"status       {detail}")
    print(f"cases        {len(list((rung / 'cases').glob('*.args')))}")
    print()
    print("TASK: add the language capability named in blocked-on, then write")
    print(f"      {rung_name}/program.ail so the rung passes on VM and native.")
    print("      Adding a document, a report, or a catalog entry closes nothing.")
    print()
    print(body)
    return 0


def show_verify() -> int:
    checks: list[tuple[str, bool, str]] = []

    code, output = run(["cargo", "fmt", "--check"])
    checks.append(("cargo fmt", code == 0, output.strip().splitlines()[:1]))

    code, output = run(["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], 600)
    checks.append(("clippy -D warnings", code == 0, ""))

    code, output = run(["cargo", "test"], 900)
    summary = next(
        (line for line in output.splitlines() if line.startswith("test result: FAILED")), ""
    )
    checks.append(("cargo test", code == 0, summary))

    code, output = run([sys.executable, "scripts/ail_differential.py"])
    diverged = next((line for line in output.splitlines() if "diverged" in line), "")
    checks.append(("vm/native differential", code == 0, diverged.strip()))

    # ail_bench enforces the ratchet, the frozen-rung fingerprints, and the
    # no-deleted-rung rule. A non-zero exit here means the change is not
    # committable regardless of how good it looks.
    code, output = run([sys.executable, "scripts/ail_bench.py"])
    score_line = next((line for line in output.splitlines() if "ail-bench score" in line), "")
    reason = ""
    if code == 3:
        reason = "ladder tampered"
    elif code == 4:
        reason = "ratchet broken: a previously passing rung now fails"
    checks.append(("ail-bench ratchet", code == 0, reason or score_line.strip()))

    code, output = run(
        [sys.executable, "scripts/run_v03_release_audit.py", "--bundle-root", "/tmp/ail-loop-audit"],
        AUDIT_TIMEOUT,
    )
    checks.append(("release audit", code == 0, output.strip().splitlines()[-1:] if code else ""))

    width = max(len(name) for name, _, _ in checks)
    allowed = True
    for name, ok, detail in checks:
        allowed &= ok
        note = detail if isinstance(detail, str) else " ".join(detail)
        print(f"{'ok  ' if ok else 'FAIL'}  {name:<{width}}  {note}")

    print()
    if allowed:
        print("ALLOW — commit, then record the iteration in bench/JOURNAL.md")
        print("        (if the score rose, run: python3 scripts/ail_bench.py --update-lock)")
        return 0
    print("BLOCK — discard this change. Do not weaken a gate to make it pass.")
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description="AIL capability loop driver.")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--next", action="store_true", help="what to work on now")
    group.add_argument("--verify", action="store_true", help="may this change be committed?")
    args = parser.parse_args()
    return show_next() if args.next else show_verify()


if __name__ == "__main__":
    raise SystemExit(main())
