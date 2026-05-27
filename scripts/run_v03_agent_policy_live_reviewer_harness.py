#!/usr/bin/env python3
"""Run the AIL v0.3 AgentTool policy live reviewer harness.

This harness is opt-in. It asks each configured reviewer role to inspect the
AgentTool policy handoff evidence and records the hosted request/response
bundle for offline review before any output is promoted into ./examples.
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
DEFAULT_ARTIFACT_DIR = "/tmp/ail-v03-agent-policy-live-review"
DEFAULT_EXAMPLES_ARTIFACTS = "/tmp/ail-manual-agent-policy"
DEFAULT_CAPTURE_PLAN_DIR = "/tmp/ail-manual-agent-policy-capture-plan"
DEFAULT_IMPORT_WORK_DIR = "/tmp/ail-manual-agent-policy-import-work"
DEFAULT_MAX_TOKENS = 768
EVIDENCE_EXCERPT_MAX_CHARS = 1400
ARTIFACT_KIND = "AIL-AgentTool-Live-Reviewer-Handoff"
REQUIRED_EVIDENCE = (
    "agent-policy-review.txt",
    "agent-policy-capture-plan.json",
    "agent-policy-import-demo-report.txt",
    "agent-policy-multi-agent-handoff-report.txt",
)
ROLE_CONTRACTS = (
    ("requirements-writer", "examples/agents/codex-ail-requirements-writer.md"),
    ("spec-writer", "examples/agents/codex-ail-spec-writer.md"),
    ("diagnostic-repairer", "examples/agents/codex-ail-diagnostic-repairer.md"),
    ("prompt-reviewer", "examples/agents/codex-ail-prompt-reviewer.md"),
    (
        "agent-policy-reviewer",
        "examples/agents/codex-ail-agent-policy-reviewer.md",
    ),
)


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


def bounded_excerpt(text: str, limit: int = EVIDENCE_EXCERPT_MAX_CHARS) -> str:
    if len(text) <= limit:
        return text.rstrip()
    omitted = len(text) - limit
    return text[:limit].rstrip() + f"\n...[truncated {omitted} chars]"


def fingerprint_path_for(path: Path) -> Path:
    return path.with_suffix(".fingerprint.txt")


def evidence_paths(args: argparse.Namespace) -> list[tuple[str, Path]]:
    return [
        (
            "agent-policy-review.txt",
            Path(args.examples_artifacts)
            / "examples"
            / args.source_entry_id
            / "agent-policy-review.txt",
        ),
        (
            "agent-policy-capture-plan.json",
            Path(args.capture_plan_dir) / "agent-policy-capture-plan.json",
        ),
        (
            "agent-policy-import-demo-report.txt",
            Path(args.import_work_dir) / "agent-policy-import-demo-report.txt",
        ),
        (
            "agent-policy-multi-agent-handoff-report.txt",
            Path(args.import_work_dir)
            / "agent-policy-multi-agent-handoff-report.txt",
        ),
    ]


def build_evidence_bundle(args: argparse.Namespace) -> tuple[str, list[str]]:
    errors: list[str] = []
    artifact_sections: list[str] = []
    for artifact_name, path in evidence_paths(args):
        text = read_required_text(path, errors)
        fingerprint_path = fingerprint_path_for(path)
        expected = read_required_text(fingerprint_path, errors).strip()
        actual = fnv64(text) if text else ""
        if text and expected and expected != actual:
            errors.append(
                f"fingerprint mismatch {path}: expected {expected} got {actual}"
            )
        fingerprint_status = "matched" if text and expected == actual else "missing"
        if text and expected and expected != actual:
            fingerprint_status = "mismatch"
        artifact_sections.append(
            "\n".join(
                [
                    f"artifact {artifact_name}",
                    f"path {path}",
                    f"fingerprint {actual or '<missing>'}",
                    f"fingerprint-file {fingerprint_path}",
                    f"fingerprint-file-value {expected or '<missing>'}",
                    f"fingerprint-status {fingerprint_status}",
                    "content-excerpt:",
                    f"----- begin {artifact_name} -----",
                    bounded_excerpt(text) if text else "<missing>",
                    f"----- end {artifact_name} -----",
                ]
            )
        )
    status = "complete" if not errors else "incomplete"
    bundle_body = "\n\n".join(artifact_sections)
    bundle_fingerprint = fnv64(bundle_body)
    lines = [
        f"Evidence bundle status: {status}",
        f"source-entry-id {args.source_entry_id}",
        f"proposed-entry-id {args.proposed_entry_id}",
        f"evidence-bundle-fingerprint {bundle_fingerprint}",
        "",
        bundle_body,
    ]
    if errors:
        lines.extend(["", "evidence-bundle-errors:"])
        lines.extend(f"error {error}" for error in errors)
    return "\n".join(lines).rstrip() + "\n", errors


def evidence_bundle_declared_fingerprint(evidence_bundle: str) -> str:
    for line in evidence_bundle.splitlines():
        if line.startswith("evidence-bundle-fingerprint "):
            return line.split(" ", 1)[1]
    return fnv64(evidence_bundle)


def request_has_evidence_bundle(user_probe: str) -> tuple[bool, str]:
    if "Evidence bundle status: complete" not in user_probe:
        return False, "request missing complete evidence bundle status"
    if "evidence-bundle-fingerprint fnv64:" not in user_probe:
        return False, "request missing evidence bundle fingerprint"
    for artifact_name in REQUIRED_EVIDENCE:
        if f"artifact {artifact_name}" not in user_probe:
            return False, f"request missing evidence artifact {artifact_name}"
        if f"----- begin {artifact_name} -----" not in user_probe:
            return False, f"request missing evidence content for {artifact_name}"
    for required_snippet in [
        "agent-policy-review-fingerprint-observed-count",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
        "multi-agent-execution-evidence deterministic-role-handoff",
    ]:
        if required_snippet not in user_probe:
            return False, f"request missing evidence snippet {required_snippet}"
    return True, ""


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


SKIPPED_MODEL_CHECK_ERROR = (
    "model-check skipped; hosted AgentTool reviewer evidence requires models.json from /v1/models"
)


def model_entries(models_text: str) -> tuple[str, list[str], str]:
    if not models_text.strip():
        return "missing", [], "models.json is empty"
    try:
        payload = json.loads(models_text)
    except json.JSONDecodeError as error:
        return "missing", [], f"invalid json models.json: {error}"
    if not isinstance(payload, dict):
        return "missing", [], "models.json must be a JSON object"
    if payload.get("skipped") is True:
        return "skipped", [], ""
    raw_entries = payload.get("data")
    if not isinstance(raw_entries, list):
        raw_entries = payload.get("models")
    if not isinstance(raw_entries, list):
        return "missing", [], "models.json must include data or models list"
    ids: list[str] = []
    for entry in raw_entries:
        if not isinstance(entry, dict):
            continue
        model_id = entry.get("id") or entry.get("name") or entry.get("model")
        if isinstance(model_id, str) and model_id.strip():
            ids.append(model_id.strip())
    if not ids:
        return "missing", [], "models.json did not name any models"
    return "present", ids, ""


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


def envelope_contract(role: str) -> str:
    return f"""Envelope contract:
