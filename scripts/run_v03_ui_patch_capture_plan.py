#!/usr/bin/env python3
"""Build a deterministic capture plan from proposed UI patch evidence."""

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


def parse_patch_report(report_text: str, errors: list[str]) -> dict[str, str]:
    if not report_text.startswith("AIL-UI-Review-Patch:\n"):
        errors.append("UI review patch missing AIL-UI-Review-Patch header")
    values: dict[str, str] = {}
    for line in report_text.splitlines():
        if not line or line.endswith(":"):
            continue
        if " " not in line:
            errors.append(f"UI review patch line missing value: {line}")
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
    patch_path = entry_dir / "ui-review-patch.txt"
    patch_text = read_text(patch_path, errors)
    values = parse_patch_report(patch_text, errors)
    patch_fingerprint = fnv64(patch_text)
    stored_patch_fingerprint = read_text(
        entry_dir / "ui-review-patch.fingerprint.txt", errors
    ).strip()
    if stored_patch_fingerprint != patch_fingerprint:
        errors.append(
            "UI review patch fingerprint expected "
            f"{stored_patch_fingerprint or '<missing>'}, got {patch_fingerprint}"
        )

    require_value(values, "entry", args.entry_id, errors)
    require_value(values, "patch-source", "ui-review", errors)
    require_value(values, "visual-review-patch-plan", "deterministic-text", errors)
    require_value(values, "patch-command", "ail-flow-edit", errors)
    require_value(values, "human-approval-required", "true", errors)
    require_value(values, "patch-import-status", "proposed-only", errors)

    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/ui-review.txt",
        values.get("ui-review-fingerprint", ""),
        "UI review",
        errors,
    )
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
    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/vm-trace.txt",
        values.get("vm-trace-fingerprint", ""),
        "VM trace",
        errors,
    )
    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/target-report.txt",
        values.get("target-report-fingerprint", ""),
        "target report",
        errors,
    )

    manifest_line = (
        f"entry-artifact {args.entry_id} ui-review-patch "
        f"examples/{args.entry_id}/ui-review-patch.txt {patch_fingerprint}"
    )
    for report_name in ["examples-report.txt", "manifest.ail-examples.txt"]:
        report_text = read_text(artifact_root / report_name, errors)
        if manifest_line not in report_text:
            errors.append(f"{report_name} missing {manifest_line}")

    if errors:
        raise SystemExit("\n".join(errors))
    return values, patch_fingerprint


def build_plan(
    args: argparse.Namespace, values: dict[str, str], patch_fingerprint: str
) -> dict[str, object]:
    proposed_entry_id = args.proposed_entry_id or f"{args.entry_id}-ui-patch"
    return {
        "artifact_kind": "AIL-UI-Patch-Capture-Plan",
        "batch_capture_script": BATCH_CAPTURE_SCRIPT,
        "capture_command_template": [
            "python3",
            BATCH_CAPTURE_SCRIPT,
            "--plan-json",
            "<human-approved-ui-patch-batch.json>",
        ],
        "checked_core_fingerprint": values["checked-core-fingerprint"],
        "human_approval_required": values["human-approval-required"] == "true",
        "must_supply_request_response_json": True,
        "patch_command": values["patch-command"],
        "patch_import_decision": "accepted-for-import",
        "patch_import_status": values["patch-import-status"],
        "patch_scope": values["patch-scope"],
        "patch_source": values["patch-source"],
        "preserve_source_entry": True,
        "proposed_entry_id": proposed_entry_id,
        "source_entry_id": args.entry_id,
        "status": "plan-only",
        "target_report_fingerprint": values["target-report-fingerprint"],
        "ui_review_fingerprint": values["ui-review-fingerprint"],
        "ui_review_patch_artifact": f"examples/{args.entry_id}/ui-review-patch.txt",
        "ui_review_patch_fingerprint": patch_fingerprint,
        "visual_review_patch_plan": values["visual-review-patch-plan"],
        "vm_trace_fingerprint": values["vm-trace-fingerprint"],
    }


def render_plan_text(plan: dict[str, object]) -> str:
    command = " ".join(str(part) for part in plan["capture_command_template"])
    return "\n".join(
        [
            "AIL-UI-Patch-Capture-Plan:",
            f"source-entry-id {plan['source_entry_id']}",
            f"proposed-entry-id {plan['proposed_entry_id']}",
            f"status {plan['status']}",
            f"patch-import-decision {plan['patch_import_decision']}",
            f"patch-command {plan['patch_command']}",
            f"patch-scope {plan['patch_scope']}",
            f"human-approval-required {str(plan['human_approval_required']).lower()}",
            f"preserve-source-entry {str(plan['preserve_source_entry']).lower()}",
            "must-supply-request-response-json "
            f"{str(plan['must_supply_request_response_json']).lower()}",
            f"batch-capture-script {plan['batch_capture_script']}",
            f"capture-command-template {command}",
            f"ui-review-patch-fingerprint {plan['ui_review_patch_fingerprint']}",
            f"ui-review-fingerprint {plan['ui_review_fingerprint']}",
            f"checked-core-fingerprint {plan['checked_core_fingerprint']}",
            f"vm-trace-fingerprint {plan['vm_trace_fingerprint']}",
            f"target-report-fingerprint {plan['target_report_fingerprint']}",
            "plan-json ui-patch-capture-plan.json",
            "plan-fingerprint ui-patch-capture-plan.fingerprint.txt",
            "",
        ]
    )


def write_plan(output_dir: Path, plan: dict[str, object]) -> str:
    output_dir.mkdir(parents=True, exist_ok=True)
    plan_json = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    plan_text = render_plan_text(plan)
    (output_dir / "ui-patch-capture-plan.json").write_text(plan_json)
    (output_dir / "ui-patch-capture-plan.txt").write_text(plan_text)
    (output_dir / "ui-patch-capture-plan.fingerprint.txt").write_text(
        fnv64(plan_json) + "\n"
    )
    return plan_text


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a plan-only capture artifact from UI review patch evidence."
    )
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument("--entry-id", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--proposed-entry-id")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    values, patch_fingerprint = validate_inputs(args)
    plan = build_plan(args, values, patch_fingerprint)
    sys.stdout.write(write_plan(Path(args.output_dir), plan))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
