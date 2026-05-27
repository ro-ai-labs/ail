#!/usr/bin/env python3
"""Write deterministic multi-agent evidence for an AgentTool policy import."""

from __future__ import annotations

import argparse
import json
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


def read_fingerprinted_text(path: Path, fingerprint_path: Path, label: str) -> tuple[str, str]:
    text = path.read_text()
    expected = fingerprint_path.read_text().strip()
    actual = fnv64(text)
    if expected != actual:
        raise SystemExit(f"{label} fingerprint mismatch: expected {expected} got {actual}")
    return text, actual


def read_plan(capture_plan_dir: Path) -> tuple[dict[str, object], str]:
    plan_text, fingerprint = read_fingerprinted_text(
        capture_plan_dir / "agent-policy-capture-plan.json",
        capture_plan_dir / "agent-policy-capture-plan.fingerprint.txt",
        "agent policy capture plan",
    )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("agent policy capture plan must be an object")
    return plan, fingerprint


def plan_string(plan: dict[str, object], field: str) -> str:
    value = plan.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"agent policy capture plan is missing {field}")
    return value


def require_plan_value(plan: dict[str, object], field: str, expected: object) -> None:
    actual = plan.get(field)
    if actual != expected:
        raise SystemExit(
            f"agent policy capture plan {field} expected {expected}, got {actual or '<missing>'}"
        )


def require_line(text: str, line: str, label: str) -> None:
    if line not in text:
        raise SystemExit(f"{label} missing {line}")


def run_agent_contracts() -> str:
    completed = subprocess.run(
        ["cargo", "run", "--quiet", "--", "ail-agent-contracts", "examples/agents"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    return completed.stdout


def validate_plan(plan: dict[str, object], source_entry_id: str) -> None:
    require_plan_value(plan, "artifact_kind", "AIL-Agent-Policy-Capture-Plan")
    require_plan_value(plan, "source_entry_id", source_entry_id)
    require_plan_value(plan, "status", "plan-only")
    require_plan_value(plan, "policy_import_decision", "accepted-for-import")
    require_plan_value(plan, "policy_import_status", "proposed-only")
    require_plan_value(plan, "agent_contract_check", "ail-agent-contracts examples/agents")
    require_plan_value(plan, "handoff_roles", EXPECTED_HANDOFF_ROLES)
    require_plan_value(plan, "human_approval_required", True)
    require_plan_value(plan, "must_supply_request_response_json", True)
    require_plan_value(plan, "preserve_source_entry", True)


def validate_contracts(contracts_text: str) -> None:
    for required in [
        "AIL-Agent-Contracts-Report:",
        "contract-count 7",
        "contract codex-ail-requirements-writer",
        "contract codex-ail-spec-writer",
        "contract codex-ail-diagnostic-repairer",
        "contract codex-ail-prompt-reviewer",
        "contract codex-ail-agent-policy-reviewer",
        "contract codex-ail-ui-patch-reviewer",
        "agent-policy-import-artifact agent-policy-import-demo-report.txt",
        "ui-patch-import-artifact ui-patch-import-demo-report.txt",
        "agent-contracts-result accepted",
    ]:
        require_line(contracts_text, required, "agent contracts report")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Validate AgentTool policy import evidence and write a deterministic "
            "role-by-role multi-agent handoff report."
        )
    )
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument("--capture-plan-dir", required=True)
    parser.add_argument("--import-work-dir", required=True)
    parser.add_argument("--output-artifacts", required=True)
    parser.add_argument("--source-entry-id", default="example-40")
    parser.add_argument("--output-dir", required=True)
    parsed = parser.parse_args(argv)
    parsed.examples_artifacts = Path(parsed.examples_artifacts)
    parsed.capture_plan_dir = Path(parsed.capture_plan_dir)
    parsed.import_work_dir = Path(parsed.import_work_dir)
    parsed.output_artifacts = Path(parsed.output_artifacts)
    parsed.output_dir = Path(parsed.output_dir)
    return parsed


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    plan, plan_fingerprint = read_plan(args.capture_plan_dir)
    validate_plan(plan, args.source_entry_id)
    source_entry_id = plan_string(plan, "source_entry_id")
    proposed_entry_id = plan_string(plan, "proposed_entry_id")

    import_report_text, import_report_fingerprint = read_fingerprinted_text(
        args.import_work_dir / "agent-policy-import-demo-report.txt",
        args.import_work_dir / "agent-policy-import-demo-report.fingerprint.txt",
        "agent policy import demo report",
    )
    for required in [
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        "source-preserved true",
        "proposed-accepted true",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
    ]:
        require_line(import_report_text, required, "agent policy import demo report")

    source_review_text, source_review_fingerprint = read_fingerprinted_text(
        args.examples_artifacts / "examples" / source_entry_id / "agent-policy-review.txt",
        args.examples_artifacts
        / "examples"
        / source_entry_id
        / "agent-policy-review.fingerprint.txt",
        "agent policy review",
    )
    if source_review_fingerprint != plan_string(plan, "agent_policy_review_fingerprint"):
        raise SystemExit("agent policy review fingerprint does not match capture plan")
    for required in [
        "human-approval-required true",
        "agent-contract-check ail-agent-contracts examples/agents",
        f"handoff-roles {EXPECTED_HANDOFF_ROLES}",
        "tool-permission-review required",
        "tool-approval-review required",
        "external-call-review required",
        "secret-redaction-review required",
        "audit-trace-review required",
    ]:
        require_line(source_review_text, required, "agent policy review")

    proposed_core = (
        args.output_artifacts / "examples" / proposed_entry_id / "checked.ail-core.txt"
    )
    proposed_core_text = proposed_core.read_text()
    require_line(proposed_core_text, f"node Trace {POLICY_HANDOFF_TRACE}", "promoted core")
    contracts_text = run_agent_contracts()
    validate_contracts(contracts_text)

    output_lines = [
        "AIL-Agent-Policy-Multi-Agent-Handoff:",
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        "agent-contracts-result accepted",
        "separate-reviewer-role-count 5",
        "role requirements-writer contract codex-ail-requirements-writer evidence source-request-reviewed",
        "role spec-writer contract codex-ail-spec-writer evidence checked-core-fingerprint "
        f"{fnv64(proposed_core_text)}",
        "role diagnostic-repairer contract codex-ail-diagnostic-repairer evidence diagnostics-not-required",
        "role prompt-reviewer contract codex-ail-prompt-reviewer evidence v03-roadmap-reviewed",
        "role agent-policy-reviewer contract codex-ail-agent-policy-reviewer evidence agent-policy-import-demo-report",
        f"agent-policy-review-fingerprint {source_review_fingerprint}",
        f"agent-policy-capture-plan-fingerprint {plan_fingerprint}",
        f"agent-policy-import-demo-fingerprint {import_report_fingerprint}",
        "human-approval-required true",
        "source-preserved true",
        "proposed-accepted true",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
        "multi-agent-execution-evidence deterministic-role-handoff",
        "",
    ]
    output_text = "\n".join(output_lines)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    report_path = args.output_dir / "agent-policy-multi-agent-handoff-report.txt"
    report_path.write_text(output_text)
    (
        args.output_dir / "agent-policy-multi-agent-handoff-report.fingerprint.txt"
    ).write_text(fnv64(output_text) + "\n")
    sys.stdout.write(output_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
