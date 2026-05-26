#!/usr/bin/env python3
"""Run the AIL v0.3 prompt-pack live LLM harness.

This script is intentionally opt-in. It contacts a llama.cpp-compatible server
and records one probe per required AIL system prompt so prompt interaction
evidence can be reviewed before any output is promoted into ./examples.
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.parse
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SERVER = "http://inteligentia-pro-1:8080/"
DEFAULT_ENDPOINT = "http://inteligentia-pro-1:8080/v1/chat/completions"
DEFAULT_PROMPT_DIR = "docs/ail/prompts"
DEFAULT_ARTIFACT_DIR = "/tmp/ail-v03-prompt-llm"
REQUIRED_PROMPTS = (
    "interview.system.md",
    "requirements.system.md",
    "spec-draft.system.md",
    "core-draft.system.md",
    "repair.system.md",
    "diagnostic-repair.system.md",
    "core-to-spec.system.md",
    "core-to-summary.system.md",
    "flow-patch.system.md",
    "trace-debug.system.md",
    "interop.system.md",
)
DEFAULT_PROBE = """AIL prompt-pack live probe.

Return a compact prompt-pack envelope or blocking questions for this prompt
surface. Keep the response short. Do not claim the artifact is checked,
compiled, deployed, or trusted.
"""


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def endpoint_join(endpoint: str, suffix: str) -> str:
    return endpoint.rstrip("/") + "/" + suffix.lstrip("/")


def models_url_for_endpoint(endpoint: str) -> str:
    parsed = urllib.parse.urlsplit(endpoint)
    if not parsed.scheme or not parsed.netloc:
        raise SystemExit(f"invalid endpoint URL: {endpoint}")
    base_path = parsed.path
    if "/v1/" in base_path:
        base_path = base_path.split("/v1/", 1)[0]
    models_path = endpoint_join(base_path or "/", "/v1/models")
    return urllib.parse.urlunsplit(
        (parsed.scheme, parsed.netloc, models_path, "", "")
    )


def prompt_paths(prompt_dir: str) -> list[Path]:
    root = (ROOT / prompt_dir).resolve()
    paths = [root / name for name in REQUIRED_PROMPTS]
    missing = [str(path.relative_to(ROOT)) for path in paths if not path.exists()]
    if missing:
        raise SystemExit("missing required prompt files: " + ", ".join(missing))
    return paths


def relative(path: Path) -> str:
    return str(path.resolve().relative_to(ROOT))


def completion_body(
    endpoint: str,
    system_prompt: str,
    user_probe: str,
    max_tokens: int,
    model: str | None,
) -> dict[str, object]:
    if endpoint.rstrip("/").endswith("/chat/completions"):
        body: dict[str, object] = {
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_probe},
            ],
            "max_tokens": max_tokens,
            "temperature": 0.0,
            "stream": False,
            "chat_template_kwargs": {"enable_thinking": False},
        }
        if model:
            body["model"] = model
        return body
    prompt = f"{system_prompt.rstrip()}\n\nUSER PROBE:\n{user_probe.strip()}\n"
    body = {
        "prompt": prompt,
        "n_predict": max_tokens,
        "temperature": 0.0,
        "stream": False,
    }
    if model:
        body["model"] = model
    return body


def request_json(url: str, body: dict[str, object]) -> dict[str, object]:
    encoded = json.dumps(body, sort_keys=True).encode()
    request = urllib.request.Request(
        url,
        data=encoded,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=180) as response:
        response_text = response.read().decode("utf-8", errors="replace")
    return json.loads(response_text)


def get_json(url: str) -> dict[str, object]:
    with urllib.request.urlopen(url, timeout=30) as response:
        response_text = response.read().decode("utf-8", errors="replace")
    return json.loads(response_text)


def extract_content(response: dict[str, object]) -> str:
    content = response.get("content")
    if isinstance(content, str):
        return content.strip()
    choices = response.get("choices")
    if isinstance(choices, list) and choices:
        first = choices[0]
        if isinstance(first, dict):
            message = first.get("message")
            if isinstance(message, dict) and isinstance(message.get("content"), str):
                return str(message["content"]).strip()
            if isinstance(first.get("text"), str):
                return str(first["text"]).strip()
    return ""


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run one live prompt-pack probe for every required AIL system prompt. "
            "Use --dry-run to list probes without network access."
        ),
        epilog=f"Default live server: {DEFAULT_SERVER}",
    )
    parser.add_argument(
        "--endpoint",
        default=DEFAULT_ENDPOINT,
        help=f"LLM endpoint (default: {DEFAULT_ENDPOINT})",
    )
    parser.add_argument(
        "--prompt-dir",
        default=DEFAULT_PROMPT_DIR,
        help=f"Prompt directory (default: {DEFAULT_PROMPT_DIR})",
    )
    parser.add_argument(
        "--artifact-dir",
        default=DEFAULT_ARTIFACT_DIR,
        help=f"Artifact directory (default: {DEFAULT_ARTIFACT_DIR})",
    )
    parser.add_argument("--model", help="Optional model id for OpenAI-compatible servers")
    parser.add_argument("--max-tokens", type=int, default=512)
    parser.add_argument("--probe", default=DEFAULT_PROBE)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print prompt probes without contacting the LLM endpoint",
    )
    parser.add_argument(
        "--skip-model-check",
        action="store_true",
        help="Skip the /v1/models check before running prompt probes",
    )
    return parser.parse_args(argv)


def print_dry_run(args: argparse.Namespace, paths: list[Path]) -> None:
    print("AIL-Prompt-LLM-Harness:")
    print(f"model-check curl -sS {models_url_for_endpoint(args.endpoint)}")
    print(f"endpoint {args.endpoint}")
    print(f"prompt-dir {args.prompt_dir}")
    print(f"artifact-dir {args.artifact_dir}")
    for path in paths:
        print(f"prompt {relative(path)}")


def run_live(args: argparse.Namespace, paths: list[Path]) -> int:
    artifact_root = Path(args.artifact_dir)
    artifact_root.mkdir(parents=True, exist_ok=True)
    report_lines = [
        "AIL-Prompt-LLM-Harness:",
        f"endpoint {args.endpoint}",
        f"models-url {models_url_for_endpoint(args.endpoint)}",
        f"prompt-count {len(paths)}",
    ]
    manifest_lines = ["AIL-Prompt-LLM-Harness-Manifest:"]
    if not args.skip_model_check:
        models = get_json(models_url_for_endpoint(args.endpoint))
        models_text = json.dumps(models, indent=2, sort_keys=True) + "\n"
        write_text(artifact_root / "models.json", models_text)
        write_text(artifact_root / "models.fingerprint.txt", fnv64(models_text) + "\n")
        manifest_lines.append("artifact models models.json models.fingerprint.txt")

    for path in paths:
        prompt_text = path.read_text()
        rel = relative(path)
        stem = path.name.removesuffix(".system.md")
        body = completion_body(args.endpoint, prompt_text, args.probe, args.max_tokens, args.model)
        response = request_json(args.endpoint, body)
        request_text = json.dumps(
            {
                "endpoint": args.endpoint,
                "method": "POST",
                "prompt_file": rel,
                "prompt_fingerprint": fnv64(prompt_text),
                "body": body,
            },
            indent=2,
            sort_keys=True,
        ) + "\n"
        response_text = json.dumps(response, indent=2, sort_keys=True) + "\n"
        content_text = extract_content(response) + "\n"
        request_path = artifact_root / "requests" / f"{stem}.json"
        response_path = artifact_root / "responses" / f"{stem}.json"
        content_path = artifact_root / "content" / f"{stem}.txt"
        request_fingerprint_path = request_path.with_suffix(".fingerprint.txt")
        response_fingerprint_path = response_path.with_suffix(".fingerprint.txt")
        content_fingerprint_path = content_path.with_suffix(".fingerprint.txt")
        write_text(request_path, request_text)
        write_text(response_path, response_text)
        write_text(content_path, content_text)
        write_text(request_fingerprint_path, fnv64(request_text) + "\n")
        write_text(response_fingerprint_path, fnv64(response_text) + "\n")
        write_text(content_fingerprint_path, fnv64(content_text) + "\n")
        report_lines.append(
            "prompt "
            f"{rel} prompt-fingerprint {fnv64(prompt_text)} "
            f"response-fingerprint {fnv64(response_text)} "
            f"content-bytes {len(content_text.encode())}"
        )
        manifest_lines.append(
            f"artifact {rel} "
            f"request requests/{stem}.json requests/{stem}.fingerprint.txt "
            f"response responses/{stem}.json responses/{stem}.fingerprint.txt "
            f"content content/{stem}.txt content/{stem}.fingerprint.txt"
        )

    report_text = "\n".join(report_lines) + "\n"
    manifest_text = "\n".join(manifest_lines) + "\n"
    write_text(artifact_root / "prompt-llm-harness-report.txt", report_text)
    write_text(
        artifact_root / "prompt-llm-harness-report.fingerprint.txt",
        fnv64(report_text) + "\n",
    )
    write_text(artifact_root / "manifest.v03-prompt-llm.txt", manifest_text)
    write_text(
        artifact_root / "manifest.fingerprint.txt",
        fnv64(manifest_text) + "\n",
    )
    print(report_text, end="")
    print(f"artifacts {artifact_root}")
    return 0


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    paths = prompt_paths(args.prompt_dir)
    if args.dry_run:
        print_dry_run(args, paths)
        return 0
    return run_live(args, paths)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