Return JSON only. Do not wrap the JSON in Markdown. Use exactly this top-level
shape:
{{
  "artifact_kind": "{ARTIFACT_KIND}",
  "role": "{role}",
  "decision": "accept",
  "evidence": ["agent-policy-review.txt"],
  "questions": [],
  "checker_handoff": {{
    "must_check": true,
    "required_artifacts": ["agent-policy-review.txt"]
  }}
}}

Rules:
- artifact_kind must be "{ARTIFACT_KIND}".
- role must be "{role}".
- decision must be one of accept, needs-repair, or reject.
- evidence must name the AgentTool policy handoff artifacts reviewed.
- checker_handoff.must_check must be true.
- Return accept only if the policy review, capture plan, import demo, and
  deterministic multi-agent handoff witness are all present and coherent.
"""


def role_probe(
    role: str, source_entry_id: str, proposed_entry_id: str, evidence_bundle: str
) -> str:
    reviewed = ", ".join(REQUIRED_EVIDENCE)
    return f"""Review AgentTool policy handoff evidence.

Source entry id: {source_entry_id}
Proposed entry id: {proposed_entry_id}
Reviewer role: {role}
Evidence bundle expected: {reviewed}

Deterministic evidence bundle:
{evidence_bundle.rstrip()}

Determine whether your role accepts the handoff for human-approved import.
Do not claim that the corpus was edited by the hosted model.

