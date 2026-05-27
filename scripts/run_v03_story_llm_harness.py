#!/usr/bin/env python3
"""Run the AIL v0.3 story-mode live LLM harness.

This script is intentionally not part of the default test suite. It contacts a
llama.cpp-compatible server and then runs `ail-story` through Cargo.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import urllib.parse
import urllib.request
from pathlib import Path


DEFAULT_SERVER = "http://inteligentia-pro-1:8080/"
DEFAULT_ENDPOINT = "http://inteligentia-pro-1:8080/v1/chat/completions"
DEFAULT_PACKAGE = "examples/support_ticket.ail"
DEFAULT_STORY_FILE = "examples/stories/example-30.md"
DEFAULT_ARTIFACT_DIR = "/tmp/ail-v03-story-llm"


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


def build_ail_story_command(args: argparse.Namespace) -> list[str]:
    command = [
        "cargo",
        "run",
        "--",
        "ail-story",
        args.package,
        "--story-file",
        args.story_file,
        "--artifact-dir",
        args.artifact_dir,
        "--llm-endpoint",
        args.endpoint,
    ]
    if args.agent:
        command.extend(["--agent", args.agent])
    if args.target or args.action or args.out:
        if not (args.target and args.action and args.out):
            raise SystemExit("--target, --action, and --out must be supplied together")
        command.extend(["--target", args.target, "--action", args.action, "--out", args.out])
    return command


def check_models(endpoint: str) -> None:
    models_url = models_url_for_endpoint(endpoint)
    with urllib.request.urlopen(models_url, timeout=10) as response:
        body = response.read().decode("utf-8", errors="replace")
    print(f"models endpoint: {models_url}")
    print(body)


def read_required_text(path: Path, errors: list[str]) -> str:
    if not path.exists():
        errors.append(f"missing file {path}")
        return ""
    return path.read_text()


def check_fingerprint(
    path: Path, errors: list[str], fingerprint_path: Path | None = None
) -> bool:
    text = read_required_text(path, errors)
    fingerprint_path = fingerprint_path or path.with_suffix(".fingerprint.txt")
    expected = read_required_text(fingerprint_path, errors).strip()
    actual = fnv64(text)
    if expected != actual:
        errors.append(
            f"fingerprint mismatch {path}: expected {expected or '<missing>'} got {actual}"
        )
        return False
    return True


def parse_story_report(report_text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in report_text.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            values[key.strip()] = value.strip()
    return values


def prompt_envelope_json(content: str) -> tuple[dict[str, object] | None, str]:
    candidate = content.strip()
    if not candidate:
        return None, "prompt response content is empty"
    if candidate.startswith("```"):
        lines = candidate.splitlines()
        if lines:
            lines = lines[1:]
        if lines and lines[-1].strip().startswith("```"):
            lines = lines[:-1]
        candidate = "\n".join(lines).strip()
    try:
        value = json.loads(candidate)
    except json.JSONDecodeError:
        start = candidate.find("{")
        end = candidate.rfind("}")
        if start == -1 or end <= start:
            return None, "prompt response must be a JSON prompt-pack envelope"
        try:
            value = json.loads(candidate[start : end + 1])
        except json.JSONDecodeError as error:
            return None, f"prompt response contains invalid JSON envelope: {error}"
    if not isinstance(value, dict):
        return None, "prompt response envelope must be a JSON object"
    return value, ""


def classify_prompt_content(content: str, expected_kind: str) -> tuple[str, str]:
    envelope, error = prompt_envelope_json(content)
    if envelope is None:
        return "empty" if not content.strip() else "invalid", error
    artifact_kind = envelope.get("artifact_kind")
    if artifact_kind != expected_kind:
        return (
            "invalid",
            f"prompt envelope artifact_kind must be {expected_kind}, got {artifact_kind}",
        )
    artifact_text = envelope.get("artifact_text", "")
    if not isinstance(artifact_text, str):
        return "invalid", "prompt envelope artifact_text must be a string"
    questions_value = envelope.get("questions", [])
    if not isinstance(questions_value, list) or not all(
        isinstance(question, str) for question in questions_value
    ):
        return "invalid", "prompt envelope questions must be a list of strings"
    questions = [question.strip() for question in questions_value if question.strip()]
    has_artifact = bool(artifact_text.strip())
    has_questions = bool(questions)
    if has_artifact and has_questions:
        return "invalid", "prompt envelope cannot contain both artifact_text and questions"
    if not has_artifact and not has_questions:
        return "invalid", "prompt envelope must contain artifact_text or questions"
    checker_handoff = envelope.get("checker_handoff")
    if not isinstance(checker_handoff, dict):
        return "invalid", "prompt envelope checker_handoff must be an object"
    if checker_handoff.get("must_check") is not True:
        return "invalid", "prompt envelope checker_handoff.must_check must be true"
    if has_questions:
        return "prompt-envelope-questions", ""
    return "prompt-envelope-artifact", ""


def manifest_entries(manifest_text: str) -> list[tuple[str, str, str]]:
    entries: list[tuple[str, str, str]] = []
    for line in manifest_text.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[-1].startswith("fnv64:"):
            entries.append((parts[0], parts[-2], parts[-1]))
    return entries


def check_manifest_entries(artifact_root: Path, manifest_text: str, errors: list[str]) -> int:
    checked = 0
    for kind, file_name, expected in manifest_entries(manifest_text):
        text = read_required_text(artifact_root / file_name, errors)
        actual = fnv64(text)
        if actual != expected:
            errors.append(
                f"manifest fingerprint mismatch {kind} {file_name}: expected {expected} got {actual}"
            )
        else:
            checked += 1
    return checked


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def review_artifacts(artifact_dir: str) -> int:
    artifact_root = Path(artifact_dir)
    errors: list[str] = []
    fingerprint_checks = 0
    manifest_text = read_required_text(artifact_root / "manifest.ail-story.txt", errors)
    report_text = read_required_text(artifact_root / "story-mode-report.txt", errors)
    normalized_story = read_required_text(artifact_root / "story.normalized.md", errors)
    agent_trace = read_required_text(artifact_root / "agent-trace.txt", errors)
    transcript_checks = 0
    transcript_valid_count = 0
    transcript_invalid_count = 0

    for path, fingerprint_path in [
        (artifact_root / "story.source.md", artifact_root / "story.source.fingerprint.txt"),
        (
            artifact_root / "story.normalized.md",
            artifact_root / "story.normalized.fingerprint.txt",
        ),
        (
            artifact_root / "story-mode-report.txt",
            artifact_root / "story-mode-report.fingerprint.txt",
        ),
        (
            artifact_root / "requirements.ail-requirements.md",
            artifact_root / "requirements.fingerprint.txt",
        ),
        (
            artifact_root / "accepted.ail-spec.md",
            artifact_root / "accepted.ail-spec.fingerprint.txt",
        ),
        (
            artifact_root / "checked.ail-core.txt",
            artifact_root / "checked.ail-core.fingerprint.txt",
        ),
        (
            artifact_root / "review.ail-flow.json",
            artifact_root / "review.ail-flow.fingerprint.txt",
        ),
        (artifact_root / "artifact.ailbc.json", artifact_root / "artifact.fingerprint.txt"),
        (
            artifact_root / "manifest.ail-story.txt",
            artifact_root / "manifest.ail-story.fingerprint.txt",
        ),
    ]:
        if check_fingerprint(path, errors, fingerprint_path):
            fingerprint_checks += 1
    if (artifact_root / "manifest.ail-build.txt").exists():
        if check_fingerprint(
            artifact_root / "manifest.ail-build.txt",
            errors,
            artifact_root / "manifest.fingerprint.txt",
        ):
            fingerprint_checks += 1

    for stem, expected_kind in [
        ("requirements", "AIL-Requirements"),
        ("spec", "AIL-Spec Canonical"),
    ]:
        for suffix in ["request.json", "response.json", "content.txt"]:
            path = artifact_root / "llm" / f"{stem}.{suffix}"
            if check_fingerprint(path, errors):
                transcript_checks += 1
            manifest_label = f"llm-{stem}-{suffix.split('.', 1)[0]}"
            if manifest_label not in manifest_text:
                errors.append(f"manifest missing {manifest_label}")
        request_text = read_required_text(
            artifact_root / "llm" / f"{stem}.request.json", errors
        )
        response_text = read_required_text(
            artifact_root / "llm" / f"{stem}.response.json", errors
        )
        content = read_required_text(
            artifact_root / "llm" / f"{stem}.content.txt", errors
        )
        for label, text in [
            (f"{stem}.request.json", request_text),
            (f"{stem}.response.json", response_text),
        ]:
            if text:
                try:
                    json.loads(text)
                except json.JSONDecodeError as error:
                    errors.append(f"invalid json llm/{label}: {error}")
        content_kind, content_error = classify_prompt_content(content, expected_kind)
        if content_kind in {"prompt-envelope-artifact", "prompt-envelope-questions"}:
            transcript_valid_count += 1
        else:
            transcript_invalid_count += 1
            errors.append(f"invalid story prompt envelope {stem}: {content_error}")

    manifest_match_count = check_manifest_entries(artifact_root, manifest_text, errors)
    if "AIL-Story-Manifest:" not in manifest_text:
        errors.append("manifest missing AIL-Story-Manifest header")
    if "entrypoint ail-story" not in manifest_text:
        errors.append("manifest missing ail-story entrypoint")
    for required_manifest_entry in [
        "agent agent.ailbc.json",
        "agent-trace agent-trace.txt",
    ]:
        if required_manifest_entry not in manifest_text:
            errors.append(f"manifest missing {required_manifest_entry}")

    report_values = parse_story_report(report_text)
    story_id = report_values.get("user-story-id", "")
    anchor_count = report_values.get("semantic-anchor-count", "")
    if not story_id:
        errors.append("story report missing user-story-id")
    if not anchor_count:
        errors.append("story report missing semantic-anchor-count")
    for required in [
        "user-story:",
        "acceptance-criteria:",
        "semantic-anchors:",
        "story-journey: story-to-spec",
        "story-roundtrip: semantic-similar",
    ]:
        if required not in normalized_story:
            errors.append(f"normalized story missing {required}")
    if "entrypoint=ail-story" not in agent_trace:
        errors.append("agent trace missing entrypoint=ail-story")
    for action in [
        "action CaptureRequirements started",
        "action PrepareSpecDraft started",
        "action AcceptSpecDraft started",
        "action CompileApplication started",
        "action VerifyBytecodeArtifact started",
    ]:
        if action not in agent_trace:
            errors.append(f"agent trace missing {action}")
    for json_path in ["artifact.ailbc.json", "review.ail-flow.json"]:
        text = read_required_text(artifact_root / json_path, errors)
        if text:
            try:
                json.loads(text)
            except json.JSONDecodeError as error:
                errors.append(f"invalid json {json_path}: {error}")

    review_lines = [
        "AIL-Story-LLM-Harness-Review:",
        f"artifact-dir {artifact_root}",
        f"story-id {story_id or '<missing>'}",
        f"semantic-anchor-count {anchor_count or '<missing>'}",
        f"manifest-entry-check-count {manifest_match_count}",
        f"fingerprint-check-count {fingerprint_checks}",
        f"story-llm-transcript-check-count {transcript_checks}",
        f"story-prompt-envelope-valid-count {transcript_valid_count}",
        f"story-prompt-envelope-invalid-count {transcript_invalid_count}",
        f"agent-trace {'present' if agent_trace else 'missing'}",
    ]
    if errors:
        review_lines.append("review-result rejected")
        for error in errors:
            review_lines.append(f"error {error}")
    else:
        review_lines.append("review-result accepted")
    review_text = "\n".join(review_lines) + "\n"
    try:
        write_text(artifact_root / "story-llm-harness-report.txt", review_text)
        write_text(
            artifact_root / "story-llm-harness-report.fingerprint.txt",
            fnv64(review_text) + "\n",
        )
    except OSError as error:
        print(review_text, end="")
        print(f"error failed to write story harness review report: {error}")
        return 1
    print(review_text, end="")
    return 1 if errors else 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run ail-story against the v0.3 live llama.cpp endpoint. "
            "Use --dry-run to print commands without network access."
        ),
        epilog=f"Default live server: {DEFAULT_SERVER}",
    )
    parser.add_argument(
        "--endpoint",
        default=DEFAULT_ENDPOINT,
        help=f"LLM endpoint (default: {DEFAULT_ENDPOINT})",
    )
    parser.add_argument(
        "--package",
        default=DEFAULT_PACKAGE,
        help=f"AIL package (default: {DEFAULT_PACKAGE})",
    )
    parser.add_argument(
        "--story-file",
        default=DEFAULT_STORY_FILE,
        help=f"Story file passed to ail-story (default: {DEFAULT_STORY_FILE})",
    )
    parser.add_argument(
        "--agent",
        default=None,
        help=(
            "Override the default toolchain agent package or bytecode; "
            "ail-story discovers examples/ail_toolchain_agent.ail when available"
        ),
    )
    parser.add_argument(
        "--artifact-dir",
        default=DEFAULT_ARTIFACT_DIR,
        help=f"Artifact directory (default: {DEFAULT_ARTIFACT_DIR})",
    )
    parser.add_argument(
        "--target",
        default=None,
        help="Optional native target, such as linux-x86_64-elf",
    )
    parser.add_argument(
        "--action",
        default=None,
        help="Required with --target; action to compile",
    )
    parser.add_argument("--out", default=None, help="Required with --target; native output path")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the model check and ail-story commands without running them",
    )
    parser.add_argument(
        "--skip-model-check",
        action="store_true",
        help="Skip the /v1/models check before running ail-story",
    )
    parser.add_argument(
        "--review-artifacts",
        help="Review an existing ail-story artifact directory without network access",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.review_artifacts:
        return review_artifacts(args.review_artifacts)
    command = build_ail_story_command(args)
    if args.dry_run:
        print("model check:")
        print("curl -sS " + models_url_for_endpoint(args.endpoint))
        print("ail-story:")
        print(" ".join(command))
        print("artifacts:")
        print(args.artifact_dir)
        return 0
    if not args.skip_model_check:
        check_models(args.endpoint)
    completed = subprocess.run(command, check=False)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
