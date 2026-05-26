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
DEFAULT_MAX_TOKENS = 768
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
EXPECTED_ARTIFACT_KINDS: dict[str, str] = {
    "interview.system.md": "AIL-Interview",
    "requirements.system.md": "AIL-Requirements",
    "spec-draft.system.md": "AIL-Spec Canonical",
    "core-draft.system.md": "AIL-Core Candidate",
    "repair.system.md": "AIL-Spec Canonical",
    "diagnostic-repair.system.md": "AIL-Repair",
    "core-to-spec.system.md": "AIL-Spec Canonical",
    "core-to-summary.system.md": "AIL-Spec Friendly",
    "flow-patch.system.md": "AIL-Core Patch",
    "trace-debug.system.md": "Trace Explanation",
    "interop.system.md": "Interop Questions",
}
PROMPT_PROBES: dict[str, tuple[str, str]] = {
    "interview.system.md": (
        "interview-clarify-refund-tool",
        """Task: ask only the blocking semantic questions needed before drafting
requirements for this request.

Input:
{"profile":"Application","user_request":"Build a refund tool for support agents. It should block unsafe refunds and leave an audit trail.","known_context":["The approver role, refund threshold, and provider failure behavior are not specified."],"safety_class":"medium"}

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Interview. Put
blocking questions in questions unless the input is already sufficient. Do not
invent approvers, thresholds, provider calls, traces, or compiled artifacts.
""",
    ),
    "requirements.system.md": (
        "requirements-support-ticket-coverage",
        """Task: convert confirmed support-ticket interview answers into
AIL-Requirements.

Input answers:
- SupportAgent may close an existing ticket.
- Closing writes Ticket.status = Closed.
- Missing tickets produce TicketNotFound.
- InternalNote is Secret<Text> readable only by SupportAgent and SupportManager.
- Success records TicketClosed with ticket_id and actor_id.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Requirements.
Cover objects, action, failure, permission, secret, guarantee, and trace with
provenance to the answers. Do not add unconfirmed notifications or host calls.
""",
    ),
    "spec-draft.system.md": (
        "spec-draft-canonical-close-ticket",
        """Task: draft canonical AIL-Spec from checked requirements.

Checked requirements: SupportAgent may close an existing Ticket; closing writes
Ticket.status to Closed; missing tickets raise TicketNotFound; InternalNote is
Secret<Text> readable only by SupportAgent and SupportManager; success records
TicketClosed(ticket_id, actor_id).

Package manifest: package support_ticket profile Application.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Spec Canonical.
Use parser-owned headings and preserve permission, failure, secret, trace, and
provenance. Do not answer in friendly prose.
""",
    ),
    "core-draft.system.md": (
        "core-draft-provenance-close-ticket",
        """Task: lower canonical Close ticket AIL-Spec into candidate AIL-Core.

Spec facts: Action CloseTicket requires SupportAgent, reads Ticket.id, writes
Ticket.status, handles TicketNotFound, protects InternalNote as Secret<Text>,
and records Trace TicketClosed. Source ids are req-1 through req-5.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Core Candidate.
Emit node and edge text with provenance ids. Do not claim the graph is checked.
""",
    ),
    "repair.system.md": (
        "repair-preserve-permission-add-trace",
        """Task: repair a draft AIL-Spec from reviewer instructions.

Draft: CloseTicket already requires SupportAgent and handles TicketNotFound but
has no success trace. Repair request: add TicketClosed(ticket_id, actor_id) to
the successful CloseTicket path. Acceptance criteria: preserve the existing
permission and failure semantics.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Spec Canonical.
Return a complete replacement artifact, not a patch fragment. Do not delete
permission or failure declarations.
""",
    ),
    "diagnostic-repair.system.md": (
        "diagnostic-repair-missing-trace",
        """Task: respond to checker diagnostic AIL-TRACE-001.

Artifact kind: AIL-Core. Artifact text: node Action CloseTicket; edge
CloseTicket writes Ticket.status. Diagnostic: AIL-TRACE-001 action CloseTicket
is missing trace coverage. Provenance: req-3 says the user wants auditability
but does not name a trace event.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Repair. Ask a
blocking question if the trace name cannot be proven. Do not invent
Trace.ActionCompleted.
""",
    ),
    "core-to-spec.system.md": (
        "core-to-spec-roundtrip-close-ticket",
        """Task: render checked AIL-Core into canonical structured English.

AIL-Core: node Action CloseTicket id action.close; node Permission SupportAgent
id perm.support-agent; node Failure TicketNotFound id failure.ticket-not-found;
node Trace TicketClosed id trace.ticket-closed; edge action.close requires
perm.support-agent; edge action.close handles failure.ticket-not-found; edge
action.close records_trace trace.ticket-closed.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Spec Canonical.
Do not omit permission, failure, trace, ids, or canonical headings.
""",
    ),
    "core-to-summary.system.md": (
        "core-to-summary-human-review",
        """Task: explain checked AIL-Core to a non-engineer reviewer.

AIL-Core contains CloseTicket, SupportAgent permission, TicketNotFound failure,
InternalNote secret protection, and TicketClosed trace. There is no customer
email edge.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Spec Friendly.
Each paragraph must cite graph node ids. Do not claim friendly text is
parseable, and do not add customer email behavior.
""",
    ),
    "flow-patch.system.md": (
        "flow-patch-reviewed-escalation",
        """Task: convert a reviewed visual edit into an AIL-Core patch.

Base hash: fnv64:1111222233334444. Flow item: action-card CloseTicket. Visual
edit reviewed by Jordan: add EscalateTicket after CloseTicket when priority is
High and record TicketEscalated. Existing incident edges remain attached.

Return one JSON prompt-pack envelope. Use artifact_kind AIL-Core Patch. Emit
patch operations only; do not replace the whole spec with prose.
""",
    ),
    "trace-debug.system.md": (
        "trace-debug-ticket-closed",
        """Task: explain a runtime trace using only checked events.

Trace events: e1 CloseTicket started actor SupportAgent; e2 Ticket.status wrote
Closed; e3 TicketClosed recorded; e4 InternalNote redacted. Question: why did
the ticket close, and was the customer emailed?

Return one JSON prompt-pack envelope. Use artifact_kind Trace Explanation.
Cite event ids and graph node ids. Do not invent notification events or expose
redacted InternalNote content.
""",
    ),
    "interop.system.md": (
        "interop-pointer-ownership-questions",
        """Task: ask safe C interop questions before drafting bindings.

Binding request: expose char *read_sensor(int sensor_id) from sensor.h to AIL
System profile code. Known header says it may return NULL. Ownership, release
function, errno mapping, calling convention, symbol visibility, and
thread-safety are unknown.

Return one JSON prompt-pack envelope. Use artifact_kind Interop Questions.
Ask blocking questions for unknown ABI and safety semantics. Do not assume AIL
automatically releases returned pointers.
""",
    ),
}


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


