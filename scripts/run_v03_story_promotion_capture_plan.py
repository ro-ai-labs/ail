#!/usr/bin/env python3
"""Build a deterministic capture plan from accepted story-mode LLM evidence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


BATCH_CAPTURE_SCRIPT = "scripts/capture_example_batch.py"
STORY_REVIEW_REPORT = "story-llm-harness-report.txt"
STORY_REVIEW_FINGERPRINT = "story-llm-harness-report.fingerprint.txt"


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


def parse_key_values(text: str, header: str, label: str, errors: list[str]) -> dict[str, str]:
    if not text.startswith(header + "\n"):
        errors.append(f"{label} missing {header} header")
    values: dict[str, str] = {}
    for line in text.splitlines():
        if not line or line.endswith(":"):
            continue
        colon_index = line.find(":")
        space_index = line.find(" ")
        if colon_index != -1 and (space_index == -1 or colon_index < space_index):
            key, value = line.split(":", 1)
        elif " " in line:
            key, value = line.split(" ", 1)
        else:
            errors.append(f"{label} line missing value: {line}")
            continue
        values[key.strip()] = value.strip()
    return values


def require_value(
    values: dict[str, str], key: str, expected: str, label: str, errors: list[str]
) -> None:
    actual = values.get(key)
    if actual != expected:
        errors.append(f"{label} {key} expected {expected}, got {actual or '<missing>'}")


def require_fingerprint(path: Path, fingerprint_path: Path, errors: list[str]) -> str:
    text = read_text(path, errors)
    expected = read_text(fingerprint_path, errors).strip()
    actual = fnv64(text)
    if expected != actual:
        errors.append(
            f"fingerprint mismatch {path}: expected {expected or '<missing>'} got {actual}"
        )
    return actual


def validate_story_artifacts(
    story_artifacts: Path,
) -> tuple[dict[str, str], dict[str, str], str, str, str, str]:
    errors: list[str] = []
    review_path = story_artifacts / STORY_REVIEW_REPORT
    review_fingerprint_path = story_artifacts / STORY_REVIEW_FINGERPRINT
    review_text = read_text(review_path, errors)
    review_fingerprint = require_fingerprint(review_path, review_fingerprint_path, errors)
    review_values = parse_key_values(
        review_text, "AIL-Story-LLM-Harness-Review:", "story review", errors
    )
    require_value(review_values, "review-result", "accepted", "story review", errors)
    require_value(review_values, "agent-trace", "present", "story review", errors)
    require_value(review_values, "model-check", "present", "story review", errors)
    require_value(
        review_values, "story-llm-transcript-check-count", "6", "story review", errors
    )
    require_value(
        review_values, "story-prompt-envelope-valid-count", "2", "story review", errors
    )
    require_value(
        review_values, "story-prompt-envelope-artifact-count", "2", "story review", errors
    )
    require_value(
        review_values, "story-prompt-envelope-questions-count", "0", "story review", errors
    )
    require_value(
        review_values, "story-prompt-envelope-invalid-count", "0", "story review", errors
    )

    report_path = story_artifacts / "story-mode-report.txt"
    report_text = read_text(report_path, errors)
    story_values = parse_key_values(
        report_text, "AIL-Story-Mode-Report:", "story mode report", errors
    )
    story_id = story_values.get("user-story-id", "")
    if not story_id:
        errors.append("story mode report missing user-story-id")
    require_value(story_values, "entrypoint", "ail-story", "story mode report", errors)
    require_value(
        story_values,
        "story-prompt-envelope-valid-count",
        "2",
        "story mode report",
        errors,
    )
    require_value(
        story_values,
        "story-prompt-envelope-invalid-count",
        "0",
        "story mode report",
        errors,
    )

    story_report_fingerprint = require_fingerprint(
        report_path, story_artifacts / "story-mode-report.fingerprint.txt", errors
    )
    story_manifest_fingerprint = require_fingerprint(
        story_artifacts / "manifest.ail-story.txt",
        story_artifacts / "manifest.ail-story.fingerprint.txt",
        errors,
    )
    model_check_fingerprint = require_fingerprint(
        story_artifacts / "model-check.json",
        story_artifacts / "model-check.fingerprint.txt",
        errors,
    )

    for relative_path, fingerprint_path in [
        ("story.source.md", "story.source.fingerprint.txt"),
        ("story.normalized.md", "story.normalized.fingerprint.txt"),
        ("requirements.ail-requirements.md", "requirements.fingerprint.txt"),
        ("accepted.ail-spec.md", "accepted.ail-spec.fingerprint.txt"),
        ("checked.ail-core.txt", "checked.ail-core.fingerprint.txt"),
        ("review.ail-flow.json", "review.ail-flow.fingerprint.txt"),
        ("artifact.ailbc.json", "artifact.fingerprint.txt"),
        ("agent-trace.txt", "agent-trace.fingerprint.txt"),
        ("llm/requirements.request.json", "llm/requirements.request.fingerprint.txt"),
        ("llm/requirements.response.json", "llm/requirements.response.fingerprint.txt"),
        ("llm/requirements.content.txt", "llm/requirements.content.fingerprint.txt"),
        ("llm/spec.request.json", "llm/spec.request.fingerprint.txt"),
        ("llm/spec.response.json", "llm/spec.response.fingerprint.txt"),
        ("llm/spec.content.txt", "llm/spec.content.fingerprint.txt"),
    ]:
        require_fingerprint(
            story_artifacts / relative_path,
            story_artifacts / fingerprint_path,
            errors,
        )

    if errors:
        raise SystemExit("\n".join(errors))
    return (
        review_values,
        story_values,
        review_fingerprint,
        story_report_fingerprint,
        story_manifest_fingerprint,
        model_check_fingerprint,
    )


def build_plan(
    args: argparse.Namespace,
    review_values: dict[str, str],
    story_values: dict[str, str],
    review_fingerprint: str,
    story_report_fingerprint: str,
    story_manifest_fingerprint: str,
    model_check_fingerprint: str,
) -> dict[str, object]:
    story_id = story_values["user-story-id"]
    return {
        "artifact_kind": "AIL-Story-Promotion-Capture-Plan",
        "batch_capture_script": BATCH_CAPTURE_SCRIPT,
        "capture_command_template": [
            "python3",
            BATCH_CAPTURE_SCRIPT,
            "--plan-json",
            "<human-approved-story-promotion-batch.json>",
        ],
        "human_approval_required": True,
        "must_supply_request_response_json": True,
        "preserve_story_artifacts": True,
        "promotion_decision": "accepted-for-promotion",
        "status": "plan-only",
        "story_artifact_dir": str(args.story_artifacts),
        "story_id": story_id,
        "story_llm_harness_review_fingerprint": review_fingerprint,
        "story_llm_transcript_check_count": int(
            review_values["story-llm-transcript-check-count"]
        ),
        "story_manifest_fingerprint": story_manifest_fingerprint,
        "story_model_check_fingerprint": model_check_fingerprint,
        "story_model_check_model_id": review_values["model-check-model-id"],
        "story_mode_report_fingerprint": story_report_fingerprint,
        "story_prompt_envelope_invalid_count": int(
            review_values["story-prompt-envelope-invalid-count"]
        ),
        "story_prompt_envelope_artifact_count": int(
            review_values["story-prompt-envelope-artifact-count"]
        ),
        "story_prompt_envelope_questions_count": int(
            review_values["story-prompt-envelope-questions-count"]
        ),
        "story_prompt_envelope_valid_count": int(
            review_values["story-prompt-envelope-valid-count"]
        ),
    }


def render_plan_text(plan: dict[str, object]) -> str:
    command = " ".join(str(part) for part in plan["capture_command_template"])
    return "\n".join(
        [
            "AIL-Story-Promotion-Capture-Plan:",
            f"story-id {plan['story_id']}",
            f"status {plan['status']}",
            f"promotion-decision {plan['promotion_decision']}",
            f"human-approval-required {str(plan['human_approval_required']).lower()}",
            "must-supply-request-response-json "
            f"{str(plan['must_supply_request_response_json']).lower()}",
            f"preserve-story-artifacts {str(plan['preserve_story_artifacts']).lower()}",
            f"batch-capture-script {plan['batch_capture_script']}",
            f"capture-command-template {command}",
            f"story-artifact-dir {plan['story_artifact_dir']}",
            "story-llm-harness-review-fingerprint "
            f"{plan['story_llm_harness_review_fingerprint']}",
            f"story-mode-report-fingerprint {plan['story_mode_report_fingerprint']}",
            f"story-manifest-fingerprint {plan['story_manifest_fingerprint']}",
            "story-model-check-fingerprint "
            f"{plan['story_model_check_fingerprint']}",
            f"story-model-check-model-id {plan['story_model_check_model_id']}",
            "story-llm-transcript-check-count "
            f"{plan['story_llm_transcript_check_count']}",
            "story-prompt-envelope-valid-count "
            f"{plan['story_prompt_envelope_valid_count']}",
            "story-prompt-envelope-artifact-count "
            f"{plan['story_prompt_envelope_artifact_count']}",
            "story-prompt-envelope-questions-count "
            f"{plan['story_prompt_envelope_questions_count']}",
            "story-prompt-envelope-invalid-count "
            f"{plan['story_prompt_envelope_invalid_count']}",
            "plan-json story-promotion-capture-plan.json",
            "plan-fingerprint story-promotion-capture-plan.fingerprint.txt",
            "",
        ]
    )


def write_plan(output_dir: Path, plan: dict[str, object]) -> str:
    output_dir.mkdir(parents=True, exist_ok=True)
    plan_json = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    plan_text = render_plan_text(plan)
    (output_dir / "story-promotion-capture-plan.json").write_text(plan_json)
    (output_dir / "story-promotion-capture-plan.txt").write_text(plan_text)
    (output_dir / "story-promotion-capture-plan.fingerprint.txt").write_text(
        fnv64(plan_json) + "\n"
    )
    return plan_text


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a plan-only capture artifact from accepted story-mode LLM evidence."
    )
    parser.add_argument("--story-artifacts", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    (
        review_values,
        story_values,
        review_fingerprint,
        story_report_fingerprint,
        story_manifest_fingerprint,
        model_check_fingerprint,
    ) = validate_story_artifacts(args.story_artifacts)
    plan = build_plan(
        args,
        review_values,
        story_values,
        review_fingerprint,
        story_report_fingerprint,
        story_manifest_fingerprint,
        model_check_fingerprint,
    )
    sys.stdout.write(write_plan(args.output_dir, plan))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
