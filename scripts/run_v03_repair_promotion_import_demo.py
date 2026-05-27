#!/usr/bin/env python3
"""Run a deterministic repair-promotion import demo against a corpus copy."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def read_plan(capture_plan_dir: Path) -> tuple[dict[str, object], str]:
    plan_path = capture_plan_dir / "repair-promotion-capture-plan.json"
    fingerprint_path = capture_plan_dir / "repair-promotion-capture-plan.fingerprint.txt"
    plan_text = plan_path.read_text()
    expected = fingerprint_path.read_text().strip()
    actual = fnv64(plan_text)
    if expected != actual:
        raise SystemExit(
            f"capture plan fingerprint mismatch: expected {expected} got {actual}"
        )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("capture plan must be an object")
    return plan, actual


def plan_string(plan: dict[str, object], field: str) -> str:
    value = plan.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"capture plan is missing {field}")
    return value


def run_command(command: list[str]) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def section_for_entry(examples_text: str, entry_id: str) -> str:
    marker = f"## Example: {entry_id}"
    if marker not in examples_text:
        raise SystemExit(f"examples output is missing {entry_id}")
    section = examples_text.split(marker, 1)[1]
    return section.split("## Example:", 1)[0]


def line_present(text: str, line: str) -> bool:
    return any(candidate.strip() == line for candidate in text.splitlines())


def require_report_line(report_text: str, line: str) -> None:
    if line not in report_text:
        raise SystemExit(f"replay report missing {line}")


def build_batch_plan(
    args: argparse.Namespace,
    plan: dict[str, object],
    request_path: Path,
    response_path: Path,
    batch_plan_path: Path,
) -> None:
    source_entry_id = plan_string(plan, "source_entry_id")
    proposed_entry_id = plan_string(plan, "proposed_entry_id")
    write_json(
        batch_plan_path,
        {
            "entries": [
                {
                    "entry_id": proposed_entry_id,
                    "source_entry_id": source_entry_id,
                    "executor_family": "codex-skill-agent",
                    "executor_label": args.executor_label,
                    "semantic_task": args.semantic_task,
                    "request_json_file": str(request_path),
                    "response_json_file": str(response_path),
                    "checker_result": "accepted",
                    "repair_promotion_capture_plan_json": str(
                        args.capture_plan_dir / "repair-promotion-capture-plan.json"
                    ),
                }
            ]
        },
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Append a human-approved repair promotion entry to a corpus copy and replay it."
    )
    parser.add_argument("--base-corpus", default="examples")
    parser.add_argument(
        "--examples-artifacts", default="/tmp/ail-manual-repair-promotion"
    )
    parser.add_argument(
        "--capture-plan-dir", default="/tmp/ail-manual-repair-promotion-capture-plan"
    )
    parser.add_argument("--source-entry-id", default="example-99")
    parser.add_argument("--work-dir", default="/tmp/ail-manual-repair-promotion-import-work")
    parser.add_argument("--output-corpus", default="/tmp/ail-manual-repair-promotion-import-corpus")
    parser.add_argument(
        "--output-artifacts", default="/tmp/ail-manual-repair-promotion-import-artifacts"
    )
    parser.add_argument(
        "--executor-label", default="codex-ail-repair-promotion-reviewer-demo"
    )
    parser.add_argument("--semantic-task", default="support-ticket-repair-promoted-99")
    parsed = parser.parse_args(argv)
    parsed.examples_artifacts = Path(parsed.examples_artifacts)
    parsed.capture_plan_dir = Path(parsed.capture_plan_dir)
    parsed.work_dir = Path(parsed.work_dir)
    parsed.output_corpus = Path(parsed.output_corpus)
    parsed.output_artifacts = Path(parsed.output_artifacts)
    return parsed


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    plan, plan_fingerprint = read_plan(args.capture_plan_dir)
    source_entry_id = plan_string(plan, "source_entry_id")
    proposed_entry_id = plan_string(plan, "proposed_entry_id")
    if source_entry_id != args.source_entry_id:
        raise SystemExit(
            f"capture plan source_entry_id {source_entry_id} does not match {args.source_entry_id}"
        )

    args.work_dir.mkdir(parents=True, exist_ok=True)
    if args.output_artifacts.exists():
        shutil.rmtree(args.output_artifacts)
    request_path = args.work_dir / "approved-request.json"
    response_path = args.work_dir / "approved-response.json"
    batch_plan_path = args.work_dir / "human-approved-repair-promotion-batch.json"
    repair_candidate_path = (
        args.examples_artifacts
        / "examples"
        / source_entry_id
        / "repair-candidate.ail-spec.md"
    )
    repair_candidate = repair_candidate_path.read_text()
    write_json(
        request_path,
        {
            "approval_mode": "deterministic-demo",
            "executor_label": args.executor_label,
            "repair_promotion_capture_plan_fingerprint": plan_fingerprint,
            "source_entry_id": source_entry_id,
            "task": "Approve the repaired AIL-Spec candidate for corpus promotion.",
        },
    )
    write_json(
        response_path,
        {
            "artifact_text": repair_candidate,
            "model": "human-approved-repair-promotion-demo",
        },
    )
    build_batch_plan(args, plan, request_path, response_path, batch_plan_path)

    run_command(
        [
            sys.executable,
            "scripts/capture_example_batch.py",
            "--base-corpus",
            args.base_corpus,
            "--output-dir",
            str(args.output_corpus),
            "--plan-json",
            str(batch_plan_path),
        ]
    )
    run_command(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "ail-examples",
            str(args.output_corpus),
            "--artifact-dir",
            str(args.output_artifacts),
            "--release-evidence",
        ]
    )

    examples_text = (args.output_corpus / "examples.md").read_text()
    source_section = section_for_entry(examples_text, source_entry_id)
    proposed_section = section_for_entry(examples_text, proposed_entry_id)
    source_preserved = line_present(source_section, "checker-result: rejected")
    proposed_accepted = line_present(proposed_section, "checker-result: accepted")
    report_text = (args.output_artifacts / "examples-report.txt").read_text()
    for line in [
        "entry-count 118",
        "checker-result-count accepted 109",
        "checker-result-count rejected 9",
        f"entry {source_entry_id} ",
        f"entry {proposed_entry_id} ",
    ]:
        require_report_line(report_text, line)
    output_lines = [
        "AIL-Repair-Promotion-Import-Demo:",
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        f"source-preserved {str(source_preserved).lower()}",
        f"proposed-accepted {str(proposed_accepted).lower()}",
        "entry-count 118",
        "checker-result-count accepted 109",
        "checker-result-count rejected 9",
        f"batch-plan {batch_plan_path}",
        f"output-corpus {args.output_corpus}",
        f"output-artifacts {args.output_artifacts}",
        "",
    ]
    output_text = "\n".join(output_lines)
    if not source_preserved or not proposed_accepted:
        raise SystemExit(output_text)
    report_path = args.work_dir / "repair-promotion-import-demo-report.txt"
    report_path.write_text(output_text)
    (args.work_dir / "repair-promotion-import-demo-report.fingerprint.txt").write_text(
        fnv64(output_text) + "\n"
    )
    sys.stdout.write(output_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
