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
    parser.add_argument("--plan-json", required=True)
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
    entries[index] = (entry_id, render_entry(entry_id, fields))


def apply_codex_entry(
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

    request_file = f"requests/{entry_id}.json"
    response_file = f"responses/{entry_id}.json"
    write_json_file(output_dir / request_file, read_json_file(required_string(entry, "request_json_file")))
    write_json_file(output_dir / response_file, read_json_file(required_string(entry, "response_json_file")))

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
