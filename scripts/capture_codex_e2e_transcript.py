#!/usr/bin/env python3
"""Promote one recorded Codex/skill-agent transcript into an AIL e2e corpus copy."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

from capture_e2e_transcripts import fields_from_entry, fnv64, read_entries, render_entry


ROOT = Path(__file__).resolve().parents[1]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Copy a recorded Codex transcript into an offline ail-e2e-corpus directory."
    )
    parser.add_argument("--base-corpus", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--entry-id", required=True)
    parser.add_argument("--executor-label", required=True)
    parser.add_argument("--semantic-task", required=True)
    parser.add_argument("--request-json-file", required=True)
    parser.add_argument("--response-json-file", required=True)
    parser.add_argument("--checker-result", choices=["accepted", "rejected"])
    return parser.parse_args()


def read_json_file(path: str) -> object:
    return json.loads(Path(path).read_text())


def write_json_file(path: Path, payload: object) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def main() -> int:
    args = parse_args()
    base_corpus = (ROOT / args.base_corpus).resolve()
    output_dir = Path(args.output_dir).resolve()
    if output_dir.exists():
        shutil.rmtree(output_dir)
    shutil.copytree(base_corpus, output_dir)

    examples_path = output_dir / "examples.md"
    entries = read_entries(examples_path.read_text())
    replacement_index = next(
        (index for index, (entry_id, _lines) in enumerate(entries) if entry_id == args.entry_id),
        None,
    )
    if replacement_index is None:
        raise SystemExit(f"entry {args.entry_id} not found in {examples_path}")

    _entry_id, entry_lines = entries[replacement_index]
    fields = fields_from_entry(entry_lines)
    prompt_file = fields["prompt-file"]
    system_prompt = (ROOT / prompt_file).read_text()

    request_file = f"requests/{args.entry_id}.json"
    response_file = f"responses/{args.entry_id}.json"
    write_json_file(output_dir / request_file, read_json_file(args.request_json_file))
    write_json_file(output_dir / response_file, read_json_file(args.response_json_file))

    fields.update(
        {
            "semantic-task": args.semantic_task,
            "prompt-fingerprint": fnv64(system_prompt),
            "executor-family": "codex-skill-agent",
            "executor-label": args.executor_label,
            "capture-origin": "live-codex",
            "request-file": request_file,
            "response-file": response_file,
        }
    )
    if args.checker_result is not None:
        fields["checker-result"] = args.checker_result
    fields.pop("endpoint-label", None)
    if fields.get("checker-result") == "accepted":
        fields.pop("expected-diagnostic", None)
        fields.pop("failure-taxonomy", None)

    entries[replacement_index] = (args.entry_id, render_entry(args.entry_id, fields))
    output_lines: list[str] = []
    for _entry_id, lines in entries:
        output_lines.extend(lines)
    examples_path.write_text("\n".join(output_lines).rstrip() + "\n")
    print(f"captured {args.entry_id} from recorded Codex transcript into {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