{envelope_contract(role).rstrip()}
"""


def parse_envelope(content: str) -> tuple[dict[str, object] | None, str]:
    candidate = content.strip()
    if not candidate:
        return None, "reviewer response content is empty"
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
            return None, "reviewer response must be a JSON envelope"
        try:
            value = json.loads(candidate[start : end + 1])
        except json.JSONDecodeError as error:
            return None, f"reviewer response contains invalid JSON envelope: {error}"
    if not isinstance(value, dict):
        return None, "reviewer response envelope must be an object"
    return value, ""


def classify_reviewer_content(content: str, role: str) -> tuple[str, str, str]:
    envelope, error = parse_envelope(content)
    if envelope is None:
        return "empty" if not content.strip() else "invalid", "missing", error
    if envelope.get("artifact_kind") != ARTIFACT_KIND:
        return (
            "invalid",
            "missing",
            f"reviewer artifact_kind must be {ARTIFACT_KIND}",
        )
    if envelope.get("role") != role:
        return "invalid", "missing", f"reviewer role must be {role}"
    decision = envelope.get("decision")
    if decision not in {"accept", "needs-repair", "reject"}:
        return "invalid", "missing", "reviewer decision must be accept, needs-repair, or reject"
    evidence = envelope.get("evidence")
    if not isinstance(evidence, list) or not all(
        isinstance(item, str) for item in evidence
    ):
        return "invalid", str(decision), "reviewer evidence must be a list of strings"
    if decision == "accept":
        missing = [item for item in REQUIRED_EVIDENCE if item not in evidence]
        if missing:
            return (
                "invalid",
                str(decision),
                "accept decision missing evidence: " + ", ".join(missing),
            )
    questions = envelope.get("questions", [])
    if not isinstance(questions, list) or not all(
        isinstance(question, str) for question in questions
    ):
        return "invalid", str(decision), "reviewer questions must be a list of strings"
    checker_handoff = envelope.get("checker_handoff")
    if not isinstance(checker_handoff, dict):
        return "invalid", str(decision), "checker_handoff must be an object"
    if checker_handoff.get("must_check") is not True:
        return "invalid", str(decision), "checker_handoff.must_check must be true"
    return "reviewer-envelope", str(decision), ""


def role_contracts() -> list[tuple[str, str, str]]:
    contracts: list[tuple[str, str, str]] = []
    for role, contract_path in ROLE_CONTRACTS:
        path = ROOT / contract_path
        if not path.exists():
            raise SystemExit(f"missing reviewer contract {contract_path}")
        contracts.append((role, contract_path, path.read_text()))
    return contracts


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run live AgentTool policy reviewer probes. Use --dry-run to list "
            "role probes without network access."
        ),
        epilog=f"Default live server: {DEFAULT_SERVER}",
    )
    parser.add_argument(
        "--endpoint",
        default=DEFAULT_ENDPOINT,
        help=f"LLM endpoint (default: {DEFAULT_ENDPOINT})",
    )
    parser.add_argument(
        "--artifact-dir",
        default=DEFAULT_ARTIFACT_DIR,
        help=f"Artifact directory (default: {DEFAULT_ARTIFACT_DIR})",
    )
    parser.add_argument("--model", help="Optional model id for OpenAI-compatible servers")
    parser.add_argument("--max-tokens", type=int, default=DEFAULT_MAX_TOKENS)
    parser.add_argument("--source-entry-id", default="example-40")
    parser.add_argument("--proposed-entry-id", default="example-40-policy")
    parser.add_argument(
        "--examples-artifacts",
        default=DEFAULT_EXAMPLES_ARTIFACTS,
        help=(
            "Deterministic ail-examples artifact directory containing "
            "examples/<source-entry-id>/agent-policy-review.txt "
            f"(default: {DEFAULT_EXAMPLES_ARTIFACTS})"
        ),
    )
    parser.add_argument(
        "--capture-plan-dir",
        default=DEFAULT_CAPTURE_PLAN_DIR,
        help=(
            "Directory containing agent-policy-capture-plan.json "
            f"(default: {DEFAULT_CAPTURE_PLAN_DIR})"
        ),
    )
    parser.add_argument(
        "--import-work-dir",
        default=DEFAULT_IMPORT_WORK_DIR,
        help=(
            "Directory containing AgentTool import and multi-agent handoff reports "
            f"(default: {DEFAULT_IMPORT_WORK_DIR})"
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print reviewer probes without contacting the LLM endpoint",
    )
    parser.add_argument(
        "--skip-model-check",
        action="store_true",
        help="Skip the /v1/models check before running reviewer probes",
    )
    parser.add_argument(
        "--review-artifacts",
        help="Review an existing AgentTool live reviewer artifact directory without network access",
    )
    parser.add_argument(
        "--allow-skipped-model-check",
        action="store_true",
        help=(
            "Allow review of artifacts produced with --skip-model-check. "
            "Use only for local fake-server tests, not hosted llama.cpp evidence."
        ),
    )
    return parser.parse_args(argv)


def print_dry_run(args: argparse.Namespace, contracts: list[tuple[str, str, str]]) -> None:
    evidence_bundle, evidence_errors = build_evidence_bundle(args)
    evidence_fingerprint = evidence_bundle_declared_fingerprint(evidence_bundle)
    print("AIL-Agent-Policy-Live-Reviewer-Harness:")
    print(f"model-check curl -sS {models_url_for_endpoint(args.endpoint)}")
    print(f"endpoint {args.endpoint}")
    print(f"artifact-dir {args.artifact_dir}")
    print(f"default-max-tokens {DEFAULT_MAX_TOKENS}")
    print(f"max-tokens {args.max_tokens}")
    for line in token_budget_lines(args.max_tokens):
        print(line)
    print(f"examples-artifacts {args.examples_artifacts}")
    print(f"capture-plan-dir {args.capture_plan_dir}")
    print(f"import-work-dir {args.import_work_dir}")
    print(f"source-entry-id {args.source_entry_id}")
    print(f"proposed-entry-id {args.proposed_entry_id}")
    print(f"role-count {len(contracts)}")
    print(f"artifact-kind {ARTIFACT_KIND}")
    print(f"evidence-bundle-fingerprint {evidence_fingerprint}")
    print(f"evidence-bundle-error-count {len(evidence_errors)}")
    for role, contract_path, contract_text in contracts:
        probe = role_probe(
            role, args.source_entry_id, args.proposed_entry_id, evidence_bundle
        )
        print(
            f"role {role} contract {contract_path} "
            f"contract-fingerprint {fnv64(contract_text)} "
            f"probe-fingerprint {fnv64(probe)}"
        )


def review_artifacts(args: argparse.Namespace, contracts: list[tuple[str, str, str]]) -> int:
    artifact_root = Path(args.review_artifacts)
    errors: list[str] = []
    fingerprint_checks = 0
    content_nonempty = 0
    envelope_valid = 0
    envelope_invalid = 0
    evidence_bundle_present = 0
    decision_accept = 0
    decision_needs_repair = 0
    decision_reject = 0
    manifest_text = read_required_text(
        artifact_root / "manifest.v03-agent-policy-live-review.txt", errors
    )
    report_text = read_required_text(
        artifact_root / "agent-policy-live-review-report.txt", errors
    )
    models_text = read_required_text(artifact_root / "models.json", errors)
    model_check_status, model_ids, model_check_error = model_entries(models_text)
    if model_check_error:
        errors.append(model_check_error)
    if model_check_status == "skipped" and not args.allow_skipped_model_check:
        errors.append(SKIPPED_MODEL_CHECK_ERROR)
    model_id_set = set(model_ids)
    report_lines = report_text.splitlines()

    for path, fingerprint_path in [
        (
            artifact_root / "manifest.v03-agent-policy-live-review.txt",
            artifact_root / "manifest.fingerprint.txt",
        ),
        (
            artifact_root / "agent-policy-live-review-report.txt",
            artifact_root / "agent-policy-live-review-report.fingerprint.txt",
        ),
    ]:
        if check_fingerprint(path, errors, fingerprint_path):
            fingerprint_checks += 1
    if check_fingerprint(artifact_root / "models.json", errors):
        fingerprint_checks += 1

    if "AIL-Agent-Policy-Live-Reviewer-Harness-Manifest:" not in manifest_text:
        errors.append("manifest missing AIL-Agent-Policy-Live-Reviewer-Harness-Manifest header")
    if "AIL-Agent-Policy-Live-Reviewer-Harness:" not in report_text:
        errors.append("report missing AIL-Agent-Policy-Live-Reviewer-Harness header")
    if f"role-count {len(contracts)}" not in report_text:
        errors.append(f"report missing role-count {len(contracts)}")
    default_max_tokens = report_field(report_lines, "default-max-tokens")
    max_tokens = report_field(report_lines, "max-tokens")
    token_budget_default = report_field(report_lines, "token-budget-default")
    token_budget_warnings = [
        line for line in report_lines if line.startswith("token-budget-warning ")
    ]
    expected_default = str(DEFAULT_MAX_TOKENS)
    if default_max_tokens != expected_default:
        errors.append(
            "report default-max-tokens mismatch: "
            f"expected {expected_default} got {default_max_tokens or '<missing>'}"
        )
    if max_tokens is None:
        errors.append("report missing max-tokens")
    if token_budget_default is None:
        errors.append("report missing token-budget-default")

    for role, contract_path, contract_text in contracts:
        request_path = artifact_root / "requests" / f"{role}.json"
        response_path = artifact_root / "responses" / f"{role}.json"
        content_path = artifact_root / "content" / f"{role}.txt"
        request = load_json(request_path, errors)
        response = load_json(response_path, errors)
        content = read_required_text(content_path, errors)
        content_kind, decision, content_error = classify_reviewer_content(content, role)
        if content.strip():
            content_nonempty += 1
        else:
            errors.append(f"empty content {role}")
        if content_kind == "reviewer-envelope":
            envelope_valid += 1
        else:
            envelope_invalid += 1
            errors.append(f"invalid reviewer envelope {role}: {content_error}")
        if decision == "accept":
            decision_accept += 1
        elif decision == "needs-repair":
            decision_needs_repair += 1
        elif decision == "reject":
            decision_reject += 1
        for artifact_path in [request_path, response_path, content_path]:
            if check_fingerprint(artifact_path, errors):
                fingerprint_checks += 1
        if request:
            if request.get("role") != role:
                errors.append(f"request role mismatch {role}")
            if request.get("contract_file") != contract_path:
                errors.append(f"request contract_file mismatch {role}")
            if request.get("contract_fingerprint") != fnv64(contract_text):
                errors.append(f"request contract_fingerprint mismatch {role}")
            user_probe = request_user_probe(request.get("body"))
            if ARTIFACT_KIND not in user_probe:
                errors.append(f"request missing envelope contract {role}")
            has_evidence, evidence_error = request_has_evidence_bundle(user_probe)
            if has_evidence:
                evidence_bundle_present += 1
            else:
                errors.append(f"{evidence_error} {role}")
        if not response:
            errors.append(f"missing response json {role}")
        elif model_check_status == "present":
            response_model = response.get("model")
            if not isinstance(response_model, str) or not response_model.strip():
                errors.append(f"response model missing for {role}")
            elif response_model.strip() not in model_id_set:
                errors.append(
                    f"response model {response_model.strip()} not present in models.json for {role}"
                )
        if f"artifact {role} contract {contract_path}" not in manifest_text:
            errors.append(f"manifest missing reviewer artifact {role}")
        role_report_line = next(
            (line for line in report_lines if line.startswith(f"role {role} ")),
            "",
        )
        if not role_report_line:
            errors.append(f"report missing role {role}")
        elif f"content-kind {content_kind}" not in role_report_line:
            errors.append(f"report content-kind mismatch {role}: expected {content_kind}")
        if role_report_line and f"decision {decision}" not in role_report_line:
            errors.append(f"report decision mismatch {role}: expected {decision}")

    review_lines = [
        "AIL-Agent-Policy-Live-Reviewer-Harness-Review:",
        f"artifact-dir {artifact_root}",
        f"role-count {len(contracts)}",
        f"default-max-tokens {default_max_tokens or '<missing>'}",
        f"max-tokens {max_tokens or '<missing>'}",
        f"token-budget-default {token_budget_default or '<missing>'}",
        *token_budget_warnings,
        f"content-nonempty-count {content_nonempty}",
        f"reviewer-envelope-valid-count {envelope_valid}",
        f"reviewer-envelope-invalid-count {envelope_invalid}",
        f"evidence-bundle-present-count {evidence_bundle_present}",
        f"reviewer-decision-accept-count {decision_accept}",
        f"reviewer-decision-needs-repair-count {decision_needs_repair}",
        f"reviewer-decision-reject-count {decision_reject}",
        f"model-check {model_check_status}",
        f"model-check-model-count {len(model_ids)}",
        f"model-check-model-id {','.join(model_ids) if model_ids else '<missing>'}",
        f"fingerprint-check-count {fingerprint_checks}",
    ]
    if errors:
        review_lines.append("review-result rejected")
        for error in errors:
            review_lines.append(f"error {error}")
    elif decision_accept != len(contracts):
        review_lines.append("review-result needs-repair")
        review_lines.append("error reviewer decisions require repair before promotion")
    else:
        review_lines.append("review-result accepted")
    review_text = "\n".join(review_lines) + "\n"
    try:
        write_text(artifact_root / "agent-policy-live-review-review.txt", review_text)
        write_text(
            artifact_root / "agent-policy-live-review-review.fingerprint.txt",
            fnv64(review_text) + "\n",
        )
    except OSError as error:
        print(review_text, end="")
        print(f"error failed to write AgentTool live reviewer review report: {error}")
        return 1
    print(review_text, end="")
    return 1 if errors or decision_accept != len(contracts) else 0


def report_field(report_lines: list[str], key: str) -> str | None:
    prefix = f"{key} "
    for line in report_lines:
        if line.startswith(prefix):
            return line[len(prefix) :].strip()
    return None


def run_live(args: argparse.Namespace, contracts: list[tuple[str, str, str]]) -> int:
    evidence_bundle, evidence_errors = build_evidence_bundle(args)
    evidence_fingerprint = evidence_bundle_declared_fingerprint(evidence_bundle)
    if evidence_errors:
        print("AIL-Agent-Policy-Live-Reviewer-Harness:")
        print("evidence-result rejected")
        for error in evidence_errors:
            print(f"error {error}")
        return 1
    artifact_root = Path(args.artifact_dir)
    artifact_root.mkdir(parents=True, exist_ok=True)
    report_lines = [
        "AIL-Agent-Policy-Live-Reviewer-Harness:",
        f"endpoint {args.endpoint}",
        f"models-url {models_url_for_endpoint(args.endpoint)}",
        f"default-max-tokens {DEFAULT_MAX_TOKENS}",
        f"max-tokens {args.max_tokens}",
        *token_budget_lines(args.max_tokens),
        f"role-count {len(contracts)}",
        f"source-entry-id {args.source_entry_id}",
        f"proposed-entry-id {args.proposed_entry_id}",
        f"examples-artifacts {args.examples_artifacts}",
        f"capture-plan-dir {args.capture_plan_dir}",
        f"import-work-dir {args.import_work_dir}",
        f"evidence-bundle-fingerprint {evidence_fingerprint}",
    ]
    manifest_lines = ["AIL-Agent-Policy-Live-Reviewer-Harness-Manifest:"]
    if not args.skip_model_check:
        models = get_json(models_url_for_endpoint(args.endpoint))
        models_text = json.dumps(models, indent=2, sort_keys=True) + "\n"
        write_text(artifact_root / "models.json", models_text)
        write_text(artifact_root / "models.fingerprint.txt", fnv64(models_text) + "\n")
        manifest_lines.append("artifact models models.json models.fingerprint.txt")
    else:
        models_text = (
            json.dumps(
                {
                    "object": "ail-model-check",
                    "skipped": True,
                    "endpoint": args.endpoint,
                    "models_url": models_url_for_endpoint(args.endpoint),
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        write_text(artifact_root / "models.json", models_text)
        write_text(artifact_root / "models.fingerprint.txt", fnv64(models_text) + "\n")
        manifest_lines.append("artifact models models.json models.fingerprint.txt")

    for role, contract_path, contract_text in contracts:
        user_probe = role_probe(
            role, args.source_entry_id, args.proposed_entry_id, evidence_bundle
        )
        body = completion_body(
            args.endpoint, contract_text, user_probe, args.max_tokens, args.model
        )
        response = request_json(args.endpoint, body)
        request_text = (
            json.dumps(
                {
                    "endpoint": args.endpoint,
                    "method": "POST",
                    "role": role,
                    "contract_file": contract_path,
                    "contract_fingerprint": fnv64(contract_text),
                    "evidence_bundle_fingerprint": evidence_fingerprint,
                    "probe_fingerprint": fnv64(user_probe),
                    "body": body,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        response_text = json.dumps(response, indent=2, sort_keys=True) + "\n"
        content_text = extract_content(response) + "\n"
        content_kind, decision, _error = classify_reviewer_content(content_text, role)
        request_path = artifact_root / "requests" / f"{role}.json"
        response_path = artifact_root / "responses" / f"{role}.json"
        content_path = artifact_root / "content" / f"{role}.txt"
        write_text(request_path, request_text)
        write_text(response_path, response_text)
        write_text(content_path, content_text)
        write_text(request_path.with_suffix(".fingerprint.txt"), fnv64(request_text) + "\n")
        write_text(response_path.with_suffix(".fingerprint.txt"), fnv64(response_text) + "\n")
        write_text(content_path.with_suffix(".fingerprint.txt"), fnv64(content_text) + "\n")
        report_lines.append(
            f"role {role} contract {contract_path} "
            f"contract-fingerprint {fnv64(contract_text)} "
            f"probe-fingerprint {fnv64(user_probe)} "
            f"response-fingerprint {fnv64(response_text)} "
            f"content-kind {content_kind} decision {decision} "
            f"content-bytes {len(content_text.encode())}"
        )
        manifest_lines.append(
            f"artifact {role} contract {contract_path} "
            f"request requests/{role}.json requests/{role}.fingerprint.txt "
            f"response responses/{role}.json responses/{role}.fingerprint.txt "
            f"content content/{role}.txt content/{role}.fingerprint.txt"
        )

    report_text = "\n".join(report_lines) + "\n"
    manifest_text = "\n".join(manifest_lines) + "\n"
    write_text(artifact_root / "agent-policy-live-review-report.txt", report_text)
    write_text(
        artifact_root / "agent-policy-live-review-report.fingerprint.txt",
        fnv64(report_text) + "\n",
    )
    write_text(artifact_root / "manifest.v03-agent-policy-live-review.txt", manifest_text)
    write_text(artifact_root / "manifest.fingerprint.txt", fnv64(manifest_text) + "\n")
    print(report_text, end="")
    print(f"artifacts {artifact_root}")
    return 0


def token_budget_lines(max_tokens: int) -> list[str]:
    if max_tokens < DEFAULT_MAX_TOKENS:
        return [
            "token-budget-default false",
            "token-budget-warning max-tokens-below-default",
        ]
    if max_tokens == DEFAULT_MAX_TOKENS:
        return ["token-budget-default true"]
    return [
        "token-budget-default false",
        "token-budget-warning max-tokens-above-default",
    ]


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    contracts = role_contracts()
    if args.review_artifacts:
        return review_artifacts(args, contracts)
    if args.dry_run:
        print_dry_run(args, contracts)
        return 0
    return run_live(args, contracts)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
