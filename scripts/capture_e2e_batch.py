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
    append_entry = repair_plan is not None
    source_entry_id = (
        plan_string(repair_plan, "source_entry_id")
        if repair_plan is not None
        else entry_id
    )
    if append_entry and any(candidate_id == entry_id for candidate_id, _lines in entries):
        raise SystemExit(f"proposed repair promotion entry {entry_id} already exists")
    index = entry_index(entries, source_entry_id)
    _entry_id, entry_lines = entries[index]
    fields = fields_from_entry(entry_lines)
    prompt_file = fields["prompt-file"]
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
        source_story_file = output_dir / fields["story-file"]
        source_story_fields = story_fields_from_file(source_story_file)
        semantic_anchors = source_story_fields.get("semantic-anchors", "")
        if fields.get("program-domain") == "diagnostic":
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
        fields["story-file"] = f"stories/{entry_id}.md"
        fields["story-journey"] = optional_string(entry, "story_journey") or "story-to-spec"
        fields["story-roundtrip"] = (
            optional_string(entry, "story_roundtrip") or "semantic-similar"
        )
        fields["story-evidence"] = optional_string(entry, "story_evidence") or "vm-trace"
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
