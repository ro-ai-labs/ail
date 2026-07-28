#!/usr/bin/env python3
"""Round-trip benchmark: can specs be reconstructed from the IR alone?

This measures the property that defines AIL. Today's LLM pipeline is
spec -> TypeScript/Python -> binary, so the de facto IR is a general-purpose
programming language, which a domain expert cannot read or correct. AIL's claim
is an IR layer that a non-programmer can review, and that claim is only true if
the IR carries the whole meaning in a form that can be turned back into specs.

Two modes, measuring two different things:

  deterministic  core -> ail-spec -> reparse -> compare against the original core
                 Does the IR *contain* the meaning? Pure toolchain, no model.
                 A failure here is a losslessness bug in the IR or its renderer.

  agent          core -> LLM -> spec -> reparse -> compare against the original
                 Is the IR *understandable*? A failure here means the IR reads
                 as something a reader cannot faithfully reconstruct from, which
                 is the democratization claim failing even when nothing is lost.

Both compare with `ail-equivalence`, which compares meaning rather than text:
English phrasing is expected to vary across a round trip, and meaning is not.

Usage:
    python3 scripts/ail_roundtrip.py
    python3 scripts/ail_roundtrip.py --package examples/support_ticket.ail --verbose
    python3 scripts/ail_roundtrip.py --mode agent --endpoint http://192.168.60.171:8091/v1/chat/completions
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TIMEOUT = 120
AGENT_TIMEOUT = 600
CORE_TO_SPEC_PROMPT = ROOT / "docs/ail/prompts/core-to-spec.system.md"


def ail_binary() -> Path:
    for candidate in (ROOT / "target/debug/ail", ROOT / "target/release/ail"):
        if candidate.is_file():
            return candidate
    raise SystemExit("no ail binary found; run 'cargo build' first")


def run(command: list[str], timeout: int = TIMEOUT) -> tuple[int, str]:
    try:
        done = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return 124, f"timed out after {timeout}s"
    return done.returncode, done.stdout + done.stderr


def package_manifest(package: Path) -> str:
    manifest = package / "ail-package.md"
    return manifest.read_text(encoding="utf-8") if manifest.is_file() else ""


def reconstruct_deterministically(ail: Path, core_path: Path) -> tuple[str, str]:
    """Render a checked Core back to AIL-Spec using the toolchain itself."""
    code, output = run([str(ail), "ail-spec", "--core-file", str(core_path)])
    if code != 0:
        return "", output
    return output, ""


def reconstruct_with_agent(core_text: str, endpoint: str, model: str) -> tuple[str, str]:
    """Ask a model to reconstruct specs from the IR and nothing else."""
    if not CORE_TO_SPEC_PROMPT.is_file():
        return "", f"missing prompt {CORE_TO_SPEC_PROMPT}"
    system = CORE_TO_SPEC_PROMPT.read_text(encoding="utf-8")
    # The IR is the only input. That restriction is the whole point: if the
    # model needs the original spec to reproduce the spec, the IR has not
    # carried the meaning.
    body = json.dumps(
        {
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {
                    "role": "user",
                    "content": (
                        "Reconstruct the AIL-Spec for this checked AIL-Core. "
                        "Return only the AIL-Spec document.\n\n" + core_text
                    ),
                },
            ],
            "temperature": 0.0,
            "stream": False,
            "chat_template_kwargs": {"enable_thinking": False},
        }
    ).encode("utf-8")
    request = urllib.request.Request(
        endpoint, data=body, headers={"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(request, timeout=AGENT_TIMEOUT) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except Exception as error:  # noqa: BLE001 - report any transport failure verbatim
        return "", f"model call failed: {error}"
    try:
        content = payload["choices"][0]["message"]["content"]
    except (KeyError, IndexError) as error:
        return "", f"unexpected response shape: {error}"
    if not content.strip():
        return "", "model returned empty content"
    return content, ""


def evaluate(package: Path, ail: Path, args) -> dict[str, str]:
    result = {"package": package.name, "status": "fail", "detail": ""}
    workspace = Path(tempfile.mkdtemp(prefix=f"ail-roundtrip-{package.name}-"))
    try:
        code, core_text = run([str(ail), "ail-core", str(package)])
        if code != 0:
            result["status"] = "skipped"
            result["detail"] = "package does not check clean"
            return result
        core_path = workspace / "original.ail-core.txt"
        core_path.write_text(core_text, encoding="utf-8")

        if args.mode == "deterministic":
            spec_text, error = reconstruct_deterministically(ail, core_path)
        else:
            spec_text, error = reconstruct_with_agent(core_text, args.endpoint, args.model)
        if error:
            result["detail"] = error
            return result

        rebuilt = workspace / "rebuilt.ail"
        rebuilt.mkdir()
        (rebuilt / "ail-package.md").write_text(package_manifest(package), encoding="utf-8")
        (rebuilt / "spec.ail-spec.md").write_text(spec_text, encoding="utf-8")

        code, check_output = run([str(ail), "ail-check", str(rebuilt)])
        if code != 0 or "ail diagnostics: none" not in check_output:
            result["status"] = "check-failed"
            result["detail"] = next(
                (line for line in check_output.splitlines() if line.startswith("AIL")),
                "reconstructed spec does not check clean",
            )
            if args.verbose:
                print(check_output, file=sys.stderr)
            return result

        code, diff = run(
            [str(ail), "ail-equivalence", str(rebuilt), "--baseline", str(core_path)]
        )
        if code == 0:
            result["status"] = "pass"
            result["detail"] = "semantically equivalent"
        else:
            counts = [
                line
                for line in diff.splitlines()
                if line.startswith(("removed-count", "added-count", "retyped-count"))
            ]
            result["detail"] = " ".join(counts)
            if args.verbose:
                print(diff, file=sys.stderr)
        return result
    finally:
        shutil.rmtree(workspace, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="AIL IR round-trip benchmark.")
    parser.add_argument("--mode", choices=["deterministic", "agent"], default="deterministic")
    parser.add_argument("--package", help="restrict to one package directory")
    parser.add_argument(
        "--endpoint",
        default="http://192.168.60.171:8091/v1/chat/completions",
        help="OpenAI-compatible endpoint, agent mode only",
    )
    parser.add_argument("--model", default="ornith-1.0-35b", help="model id, agent mode only")
    parser.add_argument("--verbose", action="store_true", help="print diffs and checker output")
    args = parser.parse_args()

    ail = ail_binary()
    if args.package:
        packages = [Path(args.package)]
    else:
        packages = sorted(p for p in (ROOT / "examples").iterdir() if p.suffix == ".ail")

    passed = skipped = 0
    failures = []
    for package in packages:
        result = evaluate(package, ail, args)
        marker = {"pass": "PASS", "skipped": "----", "check-failed": "CHECK"}.get(
            result["status"], "FAIL"
        )
        if result["status"] == "pass":
            passed += 1
        elif result["status"] == "skipped":
            skipped += 1
        else:
            failures.append(result)
        print(f"{marker:>5}  {result['package']:<28} {result['detail']}")

    scored = len(packages) - skipped
    print()
    print(f"round-trip ({args.mode}) {passed}/{scored} semantically equivalent, {skipped} skipped")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
