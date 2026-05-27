#!/usr/bin/env python3
"""Apply a batch of live LLM and Codex transcript captures to one examples copy."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

from capture_e2e_transcripts import (
    capture_completion,
    completion_body,
    fields_from_entry,
    fnv64,
    read_entries,
    refresh_distinctness_claim,
    render_entry,
    render_prompt,
)


ROOT = Path(__file__).resolve().parents[1]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capture multiple transcript entries into one offline AIL examples copy."
    )
    parser.add_argument("--base-corpus", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--plan-json", "--batch-file", dest="plan_json", required=True)
    return parser.parse_args()


def read_json_file(path: str) -> object:
    return json.loads(Path(path).read_text())


def write_json_file(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def required_string(entry: dict[str, object], field: str) -> str:
    value = entry.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"batch entry is missing {field}")
    return value


def optional_string(entry: dict[str, object], field: str) -> str | None:
    value = entry.get(field)
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise SystemExit(f"batch entry {required_string(entry, 'entry_id')} has invalid {field}")
    return value


def prompt_text_for_entry(entry: dict[str, object]) -> str:
    prompt = optional_string(entry, "prompt")
    prompt_file = optional_string(entry, "prompt_file")
    if (prompt is None) == (prompt_file is None):
        raise SystemExit(
            f"batch entry {required_string(entry, 'entry_id')} needs exactly one of prompt or prompt_file"
        )
    if prompt_file is not None:
        return Path(prompt_file).read_text().strip()
    return prompt or ""


def entry_index(entries: list[tuple[str | None, list[str]]], entry_id: str) -> int:
    replacement_index = next(
        (index for index, (candidate_id, _lines) in enumerate(entries) if candidate_id == entry_id),
        None,
    )
    if replacement_index is None:
        raise SystemExit(f"entry {entry_id} not found in batch corpus")
    return replacement_index


def story_fields_from_file(path: Path) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in path.read_text().splitlines():
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        key = key.strip()
        if key:
            fields[key] = value.strip()
    return fields


def render_story_file(fields: dict[str, str], semantic_anchors: str) -> str:
    return (
        f"# {fields['user-story-id']} User Story\n\n"
        f"user-story-id: {fields['user-story-id']}\n"
        f"user-story: {fields['user-story']}\n"
        f"acceptance-criteria: {fields['acceptance-criteria']}\n"
        f"story-journey: {fields['story-journey']}\n"
        f"story-roundtrip: {fields['story-roundtrip']}\n"
        f"story-evidence: {fields['story-evidence']}\n"
        f"program-domain: {fields['program-domain']}\n"
        f"module-count: {fields['module-count']}\n"
        f"spec-count: {fields['spec-count']}\n"
        f"story-count: {fields['story-count']}\n"
        f"interacts-with: {fields['interacts-with']}\n"
        f"semantic-anchors: {semantic_anchors}\n"
    )


def plan_string(plan: dict[str, object], field: str) -> str:
    value = plan.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"repair promotion capture plan is missing {field}")
    return value


def plan_bool(plan: dict[str, object], field: str) -> bool:
    value = plan.get(field)
    if not isinstance(value, bool):
        raise SystemExit(f"repair promotion capture plan has invalid {field}")
    return value


def plan_int(plan: dict[str, object], field: str) -> int:
    value = plan.get(field)
    if not isinstance(value, int):
        raise SystemExit(f"repair promotion capture plan has invalid {field}")
    return value


def repair_plan_fingerprint_path(plan_path: Path) -> Path:
    canonical = plan_path.with_name("repair-promotion-capture-plan.fingerprint.txt")
    if canonical.exists():
        return canonical
    return plan_path.with_suffix(".fingerprint.txt")


def story_plan_fingerprint_path(plan_path: Path) -> Path:
    canonical = plan_path.with_name("story-promotion-capture-plan.fingerprint.txt")
    if canonical.exists():
        return canonical
    return plan_path.with_suffix(".fingerprint.txt")


def ui_patch_plan_fingerprint_path(plan_path: Path) -> Path:
    canonical = plan_path.with_name("ui-patch-capture-plan.fingerprint.txt")
    if canonical.exists():
        return canonical
    return plan_path.with_suffix(".fingerprint.txt")


def require_story_artifact_fingerprint(
    artifact_dir: Path,
    relative_path: str,
    fingerprint_relative_path: str,
    expected: str | None = None,
) -> str:
    artifact_path = artifact_dir / relative_path
    fingerprint_path = artifact_dir / fingerprint_relative_path
    if not artifact_path.exists():
        raise SystemExit(f"story promotion artifacts missing {relative_path}")
    if not fingerprint_path.exists():
        raise SystemExit(f"story promotion artifacts missing {fingerprint_relative_path}")
    text = artifact_path.read_text()
    fingerprint = fingerprint_path.read_text().strip()
    actual = fnv64(text)
    if fingerprint != actual:
        raise SystemExit(
            "story promotion artifact fingerprint mismatch: "
            f"{relative_path} expected {fingerprint} got {actual}"
        )
    if expected is not None and actual != expected:
        raise SystemExit(
            "story promotion capture plan fingerprint mismatch: "
            f"{relative_path} expected {expected} got {actual}"
        )
    return actual


def load_repair_promotion_capture_plan(
    entry: dict[str, object],
) -> dict[str, object] | None:
    plan_json = optional_string(entry, "repair_promotion_capture_plan_json")
    if plan_json is None:
        return None
    plan_path = Path(plan_json)
    plan_text = plan_path.read_text()
    expected_fingerprint = repair_plan_fingerprint_path(plan_path).read_text().strip()
    actual_fingerprint = fnv64(plan_text)
    if expected_fingerprint != actual_fingerprint:
        raise SystemExit(
            "repair promotion capture plan fingerprint mismatch: "
            f"expected {expected_fingerprint} got {actual_fingerprint}"
        )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("repair promotion capture plan must be an object")
    if plan_string(plan, "artifact_kind") != "AIL-Repair-Promotion-Capture-Plan":
        raise SystemExit("repair promotion capture plan has invalid artifact_kind")
    for field, expected in [
        ("status", "plan-only"),
        ("promotion_decision", "accepted-for-promotion"),
        ("checker_result", "rejected-to-repaired"),
        ("batch_capture_script", "scripts/capture_example_batch.py"),
    ]:
        actual = plan_string(plan, field)
        if actual != expected:
            raise SystemExit(
                f"repair promotion capture plan {field} expected {expected}, got {actual}"
            )
    for field in [
        "human_approval_required",
        "must_supply_request_response_json",
        "preserve_rejected_entry",
        "expected_diagnostic_removed",
    ]:
        if not plan_bool(plan, field):
            raise SystemExit(f"repair promotion capture plan requires {field}")
    if plan_int(plan, "semantic_anchor_missing_count") != 0:
        raise SystemExit("repair promotion capture plan has missing semantic anchors")
    entry_id = required_string(entry, "entry_id")
    if plan_string(plan, "proposed_entry_id") != entry_id:
        raise SystemExit(
            "repair promotion capture plan proposed_entry_id must match batch entry_id"
        )
    source_entry_id = optional_string(entry, "source_entry_id")
    if source_entry_id is not None and plan_string(plan, "source_entry_id") != source_entry_id:
        raise SystemExit(
            "repair promotion capture plan source_entry_id must match batch source_entry_id"
        )
    return plan


def load_story_promotion_capture_plan(
    entry: dict[str, object],
) -> dict[str, object] | None:
    plan_json = optional_string(entry, "story_promotion_capture_plan_json")
    if plan_json is None:
        return None
    plan_path = Path(plan_json)
    plan_text = plan_path.read_text()
    expected_fingerprint = story_plan_fingerprint_path(plan_path).read_text().strip()
    actual_fingerprint = fnv64(plan_text)
    if expected_fingerprint != actual_fingerprint:
        raise SystemExit(
            "story promotion capture plan fingerprint mismatch: "
            f"expected {expected_fingerprint} got {actual_fingerprint}"
        )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("story promotion capture plan must be an object")
    if plan_string(plan, "artifact_kind") != "AIL-Story-Promotion-Capture-Plan":
        raise SystemExit("story promotion capture plan has invalid artifact_kind")
    for field, expected in [
        ("status", "plan-only"),
        ("promotion_decision", "accepted-for-promotion"),
        ("batch_capture_script", "scripts/capture_example_batch.py"),
    ]:
        actual = plan_string(plan, field)
        if actual != expected:
            raise SystemExit(
                f"story promotion capture plan {field} expected {expected}, got {actual}"
            )
    for field in [
        "human_approval_required",
        "must_supply_request_response_json",
        "preserve_story_artifacts",
    ]:
        if not plan_bool(plan, field):
            raise SystemExit(f"story promotion capture plan requires {field}")
    for field, expected in [
        ("story_llm_transcript_check_count", 6),
        ("story_prompt_envelope_valid_count", 2),
        ("story_prompt_envelope_invalid_count", 0),
    ]:
        actual = plan_int(plan, field)
        if actual != expected:
            raise SystemExit(
                f"story promotion capture plan {field} expected {expected}, got {actual}"
            )

    artifact_dir = Path(plan_string(plan, "story_artifact_dir"))
    if not artifact_dir.is_dir():
        raise SystemExit(f"story promotion artifact dir is missing: {artifact_dir}")
    require_story_artifact_fingerprint(
        artifact_dir,
        "story-llm-harness-report.txt",
        "story-llm-harness-report.fingerprint.txt",
        plan_string(plan, "story_llm_harness_review_fingerprint"),
    )
    require_story_artifact_fingerprint(
        artifact_dir,
        "story-mode-report.txt",
        "story-mode-report.fingerprint.txt",
        plan_string(plan, "story_mode_report_fingerprint"),
    )
    require_story_artifact_fingerprint(
        artifact_dir,
        "manifest.ail-story.txt",
        "manifest.ail-story.fingerprint.txt",
        plan_string(plan, "story_manifest_fingerprint"),
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
        require_story_artifact_fingerprint(artifact_dir, relative_path, fingerprint_path)
    report_fields = story_fields_from_file(artifact_dir / "story-mode-report.txt")
    normalized_story_fields = story_fields_from_file(artifact_dir / "story.normalized.md")
    if report_fields.get("entrypoint") != "ail-story":
        raise SystemExit("story promotion report missing entrypoint ail-story")
    story_id = plan_string(plan, "story_id")
    if report_fields.get("user-story-id") != story_id:
        raise SystemExit("story promotion report story id does not match capture plan")
    if normalized_story_fields.get("user-story-id") != story_id:
        raise SystemExit("story promotion normalized story id does not match capture plan")
    return plan


def load_ui_patch_capture_plan(
    entry: dict[str, object],
) -> dict[str, object] | None:
    plan_json = optional_string(entry, "ui_patch_capture_plan_json")
    if plan_json is None:
        return None
    plan_path = Path(plan_json)
    plan_text = plan_path.read_text()
    expected_fingerprint = ui_patch_plan_fingerprint_path(plan_path).read_text().strip()
    actual_fingerprint = fnv64(plan_text)
    if expected_fingerprint != actual_fingerprint:
        raise SystemExit(
            "ui patch capture plan fingerprint mismatch: "
            f"expected {expected_fingerprint} got {actual_fingerprint}"
        )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("ui patch capture plan must be an object")
    if plan_string(plan, "artifact_kind") != "AIL-UI-Patch-Capture-Plan":
        raise SystemExit("ui patch capture plan has invalid artifact_kind")
    for field, expected in [
        ("status", "plan-only"),
        ("patch_import_decision", "accepted-for-import"),
        ("patch_command", "ail-flow-edit"),
        ("patch_import_status", "proposed-only"),
        ("batch_capture_script", "scripts/capture_example_batch.py"),
    ]:
        actual = plan_string(plan, field)
        if actual != expected:
            raise SystemExit(f"ui patch capture plan {field} expected {expected}, got {actual}")
    for field in [
        "human_approval_required",
        "must_supply_request_response_json",
        "preserve_source_entry",
    ]:
        if not plan_bool(plan, field):
            raise SystemExit(f"ui patch capture plan requires {field}")
    entry_id = required_string(entry, "entry_id")
    if plan_string(plan, "proposed_entry_id") != entry_id:
        raise SystemExit("ui patch capture plan proposed_entry_id must match batch entry_id")
    source_entry_id = optional_string(entry, "source_entry_id")
    if source_entry_id is not None and plan_string(plan, "source_entry_id") != source_entry_id:
        raise SystemExit("ui patch capture plan source_entry_id must match batch source_entry_id")
    return plan


def apply_llm_entry(
    output_dir: Path,
    entries: list[tuple[str | None, list[str]]],
    entry: dict[str, object],
) -> None:
    entry_id = required_string(entry, "entry_id")
    index = entry_index(entries, entry_id)
    _entry_id, entry_lines = entries[index]
    fields = fields_from_entry(entry_lines)
    prompt_file = fields["prompt-file"]
    system_prompt = (ROOT / prompt_file).read_text()
    n_predict_value = entry.get("n_predict", 2048)
    if not isinstance(n_predict_value, int):
        raise SystemExit(f"batch entry {entry_id} has invalid n_predict")
    prompt = render_prompt(system_prompt, prompt_text_for_entry(entry), optional_string(entry, "input_json_file"))
    endpoint = required_string(entry, "endpoint")
    body = completion_body(endpoint, prompt, n_predict_value)
    response_json = capture_completion(endpoint, body)

    request_file = f"requests/{entry_id}.json"
    response_file = f"responses/{entry_id}.json"
    write_json_file(
        output_dir / request_file,
        {"endpoint": endpoint, "method": "POST", "body": body},
    )
    write_json_file(output_dir / response_file, response_json)

    fields.update(
        {
            "semantic-task": required_string(entry, "semantic_task"),
            "prompt-fingerprint": fnv64(system_prompt),
            "executor-family": "llm-http",
            "executor-label": required_string(entry, "executor_label"),
            "capture-origin": "live-llm",
            "endpoint-label": required_string(entry, "endpoint_label"),
            "request-file": request_file,
            "response-file": response_file,
            "checker-result": "accepted",
        }
    )
    refresh_distinctness_claim(fields)
    entries[index] = (entry_id, render_entry(entry_id, fields))


def apply_codex_entry(
    output_dir: Path,
    entries: list[tuple[str | None, list[str]]],
    entry: dict[str, object],
) -> None:
    entry_id = required_string(entry, "entry_id")
    repair_plan = load_repair_promotion_capture_plan(entry)
    story_plan = load_story_promotion_capture_plan(entry)
    ui_patch_plan = load_ui_patch_capture_plan(entry)
    plan_count = sum(
        1 for plan in [repair_plan, story_plan, ui_patch_plan] if plan is not None
    )
    if plan_count > 1:
        raise SystemExit(
            "batch entry cannot use repair, story, and UI patch promotion plans together"
        )
    append_entry = plan_count == 1
    source_entry_id = (
        plan_string(repair_plan, "source_entry_id")
        if repair_plan is not None
        else plan_string(ui_patch_plan, "source_entry_id")
        if ui_patch_plan is not None
        else required_string(entry, "source_entry_id")
        if story_plan is not None
        else entry_id
    )
    if append_entry and any(candidate_id == entry_id for candidate_id, _lines in entries):
        raise SystemExit(f"proposed promotion entry {entry_id} already exists")
    index = entry_index(entries, source_entry_id)
    _entry_id, entry_lines = entries[index]
    fields = fields_from_entry(entry_lines)
    prompt_file = optional_string(entry, "prompt_file") or fields["prompt-file"]
    if story_plan is not None and optional_string(entry, "prompt_file") is None:
        prompt_file = "docs/ail/prompts/spec-draft.system.md"
    if ui_patch_plan is not None and optional_string(entry, "prompt_file") is None:
        prompt_file = "docs/ail/prompts/flow-patch.system.md"
    fields["prompt-file"] = prompt_file
    system_prompt = (ROOT / prompt_file).read_text()

    request_file = f"requests/{entry_id}.json"
    response_file = f"responses/{entry_id}.json"
    write_json_file(
        output_dir / request_file, read_json_file(required_string(entry, "request_json_file"))
    )
    write_json_file(
        output_dir / response_file, read_json_file(required_string(entry, "response_json_file"))
    )

    fields.update(
        {
            "semantic-task": required_string(entry, "semantic_task"),
            "prompt-fingerprint": fnv64(system_prompt),
            "executor-family": "codex-skill-agent",
            "executor-label": required_string(entry, "executor_label"),
            "capture-origin": "live-codex",
            "request-file": request_file,
            "response-file": response_file,
        }
    )
    checker_result = optional_string(entry, "checker_result")
    if checker_result is not None:
        if checker_result not in {"accepted", "rejected"}:
            raise SystemExit(f"batch entry {entry_id} has invalid checker_result")
        fields["checker-result"] = checker_result
    fields.pop("endpoint-label", None)
    if fields.get("checker-result") == "accepted":
        fields.pop("expected-diagnostic", None)
        fields.pop("failure-taxonomy", None)
    if append_entry:
        if story_plan is not None:
            story_artifact_dir = Path(plan_string(story_plan, "story_artifact_dir"))
            story_fields = story_fields_from_file(story_artifact_dir / "story.normalized.md")
            semantic_anchors = story_fields.get("semantic-anchors", "")
            for key in ["user-story-id", "user-story", "acceptance-criteria"]:
                fields[key] = story_fields[key]
            fields["story-file"] = f"stories/{entry_id}.md"
            fields["story-journey"] = optional_string(entry, "story_journey") or story_fields.get(
                "story-journey", "story-to-spec"
            )
            fields["story-roundtrip"] = optional_string(
                entry, "story_roundtrip"
            ) or story_fields.get("story-roundtrip", "semantic-similar")
            fields["story-evidence"] = optional_string(entry, "story_evidence") or "vm-trace"
            fields["program-domain"] = optional_string(entry, "program_domain") or story_fields.get(
                "program-domain", fields["program-domain"]
            )
            fields["module-count"] = optional_string(entry, "module_count") or story_fields.get(
                "module-count", fields["module-count"]
            )
            fields["spec-count"] = optional_string(entry, "spec_count") or story_fields.get(
                "spec-count", fields["spec-count"]
            )
            fields["story-count"] = optional_string(entry, "story_count") or story_fields.get(
                "story-count", fields["story-count"]
            )
            fields["interacts-with"] = optional_string(entry, "interacts_with") or story_fields.get(
                "interacts-with", fields["interacts-with"]
            )
            fields["surface-tags"] = optional_string(entry, "surface_tags") or "user-story-mode"
            fields["capability-under-test"] = (
                optional_string(entry, "capability_under_test") or "user-story-mode-promotion"
            )
            fields["use-case"] = optional_string(entry, "use_case") or (
                "Human-approved User Story mode promotion for a reviewed story artifact "
                "bundle that already produced requirements, spec, Core, bytecode, and trace evidence."
            )
            fields["v0.3-signal"] = optional_string(entry, "v03_signal") or (
                "User Story mode needs replayable promotion evidence that preserves the "
                "story artifact bundle while appending an accepted corpus candidate."
            )
            fields["story-artifacts"] = f"story-artifacts/{entry_id}"
            artifact_output = output_dir / fields["story-artifacts"]
            if artifact_output.exists():
                shutil.rmtree(artifact_output)
            shutil.copytree(story_artifact_dir, artifact_output)
        else:
            source_story_file = output_dir / fields["story-file"]
            source_story_fields = story_fields_from_file(source_story_file)
            semantic_anchors = source_story_fields.get("semantic-anchors", "")
        if repair_plan is not None and fields.get("program-domain") == "diagnostic":
            fields["program-domain"] = optional_string(entry, "program_domain") or "application"
            fields["capability-under-test"] = (
                optional_string(entry, "capability_under_test") or "repair-promotion-import"
            )
            fields["use-case"] = optional_string(entry, "use_case") or (
                f"Human-approved repair promotion for rejected entry {source_entry_id} "
                "that preserves diagnostic evidence while adding a replayable accepted spec."
            )
            fields["v0.3-signal"] = optional_string(entry, "v03_signal") or (
                "Repair promotion imports need batch capture evidence that creates accepted "
                "entries while preserving rejected diagnostics for learning."
            )
        if repair_plan is not None:
            fields["story-file"] = f"stories/{entry_id}.md"
            fields["story-journey"] = optional_string(entry, "story_journey") or "story-to-spec"
            fields["story-roundtrip"] = (
                optional_string(entry, "story_roundtrip") or "semantic-similar"
            )
            fields["story-evidence"] = optional_string(entry, "story_evidence") or "vm-trace"
        if ui_patch_plan is not None:
            fields["story-file"] = f"stories/{entry_id}.md"
            fields["story-journey"] = optional_string(entry, "story_journey") or "story-amendment"
            fields["story-roundtrip"] = (
                optional_string(entry, "story_roundtrip") or "semantic-similar"
            )
            fields["story-evidence"] = optional_string(entry, "story_evidence") or "vm-trace"
            fields["surface-tags"] = optional_string(entry, "surface_tags") or "ui,flow-patch"
            fields["capability-under-test"] = (
                optional_string(entry, "capability_under_test") or "ui-patch-import"
            )
            fields["use-case"] = optional_string(entry, "use_case") or (
                f"Human-approved UI patch import for deterministic review plan {source_entry_id} "
                "that applies an ail-flow-edit and replays the patched spec through Core, "
                "bytecode, target contract, and trace evidence."
            )
            fields["v0.3-signal"] = optional_string(entry, "v03_signal") or (
                "UI authoring needs imported visual patch plans to carry replay evidence "
                "after human-approved ail-flow-edit changes are applied."
            )
        story_path = output_dir / fields["story-file"]
        story_path.parent.mkdir(parents=True, exist_ok=True)
        story_path.write_text(render_story_file(fields, semantic_anchors))
    refresh_distinctness_claim(fields)
    if append_entry:
        entries.append((entry_id, render_entry(entry_id, fields)))
    else:
        entries[index] = (entry_id, render_entry(entry_id, fields))


def write_examples(output_dir: Path, entries: list[tuple[str | None, list[str]]]) -> None:
    output_lines: list[str] = []
    for _entry_id, lines in entries:
        output_lines.extend(lines)
    (output_dir / "examples.md").write_text("\n".join(output_lines).rstrip() + "\n")


def main() -> int:
    args = parse_args()
    base_corpus = (ROOT / args.base_corpus).resolve()
    output_dir = Path(args.output_dir).resolve()
    if output_dir.exists():
        shutil.rmtree(output_dir)
    shutil.copytree(base_corpus, output_dir)

    plan = read_json_file(args.plan_json)
    if not isinstance(plan, dict) or not isinstance(plan.get("entries"), list):
        raise SystemExit("batch plan must contain an entries array")
    examples_path = output_dir / "examples.md"
    entries = read_entries(examples_path.read_text())
    for entry in plan["entries"]:
        if not isinstance(entry, dict):
            raise SystemExit("batch plan entries must be objects")
        executor_family = required_string(entry, "executor_family")
        if executor_family == "llm-http":
            apply_llm_entry(output_dir, entries, entry)
        elif executor_family == "codex-skill-agent":
            apply_codex_entry(output_dir, entries, entry)
        else:
            raise SystemExit(f"batch entry has unsupported executor_family {executor_family}")

    write_examples(output_dir, entries)
    print(f"captured {len(plan['entries'])} entries into {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