def prompt_probe(prompt_name: str, override: str | None) -> tuple[str, str]:
    if override is not None:
        return "custom-live-probe", override
    try:
        return PROMPT_PROBES[prompt_name]
    except KeyError:
        raise SystemExit(f"missing hosted probe for prompt {prompt_name}") from None


def expected_artifact_kind(prompt_name: str) -> str:
    try:
        return EXPECTED_ARTIFACT_KINDS[prompt_name]
    except KeyError:
        raise SystemExit(f"missing expected artifact kind for prompt {prompt_name}") from None


def prompt_envelope_contract(prompt_name: str) -> str:
    artifact_kind = expected_artifact_kind(prompt_name)
    return f"""Envelope contract:
Return JSON only. Do not wrap the JSON in Markdown. Use exactly this top-level
shape:
{{
  "artifact_kind": "{artifact_kind}",
  "artifact_text": "",
  "questions": ["..."],
  "assumptions": [],
  "provenance": [],
  "checker_handoff": {{
    "must_check": true,
    "expected_profile": "Application",
    "expected_features": []
  }}
}}

Rules:
- artifact_kind must be "{artifact_kind}".
- Use artifact_text for a drafted artifact, or questions for blocking
  questions, not both.
- questions must be an array of strings, not objects.
- checker_handoff.must_check must be true.
- Do not include any keys outside this envelope unless the checker can ignore
  them without changing semantics.
"""


def render_user_probe(prompt_name: str, override: str | None) -> str:
    _label, probe_text = prompt_probe(prompt_name, override)
    return f"{probe_text.rstrip()}\n\n{prompt_envelope_contract(prompt_name).rstrip()}\n"


def request_user_probe(body: object) -> str:
    if not isinstance(body, dict):
        return ""
    messages = body.get("messages")
    if isinstance(messages, list):
        for message in messages:
            if (
                isinstance(message, dict)
                and message.get("role") == "user"
                and isinstance(message.get("content"), str)
            ):
                return str(message["content"]).strip()
    prompt = body.get("prompt")
    if isinstance(prompt, str) and "USER PROBE:" in prompt:
        return prompt.split("USER PROBE:", 1)[1].strip()
    return ""


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
            "response_format": {"type": "json_object"},
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


