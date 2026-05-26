#!/usr/bin/env python3
"""Capture one live LLM transcript into a replayable AIL examples copy."""

from __future__ import annotations

import argparse
import json
import shutil
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capture a live LLM response into an offline AIL examples directory."
    )
    parser.add_argument("--base-corpus", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--entry-id", required=True)
    parser.add_argument("--endpoint", required=True)
    parser.add_argument("--endpoint-label", required=True)
    parser.add_argument("--executor-label", required=True)
    parser.add_argument("--semantic-task", required=True)
    parser.add_argument("--prompt")
    parser.add_argument("--prompt-file")
    parser.add_argument("--input-json-file")
    parser.add_argument("--n-predict", type=int, default=2048)
    args = parser.parse_args()
    if (args.prompt is None) == (args.prompt_file is None):
        parser.error("exactly one of --prompt or --prompt-file is required")
    return args


def read_entries(text: str) -> list[tuple[str | None, list[str]]]:
    entries: list[tuple[str | None, list[str]]] = []
    current_id: str | None = None
    current_lines: list[str] = []
    for line in text.splitlines():
        if line.startswith("## Example: ") or line.startswith("## End-To-End Example: "):
            if current_id is not None:
                entries.append((current_id, current_lines))
            current_id = line.removeprefix("## Example: ")
            current_id = current_id.removeprefix("## End-To-End Example: ").strip()
            current_lines = [line]
        elif current_id is None:
            entries.append((None, [line]))
        else:
            current_lines.append(line)
    if current_id is not None:
        entries.append((current_id, current_lines))
    return entries


def fields_from_entry(lines: list[str]) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in lines[1:]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        key = key.strip()
        if key and all(ch.islower() or ch.isdigit() or ch in {"-", "."} for ch in key):
            fields[key] = value.strip()
    return fields


def render_entry(entry_id: str, fields: dict[str, str]) -> list[str]:
    lines = [f"## Example: {entry_id}"]
    for key, value in fields.items():
        lines.append(f"{key}: {value}")
    lines.append("")
    return lines


def refresh_distinctness_claim(fields: dict[str, str]) -> None:
    semantic_task = fields["semantic-task"]
    capability = fields.get("capability-under-test", "declared capability")
    distinctness_claim = fields.get("distinctness-claim", "")
    if semantic_task in distinctness_claim and capability in distinctness_claim:
        return
    fields["distinctness-claim"] = (
        f"{semantic_task} validates {capability} with stored transcript replay "
        "and promoted executor evidence."
    )


def completion_body(endpoint: str, prompt: str, n_predict: int) -> dict[str, object]:
    if endpoint.rstrip("/").endswith("/chat/completions"):
        return {
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": n_predict,
            "temperature": 0.0,
            "chat_template_kwargs": {"enable_thinking": False},
        }
    return {"prompt": prompt, "n_predict": n_predict, "temperature": 0.0}


def capture_completion(endpoint: str, body: dict[str, object]) -> dict[str, object]:
    encoded = json.dumps(body, sort_keys=True).encode()
    request = urllib.request.Request(
        endpoint,
        data=encoded,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        response_text = response.read().decode()
    return json.loads(response_text)


def render_prompt(system_prompt: str, task_prompt: str, input_json_file: str | None) -> str:
    if input_json_file is None:
        return f"{system_prompt.rstrip()}\n\nUSER REQUEST:\n{task_prompt}\n"
    input_path = Path(input_json_file)
    input_payload = json.loads(input_path.read_text())
    input_text = json.dumps(input_payload, indent=2, sort_keys=True)
    return (
        f"{system_prompt.rstrip()}\n\n"
        f"TASK:\n{task_prompt}\n\n"
        f"INPUT JSON:\n{input_text}\n"
    )


def read_task_prompt(args: argparse.Namespace) -> str:
    if args.prompt_file is not None:
        return Path(args.prompt_file).read_text().strip()
    return args.prompt


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
    prompt = render_prompt(system_prompt, read_task_prompt(args), args.input_json_file)
    body = completion_body(args.endpoint, prompt, args.n_predict)
    response_json = capture_completion(args.endpoint, body)

    request_file = f"requests/{args.entry_id}.json"
    response_file = f"responses/{args.entry_id}.json"
    request_transcript = {
        "endpoint": args.endpoint,
        "method": "POST",
        "body": body,
    }
    (output_dir / request_file).write_text(
        json.dumps(request_transcript, indent=2, sort_keys=True) + "\n"
    )
    (output_dir / response_file).write_text(
        json.dumps(response_json, indent=2, sort_keys=True) + "\n"
    )

    fields.update(
        {
            "semantic-task": args.semantic_task,
            "prompt-fingerprint": fnv64(system_prompt),
            "executor-family": "llm-http",
            "executor-label": args.executor_label,
            "capture-origin": "live-llm",
            "endpoint-label": args.endpoint_label,
            "request-file": request_file,
            "response-file": response_file,
            "checker-result": "accepted",
        }
    )
    refresh_distinctness_claim(fields)
    entries[replacement_index] = (args.entry_id, render_entry(args.entry_id, fields))
    output_lines: list[str] = []
    for _entry_id, lines in entries:
        output_lines.extend(lines)
    examples_path.write_text("\n".join(output_lines).rstrip() + "\n")
    print(f"captured {args.entry_id} from {args.endpoint} into {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
