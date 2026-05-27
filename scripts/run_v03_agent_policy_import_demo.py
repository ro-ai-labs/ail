#!/usr/bin/env python3
"""Run a deterministic AgentTool policy handoff import demo against a corpus copy."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
POLICY_HANDOFF_TRACE = "PolicyHandoffApprovedScenario40"
EXPECTED_HANDOFF_ROLES = (
    "requirements-writer,spec-writer,diagnostic-repairer,"
    "prompt-reviewer,agent-policy-reviewer"
)


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
    plan_path = capture_plan_dir / "agent-policy-capture-plan.json"
    fingerprint_path = capture_plan_dir / "agent-policy-capture-plan.fingerprint.txt"
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


def require_plan_value(plan: dict[str, object], field: str, expected: object) -> None:
    actual = plan.get(field)
    if actual != expected:
        raise SystemExit(
            f"capture plan {field} expected {expected}, got {actual or '<missing>'}"
        )


def run_command(command: list[str]) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def run_text_command(command: list[str]) -> str:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    return completed.stdout.rstrip("\n")


def extract_artifact_text(response_path: Path) -> str:
    response = json.loads(response_path.read_text())
    if not isinstance(response, dict):
        raise SystemExit(f"response artifact must be an object: {response_path}")
    for field in ["content", "artifact_text"]:
        value = response.get(field)
        if isinstance(value, str) and value.strip():
            return value.strip()
    raise SystemExit(f"response artifact is missing content/artifact_text: {response_path}")


def add_policy_handoff_trace(source_spec_text: str) -> str:
    trace_line = f"- {POLICY_HANDOFF_TRACE}"
    if trace_line in source_spec_text:
        return source_spec_text
    marker = "The tool records:\n\n- RefundCustomerPaymentRequested"
    replacement = f"{marker}\n{trace_line}"
    if marker not in source_spec_text:
        raise SystemExit("source AgentTool spec is missing the tool records block")
    return source_spec_text.replace(marker, replacement, 1)


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


def validate_plan(plan: dict[str, object], source_entry_id: str) -> None:
    require_plan_value(plan, "artifact_kind", "AIL-Agent-Policy-Capture-Plan")
    require_plan_value(plan, "source_entry_id", source_entry_id)
    require_plan_value(plan, "status", "plan-only")
    require_plan_value(plan, "policy_import_decision", "accepted-for-import")
    require_plan_value(plan, "policy_import_status", "proposed-only")
    require_plan_value(plan, "agent_contract_check", "ail-agent-contracts examples/agents")
    require_plan_value(plan, "profile", "AgentTool")
    require_plan_value(plan, "program_domain", "agent-tool")
    require_plan_value(plan, "handoff_roles", EXPECTED_HANDOFF_ROLES)
    require_plan_value(plan, "human_approval_required", True)
    require_plan_value(plan, "must_supply_request_response_json", True)
    require_plan_value(plan, "preserve_source_entry", True)


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
                    "agent_policy_capture_plan_json": str(
                        args.capture_plan_dir / "agent-policy-capture-plan.json"
                    ),
                }
            ]
        },
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Append a human-approved AgentTool policy handoff entry to a corpus "
            "copy and replay it."
        )
    )
    parser.add_argument("--base-corpus", default="examples")
    parser.add_argument("--examples-artifacts", default="/tmp/ail-v03-agent-policy")
    parser.add_argument(
        "--capture-plan-dir", default="/tmp/ail-v03-agent-policy-capture-plan"
    )
    parser.add_argument("--source-entry-id", default="example-40")
    parser.add_argument("--work-dir", default="/tmp/ail-v03-agent-policy-import-work")
    parser.add_argument("--output-corpus", default="/tmp/ail-v03-agent-policy-import-corpus")
    parser.add_argument(
        "--output-artifacts", default="/tmp/ail-v03-agent-policy-import-artifacts"
    )
    parser.add_argument("--executor-label", default="codex-ail-agent-policy-reviewer-demo")
    parser.add_argument("--semantic-task", default="refund-tool-policy-handoff-import-40")
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
    validate_plan(plan, args.source_entry_id)
    source_entry_id = plan_string(plan, "source_entry_id")
    proposed_entry_id = plan_string(plan, "proposed_entry_id")

    args.work_dir.mkdir(parents=True, exist_ok=True)
    if args.output_artifacts.exists():
        shutil.rmtree(args.output_artifacts)
    request_path = args.work_dir / "approved-agent-policy-request.json"
    response_path = args.work_dir / "approved-agent-policy-response.json"
    patched_spec_path = args.work_dir / "approved-agent-policy.ail-spec.md"
    patched_core_path = args.work_dir / "approved-agent-policy.checked.ail-core.txt"
    batch_plan_path = args.work_dir / "human-approved-agent-policy-batch.json"

    entry_artifact_dir = args.examples_artifacts / "examples" / source_entry_id
    source_core_path = entry_artifact_dir / "checked.ail-core.txt"
    review_path = entry_artifact_dir / "agent-policy-review.txt"
    source_response_path = ROOT / args.base_corpus / "responses" / f"{source_entry_id}.json"
    source_core_text = source_core_path.read_text()
    review_text = review_path.read_text()
    source_spec_text = extract_artifact_text(source_response_path)
    source_core_fingerprint_preserved = (
        fnv64(source_core_text) == plan_string(plan, "checked_core_fingerprint")
    )
    review_fingerprint_preserved = (
        fnv64(review_text) == plan_string(plan, "agent_policy_review_fingerprint")
    )
    if not source_core_fingerprint_preserved or not review_fingerprint_preserved:
        raise SystemExit("source AgentTool policy evidence does not match plan fingerprints")

    patched_spec_text = add_policy_handoff_trace(source_spec_text)
    patched_spec_path.write_text(patched_spec_text)
    patched_core_text = run_text_command(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "ail-core",
            "examples/refund_tool.ail",
            "--spec-file",
            str(patched_spec_path),
        ]
    )
    patched_core_path.write_text(patched_core_text)
    policy_handoff_imported = (
        f"node Trace {POLICY_HANDOFF_TRACE}" in patched_core_text
        and patched_spec_text != source_spec_text
    )
    if not policy_handoff_imported:
        raise SystemExit(f"policy handoff trace {POLICY_HANDOFF_TRACE} was not imported")

    write_json(
        request_path,
        {
            "approval_mode": "deterministic-demo",
            "agent_policy_capture_plan_fingerprint": plan_fingerprint,
            "agent_policy_review_fingerprint": plan_string(
                plan, "agent_policy_review_fingerprint"
            ),
            "executor_label": args.executor_label,
            "policy_handoff_trace": POLICY_HANDOFF_TRACE,
            "promoted_spec_fingerprint": fnv64(patched_spec_text),
            "source_entry_id": source_entry_id,
            "source_spec_fingerprint": fnv64(source_spec_text),
            "task": "Approve the AgentTool policy handoff review and replay the amended spec.",
        },
    )
    write_json(
        response_path,
        {
            "artifact_text": patched_spec_text,
            "model": "human-approved-agent-policy-import-demo",
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
    source_preserved = line_present(source_section, "checker-result: accepted")
    proposed_accepted = line_present(proposed_section, "checker-result: accepted")
    replayed_core_path = (
        args.output_artifacts / "examples" / proposed_entry_id / "checked.ail-core.txt"
    )
    policy_handoff_replayed = (
        replayed_core_path.exists()
        and f"node Trace {POLICY_HANDOFF_TRACE}" in replayed_core_path.read_text()
    )
    report_text = (args.output_artifacts / "examples-report.txt").read_text()
    for line in [
        "entry-count 126",
        "checker-result-count accepted 117",
        "checker-result-count rejected 9",
        f"entry {source_entry_id} ",
        f"entry {proposed_entry_id} ",
    ]:
        require_report_line(report_text, line)
    output_lines = [
        "AIL-Agent-Policy-Import-Demo:",
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        f"source-preserved {str(source_preserved).lower()}",
        f"proposed-accepted {str(proposed_accepted).lower()}",
        "agent-policy-review-fingerprint-preserved "
        f"{str(review_fingerprint_preserved).lower()}",
        "checked-core-fingerprint-preserved "
        f"{str(source_core_fingerprint_preserved).lower()}",
        f"policy-handoff-imported {str(policy_handoff_imported).lower()}",
        f"policy-handoff-replayed {str(policy_handoff_replayed).lower()}",
        "entry-count 126",
        "checker-result-count accepted 117",
        "checker-result-count rejected 9",
        f"patched-spec {patched_spec_path}",
        f"batch-plan {batch_plan_path}",
        f"output-corpus {args.output_corpus}",
        f"output-artifacts {args.output_artifacts}",
        "",
    ]
    output_text = "\n".join(output_lines)
    if (
        not source_preserved
        or not proposed_accepted
        or not policy_handoff_imported
        or not policy_handoff_replayed
        or not source_core_fingerprint_preserved
        or not review_fingerprint_preserved
    ):
        raise SystemExit(output_text)
    report_path = args.work_dir / "agent-policy-import-demo-report.txt"
    report_path.write_text(output_text)
    (args.work_dir / "agent-policy-import-demo-report.fingerprint.txt").write_text(
        fnv64(output_text) + "\n"
    )
    sys.stdout.write(output_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