def classify_prompt_content(
    content: str, expected_kind: str | None = None
) -> tuple[str, str]:
    envelope, error = prompt_envelope_json(content)
    if envelope is None:
        return "empty" if not content.strip() else "invalid", error
    artifact_kind = envelope.get("artifact_kind")
    if not isinstance(artifact_kind, str) or not artifact_kind.strip():
        return "invalid", "prompt envelope artifact_kind must be a non-empty string"
    if expected_kind is not None and artifact_kind != expected_kind:
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
    questions = [
        question.strip() for question in questions_value if question.strip()
    ]
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


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


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


def load_json(path: Path, errors: list[str]) -> dict[str, object]:
    text = read_required_text(path, errors)
    if not text:
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError as error:
        errors.append(f"invalid json {path}: {error}")
        return {}
    if not isinstance(value, dict):
        errors.append(f"invalid json {path}: top-level value must be an object")
        return {}
    return value


def review_artifacts(args: argparse.Namespace, paths: list[Path]) -> int:
    artifact_root = Path(args.review_artifacts)
    errors: list[str] = []
    fingerprint_checks = 0
    content_nonempty = 0
    content_kind_counts = {
        "prompt-envelope-artifact": 0,
        "prompt-envelope-questions": 0,
        "invalid": 0,
        "empty": 0,
    }
    manifest_text = read_required_text(artifact_root / "manifest.v03-prompt-llm.txt", errors)
    report_text = read_required_text(artifact_root / "prompt-llm-harness-report.txt", errors)
    report_lines = report_text.splitlines()

    for path, fingerprint_path in [
        (
            artifact_root / "manifest.v03-prompt-llm.txt",
            artifact_root / "manifest.fingerprint.txt",
        ),
        (
            artifact_root / "prompt-llm-harness-report.txt",
            artifact_root / "prompt-llm-harness-report.fingerprint.txt",
        ),
    ]:
        if check_fingerprint(path, errors, fingerprint_path):
            fingerprint_checks += 1
    if (artifact_root / "models.json").exists():
        if check_fingerprint(artifact_root / "models.json", errors):
            fingerprint_checks += 1

    if "AIL-Prompt-LLM-Harness-Manifest:" not in manifest_text:
        errors.append("manifest missing AIL-Prompt-LLM-Harness-Manifest header")
    if "AIL-Prompt-LLM-Harness:" not in report_text:
        errors.append("report missing AIL-Prompt-LLM-Harness header")
    if f"prompt-count {len(paths)}" not in report_text:
        errors.append(f"report missing prompt-count {len(paths)}")

    for prompt_path in paths:
        rel = relative(prompt_path)
        stem = prompt_path.name.removesuffix(".system.md")
        prompt_text = prompt_path.read_text()
        expected_probe_label, _expected_probe = prompt_probe(prompt_path.name, args.probe)
        expected_probe = render_user_probe(prompt_path.name, args.probe)
        expected_probe_fingerprint = fnv64(expected_probe)
        request_path = artifact_root / "requests" / f"{stem}.json"
        response_path = artifact_root / "responses" / f"{stem}.json"
        content_path = artifact_root / "content" / f"{stem}.txt"

        request = load_json(request_path, errors)
        response = load_json(response_path, errors)
        content = read_required_text(content_path, errors)
        content_kind, content_error = classify_prompt_content(
            content, expected_artifact_kind(prompt_path.name)
        )
        content_kind_counts[content_kind] = content_kind_counts.get(content_kind, 0) + 1
        for artifact_path in [request_path, response_path, content_path]:
            if check_fingerprint(artifact_path, errors):
                fingerprint_checks += 1

        if request:
            if request.get("prompt_file") != rel:
                errors.append(f"request prompt_file mismatch {rel}")
            if request.get("prompt_fingerprint") != fnv64(prompt_text):
                errors.append(f"request prompt_fingerprint mismatch {rel}")
            if request.get("probe_label") != expected_probe_label:
                errors.append(f"request probe_label mismatch {rel}")
            if request.get("probe_fingerprint") != expected_probe_fingerprint:
                errors.append(f"request probe_fingerprint mismatch {rel}")
            if request_user_probe(request.get("body")).strip() != expected_probe.strip():
                errors.append(f"request user probe mismatch {rel}")
        if not response:
            errors.append(f"missing response json {rel}")
        if content.strip():
            content_nonempty += 1
        else:
            errors.append(f"empty content {rel}")
        if content_kind in {"invalid", "empty"}:
            errors.append(f"invalid prompt envelope {rel}: {content_error}")
        if rel not in report_text:
            errors.append(f"report missing prompt {rel}")
        if fnv64(prompt_text) not in report_text:
            errors.append(f"report missing prompt fingerprint {rel}")
        prompt_report_line = next(
            (line for line in report_lines if line.startswith(f"prompt {rel} ")),
            "",
        )
        if prompt_report_line and f"content-kind {content_kind}" not in prompt_report_line:
            errors.append(f"report content-kind mismatch {rel}: expected {content_kind}")
        if prompt_report_line and f"probe-label {expected_probe_label}" not in prompt_report_line:
            errors.append(f"report probe-label mismatch {rel}")
        if (
            prompt_report_line
            and f"probe-fingerprint {expected_probe_fingerprint}" not in prompt_report_line
        ):
            errors.append(f"report probe-fingerprint mismatch {rel}")
        if f"artifact {rel}" not in manifest_text:
            errors.append(f"manifest missing prompt artifact {rel}")

    print("AIL-Prompt-LLM-Harness-Review:")
    print(f"artifact-dir {artifact_root}")
    print(f"prompt-count {len(paths)}")
    print(f"content-nonempty-count {content_nonempty}")
    valid_count = (
        content_kind_counts["prompt-envelope-artifact"]
        + content_kind_counts["prompt-envelope-questions"]
    )
    print(f"prompt-envelope-valid-count {valid_count}")
    print(
        "prompt-envelope-artifact-count "
        f"{content_kind_counts['prompt-envelope-artifact']}"
    )
    print(
        "prompt-envelope-questions-count "
        f"{content_kind_counts['prompt-envelope-questions']}"
    )
    print(
        "prompt-envelope-invalid-count "
        f"{content_kind_counts['invalid'] + content_kind_counts['empty']}"
    )
    print(f"fingerprint-check-count {fingerprint_checks}")
    if errors:
        print("review-result rejected")
        for error in errors:
            print(f"error {error}")
        return 1
    print("review-result accepted")
    return 0


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
    parser.add_argument("--max-tokens", type=int, default=DEFAULT_MAX_TOKENS)
    parser.add_argument(
        "--probe",
        help=(
            "Override the task-specific probe text and send the same custom "
            "probe to every prompt"
        ),
    )
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
    parser.add_argument(
        "--review-artifacts",
        help="Review an existing prompt LLM harness artifact directory without network access",
    )
    return parser.parse_args(argv)


