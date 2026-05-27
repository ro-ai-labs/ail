#!/usr/bin/env python3
"""Write deterministic runtime UI-state evidence for an imported UI patch."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def read_fingerprinted_text(
    path: Path, fingerprint_path: Path, label: str
) -> tuple[str, str]:
    text = path.read_text()
    expected = fingerprint_path.read_text().strip()
    actual = fnv64(text)
    if expected != actual:
        raise SystemExit(f"{label} fingerprint mismatch: expected {expected} got {actual}")
    return text, actual


def read_plan(capture_plan_dir: Path) -> tuple[dict[str, object], str]:
    plan_text, fingerprint = read_fingerprinted_text(
        capture_plan_dir / "ui-patch-capture-plan.json",
        capture_plan_dir / "ui-patch-capture-plan.fingerprint.txt",
        "UI patch capture plan",
    )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("UI patch capture plan must be an object")
    return plan, fingerprint


def plan_string(plan: dict[str, object], field: str) -> str:
    value = plan.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"UI patch capture plan is missing {field}")
    return value


def require_plan_value(plan: dict[str, object], field: str, expected: object) -> None:
    actual = plan.get(field)
    if actual != expected:
        raise SystemExit(
            f"UI patch capture plan {field} expected {expected}, got {actual or '<missing>'}"
        )


def require_line(text: str, line: str, label: str) -> None:
    if line not in text:
        raise SystemExit(f"{label} missing {line}")


def validate_plan(plan: dict[str, object], source_entry_id: str) -> None:
    require_plan_value(plan, "artifact_kind", "AIL-UI-Patch-Capture-Plan")
    require_plan_value(plan, "source_entry_id", source_entry_id)
    require_plan_value(plan, "status", "plan-only")
    require_plan_value(plan, "patch_import_decision", "accepted-for-import")
    require_plan_value(plan, "patch_command", "ail-flow-edit")
    require_plan_value(plan, "patch_import_status", "proposed-only")
    require_plan_value(plan, "human_approval_required", True)
    require_plan_value(plan, "must_supply_request_response_json", True)
    require_plan_value(plan, "preserve_source_entry", True)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Validate imported UI patch artifacts and write deterministic "
            "visual-regression plus runtime UI-state evidence."
        )
    )
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument("--capture-plan-dir", required=True)
    parser.add_argument("--import-work-dir", required=True)
    parser.add_argument("--output-artifacts", required=True)
    parser.add_argument("--source-entry-id", default="example-108")
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
        args.import_work_dir / "ui-patch-import-demo-report.txt",
        args.import_work_dir / "ui-patch-import-demo-report.fingerprint.txt",
        "UI patch import demo report",
    )
    for required in [
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        "source-preserved true",
        "proposed-accepted true",
        "ui-review-patch-fingerprint-preserved true",
        "checked-core-fingerprint-preserved true",
        "flow-edit-applied true",
        "patched-core-replayed true",
    ]:
        require_line(import_report_text, required, "UI patch import demo report")

    source_artifact_dir = args.examples_artifacts / "examples" / source_entry_id
    ui_review_text, ui_review_fingerprint = read_fingerprinted_text(
        source_artifact_dir / "ui-review.txt",
        source_artifact_dir / "ui-review.fingerprint.txt",
        "UI review",
    )
    ui_patch_text, ui_patch_fingerprint = read_fingerprinted_text(
        source_artifact_dir / "ui-review-patch.txt",
        source_artifact_dir / "ui-review-patch.fingerprint.txt",
        "UI review patch",
    )
    if ui_review_fingerprint != plan_string(plan, "ui_review_fingerprint"):
        raise SystemExit("UI review fingerprint does not match capture plan")
    if ui_patch_fingerprint != plan_string(plan, "ui_review_patch_fingerprint"):
        raise SystemExit("UI review patch fingerprint does not match capture plan")
    for required in [
        "visual-review-artifact deterministic-text",
        "accessibility-review required",
        "runtime-evidence target-report",
        "semantic-anchor-preserved-count 6",
    ]:
        require_line(ui_review_text, required, "UI review")
    for required in [
        "visual-review-patch-plan deterministic-text",
        "patch-command ail-flow-edit",
        "human-approval-required true",
        "patch-import-status proposed-only",
    ]:
        require_line(ui_patch_text, required, "UI review patch")

    proposed_artifact_dir = args.output_artifacts / "examples" / proposed_entry_id
    proposed_core_text = (proposed_artifact_dir / "checked.ail-core.txt").read_text()
    (
        proposed_target_report_text,
        proposed_target_report_fingerprint,
    ) = read_fingerprinted_text(
        proposed_artifact_dir / "target-report.txt",
        proposed_artifact_dir / "target-report.fingerprint.txt",
        "promoted target report",
    )
    runtime_anchors = [
        "Ticket.reviewStatus",
        "RouteTicketDetailViewedScenario108",
        "FormValidationFailedScenario108",
        "DashboardViewedScenario108",
        "RefundApprovalWorkflowViewedScenario108",
    ]
    for anchor in runtime_anchors:
        require_line(proposed_core_text, anchor, "promoted checked Core")
    for required in [
        "AIL-Wasm-Contract-Report:",
        "target wasm32-unknown-sandbox-wasm",
        "status supported",
        "trace-preservation required",
    ]:
        require_line(proposed_target_report_text, required, "promoted target report")

    visual_regression_fingerprint_preserved = (
        ui_review_fingerprint == plan_string(plan, "ui_review_fingerprint")
        and ui_patch_fingerprint == plan_string(plan, "ui_review_patch_fingerprint")
    )
    output_lines = [
        "AIL-UI-Patch-Runtime-State-Check:",
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        f"visual-regression-baseline ui-review.txt {ui_review_fingerprint}",
        f"visual-regression-patch ui-review-patch.txt {ui_patch_fingerprint}",
        "visual-regression-fingerprint-preserved "
        f"{str(visual_regression_fingerprint_preserved).lower()}",
        "runtime-ui-state-check target-report",
        f"runtime-ui-state-anchor-count {len(runtime_anchors)}",
        *[f"runtime-ui-state-anchor {anchor}" for anchor in runtime_anchors],
        f"runtime-target-report-fingerprint {proposed_target_report_fingerprint}",
        f"ui-patch-capture-plan-fingerprint {plan_fingerprint}",
        f"ui-patch-import-demo-fingerprint {import_report_fingerprint}",
        "human-approval-required true",
        "source-preserved true",
        "proposed-accepted true",
        "flow-edit-applied true",
        "patched-core-replayed true",
        "",
    ]
    output_text = "\n".join(output_lines)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    report_path = args.output_dir / "ui-patch-runtime-state-check-report.txt"
    report_path.write_text(output_text)
    (
        args.output_dir / "ui-patch-runtime-state-check-report.fingerprint.txt"
    ).write_text(fnv64(output_text) + "\n")
    sys.stdout.write(output_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
