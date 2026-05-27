#!/usr/bin/env python3
"""Build a deterministic capture plan from proposed AgentTool policy evidence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


BATCH_CAPTURE_SCRIPT = "scripts/capture_example_batch.py"


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def read_text(path: Path, errors: list[str]) -> str:
    if not path.exists():
        errors.append(f"missing file {path}")
        return ""
    return path.read_text()


def parse_policy_review(review_text: str, errors: list[str]) -> dict[str, str]:
    if not review_text.startswith("AIL-Agent-Policy-Review:\n"):
        errors.append("agent policy review missing AIL-Agent-Policy-Review header")
    values: dict[str, str] = {}
    for line in review_text.splitlines():
        if not line or line.endswith(":"):
            continue
        if " " not in line:
            errors.append(f"agent policy review line missing value: {line}")
            continue
        key, value = line.split(" ", 1)
        values[key] = value
    return values


def require_value(
    values: dict[str, str], key: str, expected: str, errors: list[str]
) -> None:
    actual = values.get(key)
    if actual != expected:
        errors.append(f"{key} expected {expected}, got {actual or '<missing>'}")


def require_fingerprint(
    artifact_root: Path,
    relative_path: str,
    expected: str,
    label: str,
    errors: list[str],
) -> str:
    text = read_text(artifact_root / relative_path, errors)
    actual = fnv64(text)
    if expected != actual:
        errors.append(f"{label} fingerprint expected {expected}, got {actual}")
    return actual


def validate_inputs(args: argparse.Namespace) -> tuple[dict[str, str], str]:
    artifact_root = Path(args.examples_artifacts)
    entry_dir = artifact_root / "examples" / args.entry_id
    errors: list[str] = []
    review_path = entry_dir / "agent-policy-review.txt"
    review_text = read_text(review_path, errors)
    values = parse_policy_review(review_text, errors)
    review_fingerprint = fnv64(review_text)
    stored_review_fingerprint = read_text(
        entry_dir / "agent-policy-review.fingerprint.txt", errors
    ).strip()
    if stored_review_fingerprint != review_fingerprint:
        errors.append(
            "agent policy review fingerprint expected "
            f"{stored_review_fingerprint or '<missing>'}, got {review_fingerprint}"
        )

    require_value(values, "entry", args.entry_id, errors)
    require_value(values, "profile", "AgentTool", errors)
    require_value(values, "program-domain", "agent-tool", errors)
    require_value(values, "agent-policy-review-artifact", "deterministic-text", errors)
    require_value(values, "multi-agent-handoff-review", "required", errors)
    require_value(values, "agent-contract-check", "ail-agent-contracts examples/agents", errors)
    require_value(values, "tool-permission-review", "required", errors)
    require_value(values, "tool-approval-review", "required", errors)
    require_value(values, "external-call-review", "required", errors)
    require_value(values, "secret-redaction-review", "required", errors)
    require_value(values, "audit-trace-review", "required", errors)
    require_value(values, "human-approval-required", "true", errors)
    require_value(values, "policy-import-status", "proposed-only", errors)

    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/checked.ail-core.txt",
        values.get("checked-core-fingerprint", ""),
        "checked Core",
        errors,
    )
    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/artifact.ailbc.json",
        values.get("bytecode-fingerprint", ""),
        "bytecode",
        errors,
    )
    if "vm-trace-fingerprint" in values:
        require_fingerprint(
            artifact_root,
            f"examples/{args.entry_id}/vm-trace.txt",
            values["vm-trace-fingerprint"],
            "VM trace",
            errors,
        )
    if "target-report-fingerprint" in values:
        require_fingerprint(
            artifact_root,
            f"examples/{args.entry_id}/target-report.txt",
            values["target-report-fingerprint"],
            "target report",
            errors,
        )

    manifest_line = (
        f"entry-artifact {args.entry_id} agent-policy-review "
        f"examples/{args.entry_id}/agent-policy-review.txt {review_fingerprint}"
    )
    for report_name in ["examples-report.txt", "manifest.ail-examples.txt"]:
        report_text = read_text(artifact_root / report_name, errors)
        if manifest_line not in report_text:
            errors.append(f"{report_name} missing {manifest_line}")

    if errors:
        raise SystemExit("\n".join(errors))
    return values, review_fingerprint


def build_plan(
    args: argparse.Namespace, values: dict[str, str], review_fingerprint: str
) -> dict[str, object]:
    proposed_entry_id = args.proposed_entry_id or f"{args.entry_id}-policy"
    return {
        "agent_contract_check": values["agent-contract-check"],
        "agent_policy_review_artifact": f"examples/{args.entry_id}/agent-policy-review.txt",
        "agent_policy_review_fingerprint": review_fingerprint,
        "artifact_kind": "AIL-Agent-Policy-Capture-Plan",
        "audit_trace_review": values["audit-trace-review"],
        "batch_capture_script": BATCH_CAPTURE_SCRIPT,
        "capture_command_template": [
            "python3",
            BATCH_CAPTURE_SCRIPT,
            "--plan-json",
            "<human-approved-agent-policy-batch.json>",
        ],
        "checked_core_fingerprint": values["checked-core-fingerprint"],
        "external_call_review": values["external-call-review"],
        "handoff_roles": values["handoff-roles"],
        "human_approval_required": values["human-approval-required"] == "true",
        "must_supply_request_response_json": True,
        "policy_import_decision": "accepted-for-import",
        "policy_import_status": values["policy-import-status"],
        "preserve_source_entry": True,
        "profile": values["profile"],
        "program_domain": values["program-domain"],
        "proposed_entry_id": proposed_entry_id,
        "runtime_evidence": values["runtime-evidence"],
        "secret_redaction_review": values["secret-redaction-review"],
        "source_entry_id": args.entry_id,
        "status": "plan-only",
        "tool_approval_review": values["tool-approval-review"],
        "tool_permission_review": values["tool-permission-review"],
    }


def render_plan_text(plan: dict[str, object]) -> str:
    command = " ".join(str(part) for part in plan["capture_command_template"])
    return "\n".join(
        [
            "AIL-Agent-Policy-Capture-Plan:",
            f"source-entry-id {plan['source_entry_id']}",
            f"proposed-entry-id {plan['proposed_entry_id']}",
            f"status {plan['status']}",
            f"policy-import-decision {plan['policy_import_decision']}",
            f"policy-import-status {plan['policy_import_status']}",
            f"profile {plan['profile']}",
            f"program-domain {plan['program_domain']}",
            f"agent-contract-check {plan['agent_contract_check']}",
            f"handoff-roles {plan['handoff_roles']}",
            f"human-approval-required {str(plan['human_approval_required']).lower()}",
            f"preserve-source-entry {str(plan['preserve_source_entry']).lower()}",
            "must-supply-request-response-json "
            f"{str(plan['must_supply_request_response_json']).lower()}",
            f"batch-capture-script {plan['batch_capture_script']}",
            f"capture-command-template {command}",
            f"agent-policy-review-fingerprint {plan['agent_policy_review_fingerprint']}",
            f"checked-core-fingerprint {plan['checked_core_fingerprint']}",
            f"runtime-evidence {plan['runtime_evidence']}",
            "plan-json agent-policy-capture-plan.json",
            "plan-fingerprint agent-policy-capture-plan.fingerprint.txt",
            "",
        ]
    )


def write_plan(output_dir: Path, plan: dict[str, object]) -> str:
    output_dir.mkdir(parents=True, exist_ok=True)
    plan_json = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    plan_text = render_plan_text(plan)
    (output_dir / "agent-policy-capture-plan.json").write_text(plan_json)
    (output_dir / "agent-policy-capture-plan.txt").write_text(plan_text)
    (output_dir / "agent-policy-capture-plan.fingerprint.txt").write_text(
        fnv64(plan_json) + "\n"
    )
    return plan_text


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a plan-only capture artifact from AgentTool policy review evidence."
    )
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument("--entry-id", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--proposed-entry-id")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    values, review_fingerprint = validate_inputs(args)
    plan = build_plan(args, values, review_fingerprint)
    sys.stdout.write(write_plan(Path(args.output_dir), plan))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