def print_dry_run(args: argparse.Namespace, paths: list[Path]) -> None:
    print("AIL-Prompt-LLM-Harness:")
    print(f"model-check curl -sS {models_url_for_endpoint(args.endpoint)}")
    print(f"endpoint {args.endpoint}")
    print(f"prompt-dir {args.prompt_dir}")
    print(f"artifact-dir {args.artifact_dir}")
    print(f"max-tokens {args.max_tokens}")
    for path in paths:
        probe_label, probe_text = prompt_probe(path.name, args.probe)
        user_probe = render_user_probe(path.name, args.probe)
        print(
            f"prompt {relative(path)} "
            f"probe-label {probe_label} probe-fingerprint {fnv64(user_probe)}"
        )


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
        probe_label, _probe_text = prompt_probe(path.name, args.probe)
        user_probe = render_user_probe(path.name, args.probe)
        probe_fingerprint = fnv64(user_probe)
        body = completion_body(
            args.endpoint, prompt_text, user_probe, args.max_tokens, args.model
        )
        response = request_json(args.endpoint, body)
        request_text = json.dumps(
            {
                "endpoint": args.endpoint,
                "method": "POST",
                "prompt_file": rel,
                "prompt_fingerprint": fnv64(prompt_text),
                "probe_label": probe_label,
                "probe_fingerprint": probe_fingerprint,
                "body": body,
            },
            indent=2,
            sort_keys=True,
        ) + "\n"
        response_text = json.dumps(response, indent=2, sort_keys=True) + "\n"
        content_text = extract_content(response) + "\n"
        content_kind, _content_error = classify_prompt_content(
            content_text, expected_artifact_kind(path.name)
        )
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
            f"probe-label {probe_label} probe-fingerprint {probe_fingerprint} "
            f"response-fingerprint {fnv64(response_text)} "
            f"content-kind {content_kind} "
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
    if args.review_artifacts:
        return review_artifacts(args, paths)
    if args.dry_run:
        print_dry_run(args, paths)
        return 0
    return run_live(args, paths)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
