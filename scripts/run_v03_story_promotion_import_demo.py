#!/usr/bin/env python3
"""Run a deterministic story-promotion import demo against a corpus copy."""

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
    plan_path = capture_plan_dir / "story-promotion-capture-plan.json"
    fingerprint_path = capture_plan_dir / "story-promotion-capture-plan.fingerprint.txt"
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


def plan_int(plan: dict[str, object], field: str) -> int:
    value = plan.get(field)
    if not isinstance(value, int):
        raise SystemExit(f"capture plan has invalid {field}")
    return value


def plan_bool(plan: dict[str, object], field: str) -> bool:
    value = plan.get(field)
    if not isinstance(value, bool):
        raise SystemExit(f"capture plan has invalid {field}")
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
    request_path: Path,
    response_path: Path,
    batch_plan_path: Path,
) -> str:
    write_json(
        batch_plan_path,
        {
            "entries": [
                {
                    "entry_id": args.proposed_entry_id,
                    "source_entry_id": args.source_entry_id,
                    "executor_family": "codex-skill-agent",
                    "executor_label": args.executor_label,
                    "semantic_task": args.semantic_task,
                    "request_json_file": str(request_path),
                    "response_json_file": str(response_path),
                    "checker_result": "accepted",
                    "story_promotion_capture_plan_json": str(
                        args.capture_plan_dir / "story-promotion-capture-plan.json"
                    ),
                }
            ]
        },
    )
    batch_plan_fingerprint = fnv64(batch_plan_path.read_text())
    batch_plan_path.with_name(
        "human-approved-story-promotion-batch.fingerprint.txt"
    ).write_text(batch_plan_fingerprint + "\n")
    return batch_plan_fingerprint


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Append a human-approved story promotion entry to a corpus copy and replay it."
    )
    parser.add_argument("--base-corpus", default="examples")
    parser.add_argument("--story-artifacts", default="/tmp/ail-v03-story-llm")
    parser.add_argument(
        "--capture-plan-dir", default="/tmp/ail-v03-story-promotion-capture-plan"
    )
    parser.add_argument("--source-entry-id", default="example-30")
    parser.add_argument("--proposed-entry-id", default="example-30-story-demo")
    parser.add_argument("--work-dir", default="/tmp/ail-v03-story-promotion-import-work")
    parser.add_argument("--output-corpus", default="/tmp/ail-v03-story-promotion-import-corpus")
    parser.add_argument(
        "--output-artifacts", default="/tmp/ail-v03-story-promotion-import-artifacts"
    )
    parser.add_argument("--executor-label", default="codex-ail-story-promotion-reviewer-demo")
    parser.add_argument("--semantic-task", default="support-ticket-story-promoted-30")
    parsed = parser.parse_args(argv)
    parsed.story_artifacts = Path(parsed.story_artifacts)
    parsed.capture_plan_dir = Path(parsed.capture_plan_dir)
    parsed.work_dir = Path(parsed.work_dir)
    parsed.output_corpus = Path(parsed.output_corpus)
    parsed.output_artifacts = Path(parsed.output_artifacts)
    return parsed


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    plan, plan_fingerprint = read_plan(args.capture_plan_dir)
    story_id = plan_string(plan, "story_id")
    story_artifact_dir = Path(plan_string(plan, "story_artifact_dir"))
    default_max_tokens = plan_int(plan, "default_max_tokens")
    max_tokens = plan_int(plan, "max_tokens")
    token_budget_default = plan_bool(plan, "token_budget_default")
    promotion_decision = plan_string(plan, "promotion_decision")
    if promotion_decision != "accepted-for-promotion":
        raise SystemExit(
            f"capture plan promotion_decision must be accepted-for-promotion, got {promotion_decision}"
        )
    human_approval_required = plan_bool(plan, "human_approval_required")
    if not human_approval_required:
        raise SystemExit("capture plan must require human approval")
    token_budget_warning = plan.get("token_budget_warning")
    if not isinstance(token_budget_warning, str):
        raise SystemExit("capture plan has invalid token_budget_warning")
    if story_artifact_dir.resolve() != args.story_artifacts.resolve():
        raise SystemExit(
            f"capture plan story_artifact_dir {story_artifact_dir} does not match {args.story_artifacts}"
        )

    args.work_dir.mkdir(parents=True, exist_ok=True)
    if args.output_artifacts.exists():
        shutil.rmtree(args.output_artifacts)
    request_path = args.work_dir / "approved-story-request.json"
    response_path = args.work_dir / "approved-story-response.json"
    batch_plan_path = args.work_dir / "human-approved-story-promotion-batch.json"
    accepted_spec_path = args.story_artifacts / "accepted.ail-spec.md"
    accepted_spec = accepted_spec_path.read_text()
    write_json(
        request_path,
        {
            "agent_contract": "examples/agents/codex-ail-story-promotion-reviewer.md",
            "agent_contract_version": "0.1.0",
            "approval_mode": "deterministic-demo",
            "executor_label": args.executor_label,
            "source_entry_id": args.source_entry_id,
            "story_id": story_id,
            "story_promotion_capture_plan_fingerprint": plan_fingerprint,
            "task": "Approve the reviewed User Story mode artifact for corpus promotion.",
        },
    )
    write_json(
        response_path,
        {
            "artifact_text": accepted_spec,
            "model": "human-approved-story-promotion-demo",
        },
    )
    batch_plan_fingerprint = build_batch_plan(
        args, request_path, response_path, batch_plan_path
    )

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
    source_section = section_for_entry(examples_text, args.source_entry_id)
    proposed_section = section_for_entry(examples_text, args.proposed_entry_id)
    source_preserved = line_present(source_section, "checker-result: accepted")
    proposed_accepted = line_present(proposed_section, "checker-result: accepted")
    story_artifacts_copy = args.output_corpus / "story-artifacts" / args.proposed_entry_id
    story_artifacts_preserved = (
        story_artifacts_copy.joinpath("story-mode-report.txt").exists()
        and story_artifacts_copy.joinpath("manifest.ail-story.txt").exists()
        and story_artifacts_copy.joinpath("story-llm-harness-report.txt").exists()
    )
    report_text = (args.output_artifacts / "examples-report.txt").read_text()
    for line in [
        "entry-count 126",
        "checker-result-count accepted 117",
        "checker-result-count rejected 9",
        f"entry {args.source_entry_id} ",
        f"entry {args.proposed_entry_id} ",
    ]:
        require_report_line(report_text, line)
    output_lines = [
        "AIL-Story-Promotion-Import-Demo:",
        f"story-id {story_id}",
        f"source-entry-id {args.source_entry_id}",
        f"proposed-entry-id {args.proposed_entry_id}",
        f"source-preserved {str(source_preserved).lower()}",
        f"proposed-accepted {str(proposed_accepted).lower()}",
        f"story-artifacts-preserved {str(story_artifacts_preserved).lower()}",
        f"story-artifacts-source {args.story_artifacts}",
        f"capture-plan story-promotion-capture-plan.json {plan_fingerprint}",
        f"promotion-decision {promotion_decision}",
        f"human-approval-required {str(human_approval_required).lower()}",
        "promotion-source human-approved-story-promotion-batch",
        f"batch-plan-fingerprint {batch_plan_fingerprint}",
        f"default-max-tokens {default_max_tokens}",
        f"max-tokens {max_tokens}",
        f"token-budget-default {str(token_budget_default).lower()}",
        f"token-budget-warning {token_budget_warning}",
        "entry-count 126",
        "checker-result-count accepted 117",
        "checker-result-count rejected 9",
        f"batch-plan {batch_plan_path}",
        f"output-corpus {args.output_corpus}",
        f"output-artifacts {args.output_artifacts}",
        "",
    ]
    output_text = "\n".join(output_lines)
    if not source_preserved or not proposed_accepted or not story_artifacts_preserved:
        raise SystemExit(output_text)
    report_path = args.work_dir / "story-promotion-import-demo-report.txt"
    report_path.write_text(output_text)
    (args.work_dir / "story-promotion-import-demo-report.fingerprint.txt").write_text(
        fnv64(output_text) + "\n"
    )
    sys.stdout.write(output_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
