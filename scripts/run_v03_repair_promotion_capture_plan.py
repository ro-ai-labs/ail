#!/usr/bin/env python3
"""Build a deterministic capture plan from accepted repair-promotion evidence."""

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


def parse_review(review_text: str, errors: list[str]) -> dict[str, str]:
    if not review_text.startswith("AIL-Repair-Promotion-Review:\n"):
        errors.append("repair promotion review missing AIL-Repair-Promotion-Review header")
    values: dict[str, str] = {}
    for line in review_text.splitlines():
        if not line or line.endswith(":"):
            continue
        if " " not in line:
            errors.append(f"review line missing value: {line}")
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
) -> None:
    text = read_text(artifact_root / relative_path, errors)
    actual = fnv64(text)
    if expected != actual:
        errors.append(f"{label} fingerprint expected {expected}, got {actual}")


def validate_inputs(args: argparse.Namespace) -> tuple[dict[str, str], str]:
    artifact_root = Path(args.examples_artifacts)
    entry_dir = artifact_root / "examples" / args.entry_id
    errors: list[str] = []
    review_path = entry_dir / "repair-promotion-review.txt"
    review_text = read_text(review_path, errors)
    values = parse_review(review_text, errors)
    review_fingerprint = fnv64(review_text)
    stored_review_fingerprint = read_text(
        entry_dir / "repair-promotion-review.fingerprint.txt", errors
    ).strip()
    if stored_review_fingerprint != review_fingerprint:
        errors.append(
            "repair promotion review fingerprint expected "
            f"{stored_review_fingerprint or '<missing>'}, got {review_fingerprint}"
        )

    require_value(values, "entry", args.entry_id, errors)
    require_value(values, "promotion-decision", "accepted-for-promotion", errors)
    require_value(values, "human-approval-required", "true", errors)
    require_value(values, "checker-result", "rejected-to-repaired", errors)
    require_value(values, "expected-diagnostic-removed", "true", errors)
    require_value(values, "semantic-anchor-missing-count", "0", errors)

    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/repair-candidate.ail-spec.md",
        values.get("repair-candidate-fingerprint", ""),
        "repair candidate",
        errors,
    )
    require_fingerprint(
        artifact_root,
        f"examples/{args.entry_id}/repair-diff.txt",
        values.get("repair-diff-fingerprint", ""),
        "repair diff",
        errors,
    )

    review_manifest_line = (
        f"entry-artifact {args.entry_id} repair-promotion-review "
        f"examples/{args.entry_id}/repair-promotion-review.txt {review_fingerprint}"
    )
    for report_name in ["examples-report.txt", "manifest.ail-examples.txt"]:
        report_text = read_text(artifact_root / report_name, errors)
        if review_manifest_line not in report_text:
            errors.append(f"{report_name} missing {review_manifest_line}")

    if errors:
        raise SystemExit("\n".join(errors))
    return values, review_fingerprint


def build_plan(
    args: argparse.Namespace, values: dict[str, str], review_fingerprint: str
) -> dict[str, object]:
    return {
        "artifact_kind": "AIL-Repair-Promotion-Capture-Plan",
        "batch_capture_script": BATCH_CAPTURE_SCRIPT,
        "capture_command_template": [
            "python3",
            BATCH_CAPTURE_SCRIPT,
            "--batch-file",
            "<human-approved-repair-promotion-batch.json>",
        ],
        "checker_result": values["checker-result"],
        "expected_diagnostic": values.get("expected-diagnostic", ""),
        "expected_diagnostic_removed": values["expected-diagnostic-removed"] == "true",
        "human_approval_required": values["human-approval-required"] == "true",
        "must_supply_request_response_json": True,
        "preserve_rejected_entry": True,
        "promotion_decision": values["promotion-decision"],
        "proposed_entry_id": values["proposed-accepted-entry-id"],
        "repair_candidate_fingerprint": values["repair-candidate-fingerprint"],
        "repair_diff_fingerprint": values["repair-diff-fingerprint"],
        "repair_promotion_review_fingerprint": review_fingerprint,
        "review_artifact": f"examples/{args.entry_id}/repair-promotion-review.txt",
        "semantic_anchor_missing_count": int(values["semantic-anchor-missing-count"]),
        "source_entry_id": args.entry_id,
        "status": "plan-only",
    }


def render_plan_text(plan: dict[str, object]) -> str:
    command = " ".join(str(part) for part in plan["capture_command_template"])
    return "\n".join(
        [
            "AIL-Repair-Promotion-Capture-Plan:",
            f"source-entry-id {plan['source_entry_id']}",
            f"proposed-entry-id {plan['proposed_entry_id']}",
            f"status {plan['status']}",
            f"promotion-decision {plan['promotion_decision']}",
            f"human-approval-required {str(plan['human_approval_required']).lower()}",
            f"preserve-rejected-entry {str(plan['preserve_rejected_entry']).lower()}",
            "must-supply-request-response-json "
            f"{str(plan['must_supply_request_response_json']).lower()}",
            f"batch-capture-script {plan['batch_capture_script']}",
            f"capture-command-template {command}",
            "repair-promotion-review-fingerprint "
            f"{plan['repair_promotion_review_fingerprint']}",
            f"repair-candidate-fingerprint {plan['repair_candidate_fingerprint']}",
            f"repair-diff-fingerprint {plan['repair_diff_fingerprint']}",
            f"semantic-anchor-missing-count {plan['semantic_anchor_missing_count']}",
            "plan-json repair-promotion-capture-plan.json",
            "plan-fingerprint repair-promotion-capture-plan.fingerprint.txt",
            "",
        ]
    )


def write_plan(output_dir: Path, plan: dict[str, object]) -> str:
    output_dir.mkdir(parents=True, exist_ok=True)
    plan_json = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    plan_text = render_plan_text(plan)
    (output_dir / "repair-promotion-capture-plan.json").write_text(plan_json)
    (output_dir / "repair-promotion-capture-plan.txt").write_text(plan_text)
    (output_dir / "repair-promotion-capture-plan.fingerprint.txt").write_text(
        fnv64(plan_json) + "\n"
    )
    return plan_text


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a plan-only capture artifact from repair promotion review evidence."
    )
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument("--entry-id", required=True)
    parser.add_argument("--output-dir", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    values, review_fingerprint = validate_inputs(args)
    plan = build_plan(args, values, review_fingerprint)
    sys.stdout.write(write_plan(Path(args.output_dir), plan))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
