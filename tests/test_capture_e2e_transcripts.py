import json
import importlib.util
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


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

EXPECTED_ARTIFACT_KINDS = {
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
EXPECTED_PROMPT_CONTENT_KINDS = {
    "interview.system.md": "prompt-envelope-questions",
    "requirements.system.md": "prompt-envelope-artifact",
    "spec-draft.system.md": "prompt-envelope-artifact",
    "core-draft.system.md": "prompt-envelope-artifact",
    "repair.system.md": "prompt-envelope-artifact",
    "diagnostic-repair.system.md": "prompt-envelope-questions",
    "core-to-spec.system.md": "prompt-envelope-artifact",
    "core-to-summary.system.md": "prompt-envelope-artifact",
    "flow-patch.system.md": "prompt-envelope-artifact",
    "trace-debug.system.md": "prompt-envelope-artifact",
    "interop.system.md": "prompt-envelope-questions",
}

_PROMPT_HARNESS_SPEC = importlib.util.spec_from_file_location(
    "run_v03_prompt_llm_harness",
    ROOT / "scripts" / "run_v03_prompt_llm_harness.py",
)
_PROMPT_HARNESS = importlib.util.module_from_spec(_PROMPT_HARNESS_SPEC)
assert _PROMPT_HARNESS_SPEC.loader is not None
_PROMPT_HARNESS_SPEC.loader.exec_module(_PROMPT_HARNESS)


def fnv64(text):
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def write_text(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def prompt_probe_envelope(prompt_name, artifact_kind_override=None):
    stem = prompt_name.removesuffix(".system.md")
    return (
        json.dumps(
            {
                "artifact_kind": artifact_kind_override
                or EXPECTED_ARTIFACT_KINDS[prompt_name],
                "artifact_text": "",
                "questions": [f"What semantics should {stem} preserve?"],
                "assumptions": [],
                "provenance": [f"test:{stem}"],
                "checker_handoff": {
                    "must_check": True,
                    "expected_profile": "Application",
                    "expected_features": [],
                },
            },
            sort_keys=True,
        )
        + "\n"
    )


def prompt_artifact_envelope(prompt_name, artifact_kind_override=None):
    stem = prompt_name.removesuffix(".system.md")
    return (
        json.dumps(
            {
                "artifact_kind": artifact_kind_override
                or EXPECTED_ARTIFACT_KINDS[prompt_name],
                "artifact_text": f"Checked artifact output for {stem}.",
                "questions": [],
                "assumptions": [],
                "provenance": [f"test:{stem}"],
                "checker_handoff": {
                    "must_check": True,
                    "expected_profile": "Application",
                    "expected_features": [],
                },
            },
            sort_keys=True,
        )
        + "\n"
    )


def write_prompt_llm_review_fixture(
    artifact_dir,
    empty_content_for=None,
    invalid_content_for=None,
    mismatched_probe_for=None,
    mismatched_artifact_kind_for=None,
    question_content_for=None,
):
    artifact_dir.mkdir(parents=True, exist_ok=True)
    models_text = (
        json.dumps({"object": "list", "data": [{"id": "test-model"}]}, sort_keys=True)
        + "\n"
    )
    write_text(artifact_dir / "models.json", models_text)
    write_text(artifact_dir / "models.fingerprint.txt", fnv64(models_text) + "\n")
    report_lines = [
        "AIL-Prompt-LLM-Harness:",
        "endpoint http://127.0.0.1:8080/v1/chat/completions",
        "models-url http://127.0.0.1:8080/v1/models",
        f"prompt-count {len(REQUIRED_PROMPTS)}",
    ]
    manifest_lines = [
        "AIL-Prompt-LLM-Harness-Manifest:",
        "artifact models models.json models.fingerprint.txt",
    ]
    for prompt_name in REQUIRED_PROMPTS:
        prompt_path = ROOT / "docs" / "ail" / "prompts" / prompt_name
        prompt_rel = f"docs/ail/prompts/{prompt_name}"
        prompt_text = prompt_path.read_text()
        stem = prompt_name.removesuffix(".system.md")
        probe_label, _base_probe_text = _PROMPT_HARNESS.prompt_probe(prompt_name, None)
        probe_text = _PROMPT_HARNESS.render_user_probe(prompt_name, None)
        probe_fingerprint = fnv64(probe_text)
        if mismatched_probe_for == prompt_name:
            probe_label = "generic-live-probe"
            probe_text = "AIL prompt-pack live probe."
            probe_fingerprint = fnv64(probe_text)
        if empty_content_for == prompt_name:
            content = ""
            content_kind = "empty"
        elif invalid_content_for == prompt_name:
            content = f"Raw non-envelope output for {stem}.\n"
            content_kind = "invalid"
        elif mismatched_artifact_kind_for == prompt_name:
            if (
                EXPECTED_PROMPT_CONTENT_KINDS[prompt_name]
                == "prompt-envelope-artifact"
            ):
                content = prompt_artifact_envelope(prompt_name, "AIL-Prompt-Probe")
                content_kind = "prompt-envelope-artifact"
            else:
                content = prompt_probe_envelope(prompt_name, "AIL-Prompt-Probe")
                content_kind = "prompt-envelope-questions"
        elif question_content_for == prompt_name:
            content = prompt_probe_envelope(prompt_name)
            content_kind = "prompt-envelope-questions"
        elif (
            EXPECTED_PROMPT_CONTENT_KINDS[prompt_name]
            == "prompt-envelope-artifact"
        ):
            content = prompt_artifact_envelope(prompt_name)
            content_kind = "prompt-envelope-artifact"
        else:
            content = prompt_probe_envelope(prompt_name)
            content_kind = "prompt-envelope-questions"
        response = {"choices": [{"message": {"content": content.strip()}}], "model": "test-model"}
        request_text = json.dumps(
            {
                "endpoint": "http://127.0.0.1:8080/v1/chat/completions",
                "method": "POST",
                "prompt_file": prompt_rel,
                "prompt_fingerprint": fnv64(prompt_text),
                "probe_label": probe_label,
                "probe_fingerprint": probe_fingerprint,
                "body": {
                    "messages": [
                        {"role": "system", "content": prompt_text},
                        {"role": "user", "content": probe_text},
                    ],
                    "max_tokens": 64,
                    "temperature": 0.0,
                    "stream": False,
                },
            },
            indent=2,
            sort_keys=True,
        ) + "\n"
        response_text = json.dumps(response, indent=2, sort_keys=True) + "\n"
        content_text = content
        write_text(artifact_dir / "requests" / f"{stem}.json", request_text)
        write_text(
            artifact_dir / "requests" / f"{stem}.fingerprint.txt",
            fnv64(request_text) + "\n",
        )
        write_text(artifact_dir / "responses" / f"{stem}.json", response_text)
        write_text(
            artifact_dir / "responses" / f"{stem}.fingerprint.txt",
            fnv64(response_text) + "\n",
        )
        write_text(artifact_dir / "content" / f"{stem}.txt", content_text)
        write_text(
            artifact_dir / "content" / f"{stem}.fingerprint.txt",
            fnv64(content_text) + "\n",
        )
        report_lines.append(
            f"prompt {prompt_rel} prompt-fingerprint {fnv64(prompt_text)} "
            f"probe-label {probe_label} probe-fingerprint {probe_fingerprint} "
            f"response-fingerprint {fnv64(response_text)} "
            f"content-kind {content_kind} "
            f"expected-content-kind {EXPECTED_PROMPT_CONTENT_KINDS[prompt_name]} "
            f"content-bytes {len(content_text.encode())}"
        )
        manifest_lines.append(
            f"artifact {prompt_rel} "
            f"request requests/{stem}.json requests/{stem}.fingerprint.txt "
            f"response responses/{stem}.json responses/{stem}.fingerprint.txt "
            f"content content/{stem}.txt content/{stem}.fingerprint.txt"
        )
    report_text = "\n".join(report_lines) + "\n"
    manifest_text = "\n".join(manifest_lines) + "\n"
    write_text(artifact_dir / "prompt-llm-harness-report.txt", report_text)
    write_text(
        artifact_dir / "prompt-llm-harness-report.fingerprint.txt",
        fnv64(report_text) + "\n",
    )
    write_text(artifact_dir / "manifest.v03-prompt-llm.txt", manifest_text)
    write_text(artifact_dir / "manifest.fingerprint.txt", fnv64(manifest_text) + "\n")


def write_story_llm_review_fixture(artifact_dir, omit_agent_trace=False):
    artifact_dir.mkdir(parents=True, exist_ok=True)
    story_source = (
        "# Support Ticket Story\n\n"
        "user-story-id: support-ticket-agent-story\n"
        "user-story: As a support agent I can close a ticket from a reviewed story.\n"
        "acceptance-criteria: checked requirements exist; checked spec exists; bytecode exists; agent trace exists\n"
        "semantic-anchors: Support Tickets; Close ticket; TicketClosed; toolchain agent\n"
    )
    story_normalized = story_source + "story-journey: story-to-spec\nstory-roundtrip: semantic-similar\n"
    story_report = (
        "AIL-Story-Mode-Report:\n"
        "entrypoint: ail-story\n"
        "package: support-ticket\n"
        "user-story-id: support-ticket-agent-story\n"
        "semantic-anchor-count: 4\n"
        "llm-endpoint: http://127.0.0.1:8080/v1/chat/completions\n"
        "story-llm-transcript-count: 2\n"
        "story-prompt-envelope-valid-count: 2\n"
        "story-prompt-envelope-invalid-count: 0\n"
    )
    model_check = (
        json.dumps(
            {
                "object": "list",
                "data": [
                    {
                        "id": "test-story-model",
                        "object": "model",
                        "owned_by": "llamacpp",
                    }
                ],
            },
            sort_keys=True,
        )
        + "\n"
    )
    requirements = (
        "AIL-Requirements:\n"
        "- The application manages support tickets.\n"
        "- The Close ticket action records TicketClosed.\n"
    )
    spec = (ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md").read_text()
    core = "AIL-Core:\nnode action CloseTicket\n"
    flow_review = json.dumps({"profile": "Application", "actions": ["Close ticket"]}) + "\n"
    bytecode = json.dumps({"version": "ail-bytecode-0", "package": "support-ticket"}) + "\n"
    agent_bytecode = (
        json.dumps({"version": "ail-bytecode-0", "package": "ail-toolchain-agent"}) + "\n"
    )
    requirements_envelope = (
        json.dumps(
            {
                "artifact_kind": "AIL-Requirements",
                "artifact_text": requirements,
                "questions": [],
                "checker_handoff": {
                    "must_check": True,
                    "expected_profile": "Application",
                    "expected_features": [],
                },
            },
            sort_keys=True,
        )
        + "\n"
    )
    spec_envelope = (
        json.dumps(
            {
                "artifact_kind": "AIL-Spec Canonical",
                "artifact_text": spec,
                "questions": [],
                "checker_handoff": {
                    "must_check": True,
                    "expected_profile": "Application",
                    "expected_features": [],
                },
            },
            sort_keys=True,
        )
        + "\n"
    )
    requirements_request = (
        json.dumps(
            {
                "messages": [
                    {
                        "role": "user",
                        "content": "USER STORY MODE INPUT\nReturn AIL-Requirements.",
                    }
                ],
                "temperature": 0.0,
            },
            sort_keys=True,
        )
        + "\n"
    )
    requirements_response = (
        json.dumps({"choices": [{"message": {"content": requirements_envelope.strip()}}]})
        + "\n"
    )
    spec_request = (
        json.dumps(
            {
                "messages": [
                    {
                        "role": "user",
                        "content": "Preserve these story semantic anchors and return AIL-Spec Canonical.",
                    }
                ],
                "temperature": 0.0,
            },
            sort_keys=True,
        )
        + "\n"
    )
    spec_response = (
        json.dumps({"choices": [{"message": {"content": spec_envelope.strip()}}]}) + "\n"
    )
    build_manifest = (
        "AIL-Build-Manifest:\n"
        f"bytecode artifact.ailbc.json {fnv64(bytecode)}\n"
        "trace agent-trace.txt\n"
    )
    agent_trace = (
        "entrypoint=ail-story\n"
        "buildrequest.story-id=support-ticket-agent-story\n"
        "buildrequest.semantic-anchors=Support Tickets; Close ticket; TicketClosed; toolchain agent\n"
        "action CaptureRequirements started\n"
        "action PrepareSpecDraft started\n"
        "action AcceptSpecDraft started\n"
        "action CompileApplication started\n"
        "action VerifyBytecodeArtifact started\n"
    )
    artifacts = [
        ("story.source.md", "story.source.fingerprint.txt", story_source),
        ("story.normalized.md", "story.normalized.fingerprint.txt", story_normalized),
        ("story-mode-report.txt", "story-mode-report.fingerprint.txt", story_report),
        ("requirements.ail-requirements.md", "requirements.fingerprint.txt", requirements),
        ("accepted.ail-spec.md", "accepted.ail-spec.fingerprint.txt", spec),
        ("checked.ail-core.txt", "checked.ail-core.fingerprint.txt", core),
        ("review.ail-flow.json", "review.ail-flow.fingerprint.txt", flow_review),
        ("artifact.ailbc.json", "artifact.fingerprint.txt", bytecode),
        ("agent.ailbc.json", "agent.fingerprint.txt", agent_bytecode),
        ("manifest.ail-build.txt", "manifest.fingerprint.txt", build_manifest),
        ("model-check.json", "model-check.fingerprint.txt", model_check),
        (
            "llm/requirements.request.json",
            "llm/requirements.request.fingerprint.txt",
            requirements_request,
        ),
        (
            "llm/requirements.response.json",
            "llm/requirements.response.fingerprint.txt",
            requirements_response,
        ),
        (
            "llm/requirements.content.txt",
            "llm/requirements.content.fingerprint.txt",
            requirements_envelope,
        ),
        ("llm/spec.request.json", "llm/spec.request.fingerprint.txt", spec_request),
        ("llm/spec.response.json", "llm/spec.response.fingerprint.txt", spec_response),
        ("llm/spec.content.txt", "llm/spec.content.fingerprint.txt", spec_envelope),
    ]
    if not omit_agent_trace:
        artifacts.append(("agent-trace.txt", "agent-trace.fingerprint.txt", agent_trace))
    for name, fingerprint_name, text in artifacts:
        write_text(artifact_dir / name, text)
        write_text(artifact_dir / fingerprint_name, fnv64(text) + "\n")

    manifest = (
        "AIL-Story-Manifest:\n"
        "entrypoint ail-story\n"
        f"story-source story.source.md {fnv64(story_source)}\n"
        f"story-normalized story.normalized.md {fnv64(story_normalized)}\n"
        f"story-report story-mode-report.txt {fnv64(story_report)}\n"
        f"requirements requirements.ail-requirements.md {fnv64(requirements)}\n"
        f"spec accepted.ail-spec.md {fnv64(spec)}\n"
        f"core checked.ail-core.txt {fnv64(core)}\n"
        f"bytecode artifact.ailbc.json {fnv64(bytecode)}\n"
        f"agent agent.ailbc.json {fnv64(agent_bytecode)}\n"
        f"model-check model-check.json {fnv64(model_check)}\n"
        "llm-requirements-request "
        f"llm/requirements.request.json {fnv64(requirements_request)}\n"
        "llm-requirements-response "
        f"llm/requirements.response.json {fnv64(requirements_response)}\n"
        "llm-requirements-content "
        f"llm/requirements.content.txt {fnv64(requirements_envelope)}\n"
        f"llm-spec-request llm/spec.request.json {fnv64(spec_request)}\n"
        f"llm-spec-response llm/spec.response.json {fnv64(spec_response)}\n"
        f"llm-spec-content llm/spec.content.txt {fnv64(spec_envelope)}\n"
        f"build-manifest manifest.ail-build.txt {fnv64(build_manifest)}\n"
    )
    if not omit_agent_trace:
        manifest = manifest.replace(
            "llm-requirements-request ",
            f"agent-trace agent-trace.txt {fnv64(agent_trace)}\n"
            "llm-requirements-request ",
            1,
        )
    write_text(artifact_dir / "manifest.ail-story.txt", manifest)
    write_text(artifact_dir / "manifest.ail-story.fingerprint.txt", fnv64(manifest) + "\n")


AGENT_POLICY_LIVE_ROLES = (
    ("requirements-writer", "examples/agents/codex-ail-requirements-writer.md"),
    ("spec-writer", "examples/agents/codex-ail-spec-writer.md"),
    ("diagnostic-repairer", "examples/agents/codex-ail-diagnostic-repairer.md"),
    ("prompt-reviewer", "examples/agents/codex-ail-prompt-reviewer.md"),
    ("agent-policy-reviewer", "examples/agents/codex-ail-agent-policy-reviewer.md"),
)


def agent_policy_live_reviewer_envelope(role, decision="accept"):
    return (
        json.dumps(
            {
                "artifact_kind": "AIL-AgentTool-Live-Reviewer-Handoff",
                "role": role,
                "decision": decision,
                "evidence": [
                    "agent-policy-review.txt",
                    "agent-policy-capture-plan.json",
                    "agent-policy-import-demo-report.txt",
                    "agent-policy-multi-agent-handoff-report.txt",
                ],
                "questions": [],
                "checker_handoff": {
                    "must_check": True,
                    "required_artifacts": [
                        "agent-policy-review.txt",
                        "agent-policy-import-demo-report.txt",
                    ],
                },
            },
            sort_keys=True,
        )
        + "\n"
    )


def write_agent_policy_live_reviewer_fixture(
    artifact_dir, empty_content_for=None, decision_for=None
):
    decision_for = decision_for or {}
    artifact_dir.mkdir(parents=True, exist_ok=True)
    models_text = (
        json.dumps({"object": "list", "data": [{"id": "test-model"}]}, sort_keys=True)
        + "\n"
    )
    write_text(artifact_dir / "models.json", models_text)
    write_text(artifact_dir / "models.fingerprint.txt", fnv64(models_text) + "\n")
    report_lines = [
        "AIL-Agent-Policy-Live-Reviewer-Harness:",
        "endpoint http://127.0.0.1:8080/v1/chat/completions",
        "models-url http://127.0.0.1:8080/v1/models",
        f"role-count {len(AGENT_POLICY_LIVE_ROLES)}",
        "source-entry-id example-40",
        "proposed-entry-id example-40-policy",
    ]
    manifest_lines = [
        "AIL-Agent-Policy-Live-Reviewer-Harness-Manifest:",
        "artifact models models.json models.fingerprint.txt",
    ]
    for role, contract_path in AGENT_POLICY_LIVE_ROLES:
        contract_text = (ROOT / contract_path).read_text()
        probe_text = agent_policy_live_evidence_probe_text()
        probe_fingerprint = fnv64(probe_text)
        if empty_content_for == role:
            content = ""
            content_kind = "empty"
            decision = "missing"
        else:
            decision = decision_for.get(role, "accept")
            content = agent_policy_live_reviewer_envelope(role, decision=decision)
            content_kind = "reviewer-envelope"
        response = {"choices": [{"message": {"content": content.strip()}}], "model": "test-model"}
        request_text = (
            json.dumps(
                {
                    "endpoint": "http://127.0.0.1:8080/v1/chat/completions",
                    "method": "POST",
                    "role": role,
                    "contract_file": contract_path,
                    "contract_fingerprint": fnv64(contract_text),
                    "probe_fingerprint": probe_fingerprint,
                    "body": {
                        "messages": [
                            {"role": "system", "content": contract_text},
                            {"role": "user", "content": probe_text},
                        ],
                        "max_tokens": 64,
                        "temperature": 0.0,
                        "stream": False,
                    },
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        response_text = json.dumps(response, indent=2, sort_keys=True) + "\n"
        write_text(artifact_dir / "requests" / f"{role}.json", request_text)
        write_text(
            artifact_dir / "requests" / f"{role}.fingerprint.txt",
            fnv64(request_text) + "\n",
        )
        write_text(artifact_dir / "responses" / f"{role}.json", response_text)
        write_text(
            artifact_dir / "responses" / f"{role}.fingerprint.txt",
            fnv64(response_text) + "\n",
        )
        write_text(artifact_dir / "content" / f"{role}.txt", content)
        write_text(
            artifact_dir / "content" / f"{role}.fingerprint.txt",
            fnv64(content) + "\n",
        )
        report_lines.append(
            f"role {role} contract {contract_path} "
            f"contract-fingerprint {fnv64(contract_text)} "
            f"probe-fingerprint {probe_fingerprint} "
            f"response-fingerprint {fnv64(response_text)} "
            f"content-kind {content_kind} decision {decision} "
            f"content-bytes {len(content.encode())}"
        )
        manifest_lines.append(
            f"artifact {role} contract {contract_path} "
            f"request requests/{role}.json requests/{role}.fingerprint.txt "
            f"response responses/{role}.json responses/{role}.fingerprint.txt "
            f"content content/{role}.txt content/{role}.fingerprint.txt"
        )
    report_text = "\n".join(report_lines) + "\n"
    manifest_text = "\n".join(manifest_lines) + "\n"
    write_text(artifact_dir / "agent-policy-live-review-report.txt", report_text)
    write_text(
        artifact_dir / "agent-policy-live-review-report.fingerprint.txt",
        fnv64(report_text) + "\n",
    )
    write_text(artifact_dir / "manifest.v03-agent-policy-live-review.txt", manifest_text)
    write_text(artifact_dir / "manifest.fingerprint.txt", fnv64(manifest_text) + "\n")


def write_agent_policy_live_evidence_fixture(
    examples_artifacts, capture_plan_dir, import_work_dir
):
    review_text = (
        "AIL-Agent-Policy-Review:\n"
        "entry-id example-40\n"
        "agent-policy-review-fingerprint-observed-count 1\n"
        "multi-agent-handoff-review required\n"
    )
    review_path = examples_artifacts / "examples" / "example-40" / "agent-policy-review.txt"
    write_text(review_path, review_text)
    write_text(review_path.with_suffix(".fingerprint.txt"), fnv64(review_text) + "\n")

    capture_plan = {
        "artifact_kind": "AIL-AgentTool-Policy-Capture-Plan",
        "entry_id": "example-40-policy",
        "human_approval_required": True,
        "source_entry_id": "example-40",
    }
    capture_text = json.dumps(capture_plan, indent=2, sort_keys=True) + "\n"
    capture_path = capture_plan_dir / "agent-policy-capture-plan.json"
    write_text(capture_path, capture_text)
    write_text(capture_path.with_suffix(".fingerprint.txt"), fnv64(capture_text) + "\n")

    import_text = (
        "AIL-Agent-Policy-Import-Demo:\n"
        "source-entry-id example-40\n"
        "proposed-entry-id example-40-policy\n"
        "source-preserved true\n"
        "proposed-accepted true\n"
        "policy-handoff-imported true\n"
        "policy-handoff-replayed true\n"
    )
    import_path = import_work_dir / "agent-policy-import-demo-report.txt"
    write_text(import_path, import_text)
    write_text(import_path.with_suffix(".fingerprint.txt"), fnv64(import_text) + "\n")

    handoff_text = (
        "AIL-Agent-Policy-Multi-Agent-Handoff:\n"
        "source-entry-id example-40\n"
        "proposed-entry-id example-40-policy\n"
        "separate-reviewer-role-count 5\n"
        "role requirements-writer contract codex-ail-requirements-writer\n"
        "role spec-writer contract codex-ail-spec-writer\n"
        "role diagnostic-repairer contract codex-ail-diagnostic-repairer\n"
        "role prompt-reviewer contract codex-ail-prompt-reviewer\n"
        "role agent-policy-reviewer contract codex-ail-agent-policy-reviewer\n"
        "multi-agent-execution-evidence deterministic-role-handoff\n"
    )
    handoff_path = import_work_dir / "agent-policy-multi-agent-handoff-report.txt"
    write_text(handoff_path, handoff_text)
    write_text(handoff_path.with_suffix(".fingerprint.txt"), fnv64(handoff_text) + "\n")


def agent_policy_live_evidence_probe_text():
    artifact_text = {
        "agent-policy-review.txt": (
            "agent-policy-review-fingerprint-observed-count 1\n"
            "multi-agent-handoff-review required"
        ),
        "agent-policy-capture-plan.json": (
            '"artifact_kind": "AIL-AgentTool-Policy-Capture-Plan"\n'
            '"human_approval_required": true'
        ),
        "agent-policy-import-demo-report.txt": (
            "policy-handoff-imported true\n"
            "policy-handoff-replayed true"
        ),
        "agent-policy-multi-agent-handoff-report.txt": (
            "multi-agent-execution-evidence deterministic-role-handoff"
        ),
    }
    sections = []
    for artifact_name, text in artifact_text.items():
        sections.append(
            "\n".join(
                [
                    f"artifact {artifact_name}",
                    f"fingerprint {fnv64(text)}",
                    "content-excerpt:",
                    f"----- begin {artifact_name} -----",
                    text,
                    f"----- end {artifact_name} -----",
                ]
            )
        )
    bundle = "\n\n".join(sections)
    return (
        "Review AgentTool policy handoff evidence for example-40 and return the "
        "AIL-AgentTool-Live-Reviewer-Handoff JSON envelope.\n\n"
        "Evidence bundle status: complete\n"
        "source-entry-id example-40\n"
        "proposed-entry-id example-40-policy\n"
        f"evidence-bundle-fingerprint {fnv64(bundle)}\n\n"
        f"{bundle}"
    )


class _CompletionHandler(BaseHTTPRequestHandler):
    response_text = ""
    response_payload = None
    response_for_payload = None
    requests = []

    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"])).decode()
        request_payload = json.loads(body)
        self.__class__.requests.append({"path": self.path, "body": request_payload})
        response_for_payload = self.__class__.response_for_payload
        if response_for_payload is not None:
            payload = response_for_payload(request_payload)
        else:
            payload = self.__class__.response_payload or {"content": self.__class__.response_text}
        encoded = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def log_message(self, _format, *args):
        return


class CaptureE2eTranscriptsTest(unittest.TestCase):
    def test_example_capture_script_aliases_are_documented_and_callable(self):
        for script in [
            "scripts/capture_example_transcripts.py",
            "scripts/capture_codex_example_transcript.py",
            "scripts/capture_example_batch.py",
        ]:
            output = subprocess.run(
                [sys.executable, script, "--help"],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(
                output.returncode,
                0,
                f"{script}\nstdout:\n{output.stdout}\nstderr:\n{output.stderr}",
            )

        examples_readme = (ROOT / "examples" / "README.md").read_text()
        corpus_readme = (ROOT / "docs" / "ail" / "corpus" / "README.md").read_text()
        for script in [
            "scripts/capture_example_transcripts.py",
            "scripts/capture_codex_example_transcript.py",
            "scripts/capture_example_batch.py",
        ]:
            self.assertIn(script, examples_readme)
        self.assertIn("scripts/capture_example_transcripts.py", corpus_readme)
        self.assertIn("scripts/capture_codex_example_transcript.py", corpus_readme)
        self.assertIn("scripts/capture_example_batch.py", corpus_readme)

    def test_prompt_llm_harness_review_accepts_complete_artifact_bundle(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-"))
        try:
            write_prompt_llm_review_fixture(artifact_dir)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("AIL-Prompt-LLM-Harness-Review:", review.stdout)
            self.assertIn("prompt-count 11", review.stdout)
            self.assertIn("content-nonempty-count 11", review.stdout)
            self.assertIn("prompt-envelope-valid-count 11", review.stdout)
            self.assertIn("prompt-envelope-artifact-count 8", review.stdout)
            self.assertIn("prompt-envelope-questions-count 3", review.stdout)
            self.assertIn("prompt-envelope-artifact-required-count 8", review.stdout)
            self.assertIn("prompt-envelope-questions-expected-count 3", review.stdout)
            self.assertIn("prompt-outcome-match-count 11", review.stdout)
            self.assertIn("review-result accepted", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_review_rejects_empty_content(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-empty-"))
        try:
            write_prompt_llm_review_fixture(
                artifact_dir, empty_content_for="requirements.system.md"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "empty content docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_review_rejects_raw_non_envelope_content(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-invalid-"))
        try:
            write_prompt_llm_review_fixture(
                artifact_dir, invalid_content_for="requirements.system.md"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "invalid prompt envelope docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
            self.assertIn("prompt-envelope-invalid-count 1", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_review_rejects_artifact_required_prompt_returning_questions(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-question-"))
        try:
            write_prompt_llm_review_fixture(
                artifact_dir, question_content_for="requirements.system.md"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "prompt outcome mismatch docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
            self.assertIn(
                "expected prompt-envelope-artifact got prompt-envelope-questions",
                review.stdout,
            )
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_review_rejects_mismatched_probe_metadata(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-probe-"))
        try:
            write_prompt_llm_review_fixture(
                artifact_dir, mismatched_probe_for="requirements.system.md"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "request probe_label mismatch docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
            self.assertIn(
                "request probe_fingerprint mismatch docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_review_rejects_mismatched_artifact_kind(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-kind-"))
        try:
            write_prompt_llm_review_fixture(
                artifact_dir, mismatched_artifact_kind_for="requirements.system.md"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "invalid prompt envelope docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
            self.assertIn(
                "prompt envelope artifact_kind must be AIL-Requirements",
                review.stdout,
            )
            self.assertIn("prompt-envelope-invalid-count 1", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_request_shapes_envelope_contract_and_json_mode(self):
        prompt_name = "requirements.system.md"
        _label, base_probe = _PROMPT_HARNESS.prompt_probe(prompt_name, None)
        probe_text = _PROMPT_HARNESS.render_user_probe(prompt_name, None)
        self.assertIn(base_probe.strip(), probe_text)
        self.assertIn('"artifact_kind": "AIL-Requirements"', probe_text)
        self.assertIn('"artifact_text": ""', probe_text)
        self.assertIn('"questions": ["..."]', probe_text)
        self.assertIn('"checker_handoff"', probe_text)
        self.assertIn('"must_check": true', probe_text)
        self.assertIn("questions must be an array of strings", probe_text)
        self.assertIn("Return JSON only", probe_text)

        body = _PROMPT_HARNESS.completion_body(
            "http://127.0.0.1:8080/v1/chat/completions",
            "system prompt",
            probe_text,
            64,
            None,
        )
        self.assertEqual(body["response_format"], {"type": "json_object"})
        self.assertEqual(body["messages"][1]["content"], probe_text)

    def test_story_llm_harness_review_accepts_complete_artifact_bundle(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-story-llm-review-"))
        try:
            write_story_llm_review_fixture(artifact_dir)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("AIL-Story-LLM-Harness-Review:", review.stdout)
            self.assertIn("story-id support-ticket-agent-story", review.stdout)
            self.assertIn("semantic-anchor-count 4", review.stdout)
            self.assertIn("model-check present", review.stdout)
            self.assertIn("model-check-model-count 1", review.stdout)
            self.assertIn("model-check-model-id test-story-model", review.stdout)
            self.assertIn("fingerprint-check-count 12", review.stdout)
            self.assertIn("story-llm-transcript-check-count 6", review.stdout)
            self.assertIn("story-prompt-envelope-valid-count 2", review.stdout)
            self.assertIn("story-prompt-envelope-artifact-count 2", review.stdout)
            self.assertIn("story-prompt-envelope-questions-count 0", review.stdout)
            self.assertIn("story-prompt-envelope-invalid-count 0", review.stdout)
            self.assertIn("agent-trace present", review.stdout)
            self.assertIn("review-result accepted", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_story_llm_harness_review_rejects_missing_agent_trace(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-story-llm-review-missing-agent-"))
        try:
            write_story_llm_review_fixture(artifact_dir, omit_agent_trace=True)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn("missing file", review.stdout)
            self.assertIn("agent-trace.txt", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_story_llm_harness_review_rejects_missing_agent_trace_fingerprint(self):
        artifact_dir = Path(
            tempfile.mkdtemp(prefix="ail-story-llm-review-missing-agent-fingerprint-")
        )
        try:
            write_story_llm_review_fixture(artifact_dir)
            (artifact_dir / "agent-trace.fingerprint.txt").unlink()
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn("missing file", review.stdout)
            self.assertIn("agent-trace.fingerprint.txt", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_story_llm_harness_review_rejects_question_only_artifact_transcript(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-story-llm-question-only-"))
        try:
            write_story_llm_review_fixture(artifact_dir)
            content = (
                json.dumps(
                    {
                        "artifact_kind": "AIL-Spec Canonical",
                        "artifact_text": "",
                        "questions": [
                            "Which exact acceptance criteria should be compiled?"
                        ],
                        "checker_handoff": {
                            "must_check": True,
                            "expected_profile": "Application",
                            "expected_features": [],
                        },
                    },
                    sort_keys=True,
                )
                + "\n"
            )
            write_text(artifact_dir / "llm/spec.content.txt", content)
            write_text(
                artifact_dir / "llm/spec.content.fingerprint.txt", fnv64(content) + "\n"
            )
            manifest_path = artifact_dir / "manifest.ail-story.txt"
            manifest = "\n".join(
                (
                    f"llm-spec-content llm/spec.content.txt {fnv64(content)}"
                    if line.startswith("llm-spec-content llm/spec.content.txt ")
                    else line
                )
                for line in manifest_path.read_text().splitlines()
            )
            manifest += "\n"
            write_text(manifest_path, manifest)
            write_text(
                artifact_dir / "manifest.ail-story.fingerprint.txt",
                fnv64(manifest) + "\n",
            )

            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "story prompt envelope spec must contain artifact_text",
                review.stdout,
            )
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_agent_policy_live_reviewer_harness_dry_run_lists_roles(self):
        dry_run = subprocess.run(
            [
                "python3",
                "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                "--dry-run",
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        self.assertEqual(
            dry_run.returncode,
            0,
            f"stdout:\n{dry_run.stdout}\nstderr:\n{dry_run.stderr}",
        )
        self.assertIn("AIL-Agent-Policy-Live-Reviewer-Harness:", dry_run.stdout)
        self.assertIn("role-count 5", dry_run.stdout)
        self.assertIn(
            "role requirements-writer contract examples/agents/codex-ail-requirements-writer.md",
            dry_run.stdout,
        )
        self.assertIn(
            "role agent-policy-reviewer contract examples/agents/codex-ail-agent-policy-reviewer.md",
            dry_run.stdout,
        )
        self.assertIn("artifact-kind AIL-AgentTool-Live-Reviewer-Handoff", dry_run.stdout)
        self.assertIn("model-check curl -sS http://inteligentia-pro-1:8080/v1/models", dry_run.stdout)

    def test_interactive_manual_v03_authoring_gate_dry_run_threads_fake_live_endpoint(self):
        artifact_root = Path(tempfile.mkdtemp(prefix="ail-manual-live-root-"))
        server = None
        try:
            _CompletionHandler.requests = []
            _CompletionHandler.response_payload = {"choices": [{"message": {"content": "{}"}}]}
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            endpoint = f"http://127.0.0.1:{server.server_port}/v1/chat/completions"

            def run_manual(chapter):
                return subprocess.run(
                    [
                        "python3",
                        "scripts/run_ail_interactive_manual.py",
                        "--chapter",
                        chapter,
                        "--dry-run",
                        "--include-live",
                        "--live-endpoint",
                        endpoint,
                        "--skip-model-check",
                        "--live-artifact-root",
                        str(artifact_root),
                    ],
                    cwd=ROOT,
                    text=True,
                    capture_output=True,
                )

            gate_dry_run = run_manual("v03-authoring-gate")
            self.assertEqual(
                gate_dry_run.returncode,
                0,
                f"stdout:\n{gate_dry_run.stdout}\nstderr:\n{gate_dry_run.stderr}",
            )
            self.assertIn(
                "python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode "
                f"--run-checks --include-live --live-endpoint {endpoint} "
                f"--skip-model-check --live-artifact-root {artifact_root}",
                gate_dry_run.stdout,
            )

            story_dry_run = run_manual("user-story-mode")
            prompt_dry_run = run_manual("prompt-interaction")
            agent_policy_dry_run = run_manual("agent-policy-import")
            for dry_run in [story_dry_run, prompt_dry_run, agent_policy_dry_run]:
                self.assertEqual(
                    dry_run.returncode,
                    0,
                    f"stdout:\n{dry_run.stdout}\nstderr:\n{dry_run.stderr}",
                )

            combined_stdout = "\n".join(
                [
                    gate_dry_run.stdout,
                    story_dry_run.stdout,
                    prompt_dry_run.stdout,
                    agent_policy_dry_run.stdout,
                ]
            )
            self.assertIn(
                "python3 scripts/run_v03_story_llm_harness.py "
                f"--endpoint {endpoint} --skip-model-check --artifact-dir {artifact_root / 'story-llm'}",
                combined_stdout,
            )
            self.assertIn(
                f"python3 scripts/run_v03_story_llm_harness.py --review-artifacts {artifact_root / 'story-llm'}",
                combined_stdout,
            )
            self.assertIn(
                "python3 scripts/run_v03_prompt_llm_harness.py "
                f"--endpoint {endpoint} --skip-model-check --artifact-dir {artifact_root / 'prompt-llm'}",
                combined_stdout,
            )
            self.assertIn(
                "python3 scripts/run_v03_agent_policy_live_reviewer_harness.py "
                f"--endpoint {endpoint} --skip-model-check --artifact-dir {artifact_root / 'agent-policy-live-review'}",
                combined_stdout,
            )
            self.assertIn(f"--llm-endpoint {endpoint}", combined_stdout)
            self.assertNotIn("http://inteligentia-pro-1:8080", combined_stdout)
            self.assertEqual(_CompletionHandler.requests, [])
        finally:
            _CompletionHandler.response_payload = None
            _CompletionHandler.response_for_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(artifact_root, ignore_errors=True)

    def test_interactive_manual_user_story_mode_run_checks_uses_fake_live_endpoint(self):
        artifact_root = Path(tempfile.mkdtemp(prefix="ail-manual-live-run-"))
        server = None
        try:
            spec_text = (ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md").read_text()
            requirements_text = (
                "AIL-Requirements:\n"
                "- The application manages support tickets.\n"
                "- Ticket fields include id, title, status, and secret internal notes.\n"
                "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n"
                "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n"
                "- The action guarantees closed tickets do not appear in the open queue.\n"
                "- The action records trace event TicketClosed.\n"
            )

            def story_response_for(payload):
                request_text = json.dumps(payload)
                if "spec-draft.system" in request_text or "AIL-Spec Canonical" in request_text:
                    content = json.dumps(
                        {
                            "artifact_kind": "AIL-Spec Canonical",
                            "artifact_text": spec_text,
                            "questions": [],
                            "checker_handoff": {
                                "must_check": True,
                                "expected_profile": "Application",
                                "expected_features": [],
                            },
                        },
                        sort_keys=True,
                    )
                else:
                    content = json.dumps(
                        {
                            "artifact_kind": "AIL-Requirements",
                            "artifact_text": requirements_text,
                            "questions": [],
                            "checker_handoff": {
                                "must_check": True,
                                "expected_profile": "Application",
                                "expected_features": [],
                            },
                        },
                        sort_keys=True,
                    )
                return {"choices": [{"message": {"content": content}}], "model": "test-chat-model"}

            _CompletionHandler.requests = []
            _CompletionHandler.response_payload = None
            _CompletionHandler.response_for_payload = story_response_for
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            endpoint = f"http://127.0.0.1:{server.server_port}/v1/chat/completions"

            run = subprocess.run(
                [
                    "python3",
                    "scripts/run_ail_interactive_manual.py",
                    "--chapter",
                    "user-story-mode",
                    "--run-checks",
                    "--include-live",
                    "--live-endpoint",
                    endpoint,
                    "--skip-model-check",
                    "--live-artifact-root",
                    str(artifact_root),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(
                run.returncode,
                0,
                f"stdout:\n{run.stdout}\nstderr:\n{run.stderr}",
            )
            self.assertIn("running run-story-mode-live", run.stdout)
            self.assertIn("running review-story-mode-live-artifacts", run.stdout)
            self.assertIn("story-llm-transcript-check-count 6", run.stdout)
            self.assertIn("story-prompt-envelope-artifact-count 2", run.stdout)
            self.assertIn("story-prompt-envelope-questions-count 0", run.stdout)
            self.assertIn("story-artifacts-preserved true", run.stdout)
            self.assertGreaterEqual(len(_CompletionHandler.requests), 4)
            self.assertTrue((artifact_root / "story-llm" / "story-llm-harness-report.txt").exists())
            self.assertTrue(
                (
                    artifact_root
                    / "story-promotion-capture-plan"
                    / "story-promotion-capture-plan.json"
                ).exists()
            )
        finally:
            _CompletionHandler.response_payload = None
            _CompletionHandler.response_for_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(artifact_root, ignore_errors=True)

    def test_agent_policy_live_reviewer_harness_review_accepts_complete_bundle(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-live-review-"))
        try:
            write_agent_policy_live_reviewer_fixture(artifact_dir)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("AIL-Agent-Policy-Live-Reviewer-Harness-Review:", review.stdout)
            self.assertIn("role-count 5", review.stdout)
            self.assertIn("reviewer-envelope-valid-count 5", review.stdout)
            self.assertIn("reviewer-envelope-invalid-count 0", review.stdout)
            self.assertIn("evidence-bundle-present-count 5", review.stdout)
            self.assertIn("reviewer-decision-accept-count 5", review.stdout)
            self.assertIn("reviewer-decision-needs-repair-count 0", review.stdout)
            self.assertIn("reviewer-decision-reject-count 0", review.stdout)
            self.assertIn("review-result accepted", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_agent_policy_live_reviewer_harness_review_blocks_needs_repair_decision(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-live-needs-repair-"))
        try:
            write_agent_policy_live_reviewer_fixture(
                artifact_dir, decision_for={"prompt-reviewer": "needs-repair"}
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("reviewer-envelope-valid-count 5", review.stdout)
            self.assertIn("reviewer-envelope-invalid-count 0", review.stdout)
            self.assertIn("evidence-bundle-present-count 5", review.stdout)
            self.assertIn("reviewer-decision-accept-count 4", review.stdout)
            self.assertIn("reviewer-decision-needs-repair-count 1", review.stdout)
            self.assertIn("reviewer-decision-reject-count 0", review.stdout)
            self.assertIn("review-result needs-repair", review.stdout)
            self.assertIn("reviewer decisions require repair before promotion", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_agent_policy_live_reviewer_harness_review_rejects_empty_content(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-live-empty-"))
        try:
            write_agent_policy_live_reviewer_fixture(
                artifact_dir, empty_content_for="prompt-reviewer"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn("empty content prompt-reviewer", review.stdout)
            self.assertIn("reviewer-envelope-invalid-count 1", review.stdout)
            self.assertIn("evidence-bundle-present-count 5", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_agent_policy_live_reviewer_requests_include_deterministic_evidence_bundle(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-live-requests-"))
        examples_artifacts = Path(tempfile.mkdtemp(prefix="ail-agent-policy-evidence-"))
        capture_plan_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-plan-"))
        import_work_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-import-"))
        server = None
        try:
            write_agent_policy_live_evidence_fixture(
                examples_artifacts, capture_plan_dir, import_work_dir
            )
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [
                    {
                        "message": {
                            "content": agent_policy_live_reviewer_envelope(
                                "requirements-writer"
                            )
                        }
                    }
                ],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            run = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
                    "--skip-model-check",
                    "--artifact-dir",
                    str(artifact_dir),
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--import-work-dir",
                    str(import_work_dir),
                    "--max-tokens",
                    "64",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(run.returncode, 0, f"stdout:\n{run.stdout}\nstderr:\n{run.stderr}")
            self.assertEqual(len(_CompletionHandler.requests), len(AGENT_POLICY_LIVE_ROLES))
            request = json.loads(
                (artifact_dir / "requests" / "requirements-writer.json").read_text()
            )
            user_probe = request["body"]["messages"][1]["content"]
            self.assertIn("Evidence bundle status: complete", user_probe)
            for artifact_name in [
                "agent-policy-review.txt",
                "agent-policy-capture-plan.json",
                "agent-policy-import-demo-report.txt",
                "agent-policy-multi-agent-handoff-report.txt",
            ]:
                self.assertIn(artifact_name, user_probe)
                self.assertIn(f"artifact {artifact_name}", user_probe)
                self.assertIn("fingerprint fnv64:", user_probe)
            self.assertIn("agent-policy-review-fingerprint-observed-count 1", user_probe)
            self.assertIn("policy-handoff-imported true", user_probe)
            self.assertIn("policy-handoff-replayed true", user_probe)
            self.assertIn(
                "multi-agent-execution-evidence deterministic-role-handoff",
                user_probe,
            )
            self.assertIn("evidence-bundle-fingerprint fnv64:", user_probe)
            self.assertIn(request["evidence_bundle_fingerprint"], user_probe)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(examples_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)
            shutil.rmtree(import_work_dir, ignore_errors=True)

    def test_story_promotion_capture_plan_writes_fingerprinted_plan(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-story-promotion-artifacts-"))
        plan_dir = Path(tempfile.mkdtemp(prefix="ail-story-promotion-plan-"))
        try:
            write_story_llm_review_fixture(artifact_dir)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(review.returncode, 0, review.stderr)

            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_promotion_capture_plan.py",
                    "--story-artifacts",
                    str(artifact_dir),
                    "--output-dir",
                    str(plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(
                plan.returncode,
                0,
                f"stdout:\n{plan.stdout}\nstderr:\n{plan.stderr}",
            )
            self.assertIn("AIL-Story-Promotion-Capture-Plan:", plan.stdout)
            self.assertIn("story-id support-ticket-agent-story", plan.stdout)
            self.assertIn("promotion-decision accepted-for-promotion", plan.stdout)
            self.assertIn("human-approval-required true", plan.stdout)
            self.assertIn("story-llm-transcript-check-count 6", plan.stdout)
            self.assertIn("story-prompt-envelope-valid-count 2", plan.stdout)
            self.assertIn("story-prompt-envelope-artifact-count 2", plan.stdout)
            self.assertIn("story-prompt-envelope-questions-count 0", plan.stdout)
            self.assertIn("story-model-check-model-id test-story-model", plan.stdout)
            self.assertIn("plan-json story-promotion-capture-plan.json", plan.stdout)

            plan_json_path = plan_dir / "story-promotion-capture-plan.json"
            plan_text_path = plan_dir / "story-promotion-capture-plan.txt"
            plan_fingerprint_path = (
                plan_dir / "story-promotion-capture-plan.fingerprint.txt"
            )
            plan_payload = json.loads(plan_json_path.read_text())
            plan_text_path.read_text()
            self.assertEqual(
                plan_payload["artifact_kind"], "AIL-Story-Promotion-Capture-Plan"
            )
            self.assertEqual(plan_payload["story_id"], "support-ticket-agent-story")
            self.assertEqual(plan_payload["status"], "plan-only")
            self.assertTrue(plan_payload["human_approval_required"])
            self.assertEqual(plan_payload["promotion_decision"], "accepted-for-promotion")
            self.assertEqual(plan_payload["story_llm_transcript_check_count"], 6)
            self.assertEqual(plan_payload["story_prompt_envelope_artifact_count"], 2)
            self.assertEqual(plan_payload["story_prompt_envelope_questions_count"], 0)
            self.assertEqual(plan_payload["story_model_check_model_id"], "test-story-model")
            self.assertEqual(
                plan_payload["story_model_check_fingerprint"],
                fnv64((artifact_dir / "model-check.json").read_text()),
            )
            self.assertEqual(
                plan_fingerprint_path.read_text().strip(),
                fnv64(plan_json_path.read_text()),
            )
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(plan_dir, ignore_errors=True)

    def test_story_promotion_capture_plan_rejects_missing_story_fingerprint(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-story-promotion-missing-fp-"))
        plan_dir = Path(tempfile.mkdtemp(prefix="ail-story-promotion-missing-fp-plan-"))
        try:
            write_story_llm_review_fixture(artifact_dir)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(review.returncode, 0, review.stderr)
            (artifact_dir / "agent-trace.fingerprint.txt").unlink()

            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_promotion_capture_plan.py",
                    "--story-artifacts",
                    str(artifact_dir),
                    "--output-dir",
                    str(plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                plan.returncode,
                0,
                f"stdout:\n{plan.stdout}\nstderr:\n{plan.stderr}",
            )
            self.assertIn("missing file", plan.stderr)
            self.assertIn("agent-trace.fingerprint.txt", plan.stderr)
            self.assertFalse((plan_dir / "story-promotion-capture-plan.json").exists())
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(plan_dir, ignore_errors=True)

    def test_capture_replaces_seed_entry_with_live_llm_transcript(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-capture-artifacts-"))
        server = None
        try:
            _CompletionHandler.requests = []
            _CompletionHandler.response_payload = None
            _CompletionHandler.response_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_transcripts.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-30",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/completion",
                    "--endpoint-label",
                    "test-live-endpoint",
                    "--executor-label",
                    "test-live-model",
                    "--semantic-task",
                    "support-ticket-live-capture-30",
                    "--prompt",
                    "Produce the Support Ticket AIL-Spec for live capture replay.",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            self.assertIn("semantic-task: support-ticket-live-capture-30", examples)
            self.assertIn("capture-origin: live-llm", examples)
            self.assertIn("executor-label: test-live-model", examples)
            self.assertIn("endpoint-label: test-live-endpoint", examples)

            request = json.loads((output_dir / "requests" / "example-30.json").read_text())
            self.assertEqual(request["endpoint"], f"http://127.0.0.1:{server.server_port}/completion")
            self.assertEqual(request["body"]["temperature"], 0.0)
            self.assertIn("Support Ticket", request["body"]["prompt"])
            self.assertEqual(_CompletionHandler.requests[0]["path"], "/completion")

            response = json.loads((output_dir / "responses" / "example-30.json").read_text())
            self.assertIn("content", response)
            self.assertIn("AIL-Spec", response["content"])

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertNotIn("capture-origin-count deterministic-seed", report)
            self.assertIn("capture-origin-count live-llm 5", report)
            self.assertIn("capture-origin-count live-codex 117", report)
            self.assertIn(
                "entry example-30 source "
                + str(output_dir / "examples.md")
                + " semantic-task support-ticket-live-capture-30 executor-family llm-http capture-origin live-llm target linux-x86_64-elf",
                report,
            )
        finally:
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_capture_chat_completion_transcript_replays_offline(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-chat-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-chat-capture-artifacts-"))
        server = None
        try:
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [{"message": {"content": spec_text}}],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_transcripts.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-32",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
                    "--endpoint-label",
                    "test-chat-endpoint",
                    "--executor-label",
                    "test-chat-model",
                    "--semantic-task",
                    "support-ticket-live-chat-capture-32",
                    "--prompt",
                    "Produce the Support Ticket AIL-Spec for live chat capture replay.",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            request = json.loads((output_dir / "requests" / "example-32.json").read_text())
            self.assertEqual(
                request["endpoint"],
                f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
            )
            self.assertIn("messages", request["body"])
            self.assertEqual(request["body"]["messages"][0]["role"], "user")
            self.assertFalse(request["body"]["chat_template_kwargs"]["enable_thinking"])
            self.assertEqual(_CompletionHandler.requests[0]["path"], "/v1/chat/completions")

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertIn("capture-origin-count live-llm 4", report)
            self.assertIn("capture-origin-count live-codex 118", report)
            self.assertIn("entry example-32", report)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_capture_uses_schema_input_json_file_for_spec_draft_prompt(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-input-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-input-capture-artifacts-"))
        input_json = Path(tempfile.mkdtemp(prefix="ail-examples-live-input-json-")) / "input.json"
        task_prompt = Path(tempfile.mkdtemp(prefix="ail-examples-live-task-prompt-")) / "task.txt"
        server = None
        try:
            input_payload = {
                "profile": "Application",
                "package_manifest": (
                    ROOT / "examples" / "support_ticket.ail" / "ail-package.md"
                ).read_text(),
                "required_features": ["things", "actions", "failures", "guarantees", "traces"],
                "requirements": (
                    "AIL-Requirements:\n"
                    "- The application manages customer support tickets.\n"
                    "- The CloseTicket action is performed by a support agent.\n"
                    "- CloseTicket requires the ticket to exist and status not to be Closed.\n"
                    "- CloseTicket changes ticket status to Closed.\n"
                    "- CloseTicket records trace event TicketClosed.\n"
                ),
            }
            input_json.write_text(json.dumps(input_payload, indent=2, sort_keys=True) + "\n")
            task_prompt.write_text(
                "Draft the canonical Support Ticket AIL-Spec from the input JSON.\n"
            )
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [{"message": {"content": spec_text}}],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_transcripts.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-32",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
                    "--endpoint-label",
                    "test-chat-endpoint",
                    "--executor-label",
                    "test-chat-model",
                    "--semantic-task",
                    "support-ticket-live-input-capture-32",
                    "--prompt-file",
                    str(task_prompt),
                    "--input-json-file",
                    str(input_json),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            request = json.loads((output_dir / "requests" / "example-32.json").read_text())
            prompt = request["body"]["messages"][0]["content"]
            self.assertIn("INPUT JSON:", prompt)
            self.assertIn('"requirements"', prompt)
            self.assertIn("CloseTicket records trace event TicketClosed", prompt)
            self.assertIn("Draft the canonical Support Ticket AIL-Spec", prompt)
            self.assertNotIn("USER REQUEST:", prompt)

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(input_json.parent, ignore_errors=True)
            shutil.rmtree(task_prompt.parent, ignore_errors=True)

    def test_capture_codex_transcript_imports_live_codex_entry(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-codex-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-codex-capture-artifacts-"))
        transcript_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-codex-transcript-"))
        try:
            request_json = transcript_dir / "request.json"
            response_json = transcript_dir / "response.json"
            request_json.write_text(
                json.dumps(
                    {
                        "agent": "codex-ail-spec-writer",
                        "model": "codex-test-model",
                        "task": "Draft and validate the Support Ticket AIL-Spec.",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            response_json.write_text(
                json.dumps(
                    {
                        "content": (
                            ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
                        ).read_text(),
                        "model": "codex-test-model",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_codex_example_transcript.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-99",
                    "--executor-label",
                    "codex-ail-spec-writer-test",
                    "--semantic-task",
                    "support-ticket-live-codex-capture-99",
                    "--request-json-file",
                    str(request_json),
                    "--response-json-file",
                    str(response_json),
                    "--checker-result",
                    "accepted",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            self.assertIn("semantic-task: support-ticket-live-codex-capture-99", examples)
            self.assertIn("executor-family: codex-skill-agent", examples)
            self.assertIn("capture-origin: live-codex", examples)
            self.assertIn("executor-label: codex-ail-spec-writer-test", examples)
            example_99 = examples.split("## Example: example-99", 1)[1]
            self.assertNotIn("endpoint-label:", example_99)

            request = json.loads((output_dir / "requests" / "example-99.json").read_text())
            self.assertEqual(request["agent"], "codex-ail-spec-writer")
            response = json.loads((output_dir / "responses" / "example-99.json").read_text())
            self.assertIn("AIL-Spec", response["content"])

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertNotIn("capture-origin-count deterministic-seed", report)
            self.assertIn("capture-origin-count live-llm 4", report)
            self.assertIn("capture-origin-count live-codex 118", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("entry example-99", report)
            self.assertIn("capture-origin live-codex", report)
        finally:
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(transcript_dir, ignore_errors=True)

    def test_batch_capture_preserves_previous_live_entries(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-batch-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-batch-capture-artifacts-"))
        transcript_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-batch-transcript-"))
        server = None
        try:
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [{"message": {"content": spec_text}}],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            codex_request = transcript_dir / "codex-request.json"
            codex_response = transcript_dir / "codex-response.json"
            batch_plan = transcript_dir / "batch-plan.json"
            codex_request.write_text(
                json.dumps(
                    {
                        "agent_contract": (
                            "examples/agents/codex-ail-spec-writer.md"
                        ),
                        "agent_contract_version": "0.1.0",
                        "executor_label": "codex-ail-spec-writer-test",
                        "task": "Draft canonical Support Ticket AIL-Spec.",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            codex_response.write_text(
                json.dumps({"artifact_text": spec_text, "model": "codex-test-model"})
                + "\n"
            )
            batch_plan.write_text(
                json.dumps(
                    {
                        "entries": [
                            {
                                "entry_id": "example-30",
                                "executor_family": "llm-http",
                                "endpoint": (
                                    f"http://127.0.0.1:{server.server_port}"
                                    "/v1/chat/completions"
                                ),
                                "endpoint_label": "test-chat-endpoint",
                                "executor_label": "test-chat-model",
                                "semantic_task": "support-ticket-live-batch-30",
                                "prompt": (
                                    "Produce the Support Ticket AIL-Spec for live "
                                    "batch capture replay."
                                ),
                            },
                            {
                                "entry_id": "example-99",
                                "executor_family": "codex-skill-agent",
                                "executor_label": "codex-ail-spec-writer-test",
                                "semantic_task": "support-ticket-live-codex-batch-99",
                                "request_json_file": str(codex_request),
                                "response_json_file": str(codex_response),
                                "checker_result": "accepted",
                            },
                        ]
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_batch.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--plan-json",
                    str(batch_plan),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            self.assertIn("semantic-task: support-ticket-live-batch-30", examples)
            self.assertIn("semantic-task: support-ticket-live-codex-batch-99", examples)
            self.assertIn("capture-origin: live-codex", examples)
            self.assertIn("semantic-task: support-ticket-live-spec-input-32", examples)
            self.assertEqual(_CompletionHandler.requests[0]["path"], "/v1/chat/completions")

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertNotIn("capture-origin-count deterministic-seed", report)
            self.assertIn("capture-origin-count live-llm 5", report)
            self.assertIn("capture-origin-count live-codex 117", report)
            self.assertIn("entry example-30", report)
            self.assertIn("entry example-32", report)
            self.assertIn("entry example-99", report)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(transcript_dir, ignore_errors=True)

    def test_batch_capture_appends_repair_promotion_entry(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-repair-promotion-import-"))
        artifact_dir = Path(
            tempfile.mkdtemp(prefix="ail-examples-repair-promotion-import-artifacts-")
        )
        transcript_dir = Path(
            tempfile.mkdtemp(prefix="ail-examples-repair-promotion-import-transcript-")
        )
        try:
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            codex_request = transcript_dir / "repair-request.json"
            codex_response = transcript_dir / "repair-response.json"
            capture_plan = transcript_dir / "repair-promotion-capture-plan.json"
            batch_plan = transcript_dir / "batch-plan.json"
            codex_request.write_text(
                json.dumps(
                    {
                        "agent_contract": (
                            "examples/agents/codex-ail-repair-promotion-reviewer.md"
                        ),
                        "executor_label": "codex-ail-repair-promotion-reviewer-test",
                        "source_entry_id": "example-99",
                        "task": "Approve the repaired Support Ticket spec.",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            codex_response.write_text(
                json.dumps({"artifact_text": spec_text, "model": "codex-test-model"})
                + "\n"
            )
            capture_plan_text = (
                json.dumps(
                    {
                        "artifact_kind": "AIL-Repair-Promotion-Capture-Plan",
                        "batch_capture_script": "scripts/capture_example_batch.py",
                        "checker_result": "rejected-to-repaired",
                        "expected_diagnostic_removed": True,
                        "human_approval_required": True,
                        "must_supply_request_response_json": True,
                        "preserve_rejected_entry": True,
                        "promotion_decision": "accepted-for-promotion",
                        "proposed_entry_id": "example-99-repaired",
                        "semantic_anchor_missing_count": 0,
                        "source_entry_id": "example-99",
                        "status": "plan-only",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            capture_plan.write_text(capture_plan_text)
            capture_plan.with_name("repair-promotion-capture-plan.fingerprint.txt").write_text(
                fnv64(capture_plan_text) + "\n"
            )
            batch_plan.write_text(
                json.dumps(
                    {
                        "entries": [
                            {
                                "entry_id": "example-99-repaired",
                                "source_entry_id": "example-99",
                                "executor_family": "codex-skill-agent",
                                "executor_label": (
                                    "codex-ail-repair-promotion-reviewer-test"
                                ),
                                "semantic_task": "support-ticket-repair-promoted-99",
                                "request_json_file": str(codex_request),
                                "response_json_file": str(codex_response),
                                "checker_result": "accepted",
                                "repair_promotion_capture_plan_json": str(capture_plan),
                            }
                        ]
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_batch.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--plan-json",
                    str(batch_plan),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            source_section = examples.split("## Example: example-99", 1)[1].split(
                "## Example:", 1
            )[0]
            promoted_section = examples.split("## Example: example-99-repaired", 1)[1]
            self.assertIn("checker-result: rejected", source_section)
            self.assertIn("expected-diagnostic: AIL001", source_section)
            self.assertIn("semantic-task: support-ticket-repair-promoted-99", promoted_section)
            self.assertIn("checker-result: accepted", promoted_section)
            self.assertIn("capture-origin: live-codex", promoted_section)
            self.assertIn(
                "executor-label: codex-ail-repair-promotion-reviewer-test",
                promoted_section,
            )
            self.assertIn("story-file: stories/example-99-repaired.md", promoted_section)
            self.assertNotIn("expected-diagnostic:", promoted_section)
            self.assertNotIn("failure-taxonomy:", promoted_section)
            self.assertTrue((output_dir / "stories" / "example-99-repaired.md").exists())
            self.assertTrue((output_dir / "requests" / "example-99-repaired.json").exists())
            self.assertTrue((output_dir / "responses" / "example-99-repaired.json").exists())

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertIn("entry-count 123", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("checker-result-count rejected 9", report)
            self.assertIn("entry example-99 ", report)
            self.assertIn("entry example-99-repaired ", report)
        finally:
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(transcript_dir, ignore_errors=True)

    def test_batch_capture_appends_story_promotion_entry(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-story-promotion-import-"))
        artifact_dir = Path(
            tempfile.mkdtemp(prefix="ail-examples-story-promotion-import-artifacts-")
        )
        story_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-examples-story-promotion-artifacts-")
        )
        capture_plan_dir = Path(
            tempfile.mkdtemp(prefix="ail-examples-story-promotion-plan-")
        )
        transcript_dir = Path(
            tempfile.mkdtemp(prefix="ail-examples-story-promotion-transcript-")
        )
        try:
            write_story_llm_review_fixture(story_artifacts)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(story_artifacts),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(review.returncode, 0, review.stderr)
            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_promotion_capture_plan.py",
                    "--story-artifacts",
                    str(story_artifacts),
                    "--output-dir",
                    str(capture_plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)

            codex_request = transcript_dir / "story-request.json"
            codex_response = transcript_dir / "story-response.json"
            batch_plan = transcript_dir / "batch-plan.json"
            codex_request.write_text(
                json.dumps(
                    {
                        "agent_contract": "examples/agents/codex-ail-prompt-reviewer.md",
                        "executor_label": "codex-ail-prompt-reviewer-story-test",
                        "source_entry_id": "example-30",
                        "task": "Approve the reviewed User Story mode artifact for corpus promotion.",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            codex_response.write_text(
                json.dumps(
                    {
                        "artifact_text": (
                            story_artifacts / "accepted.ail-spec.md"
                        ).read_text(),
                        "model": "codex-story-promotion-test-model",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            batch_plan.write_text(
                json.dumps(
                    {
                        "entries": [
                            {
                                "entry_id": "example-30-story",
                                "source_entry_id": "example-30",
                                "executor_family": "codex-skill-agent",
                                "executor_label": "codex-ail-prompt-reviewer-story-test",
                                "semantic_task": "support-ticket-story-promoted-30",
                                "request_json_file": str(codex_request),
                                "response_json_file": str(codex_response),
                                "checker_result": "accepted",
                                "story_promotion_capture_plan_json": str(
                                    capture_plan_dir / "story-promotion-capture-plan.json"
                                ),
                            }
                        ]
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_batch.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--plan-json",
                    str(batch_plan),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            source_section = examples.split("## Example: example-30", 1)[1].split(
                "## Example:", 1
            )[0]
            promoted_section = examples.split("## Example: example-30-story", 1)[1]
            self.assertIn("checker-result: accepted", source_section)
            self.assertIn("semantic-task: support-ticket-story-promoted-30", promoted_section)
            self.assertIn("user-story-id: support-ticket-agent-story", promoted_section)
            self.assertIn("story-file: stories/example-30-story.md", promoted_section)
            self.assertIn("story-evidence: vm-trace", promoted_section)
            self.assertIn("capture-origin: live-codex", promoted_section)
            self.assertIn(
                "executor-label: codex-ail-prompt-reviewer-story-test",
                promoted_section,
            )
            story_file = output_dir / "stories" / "example-30-story.md"
            self.assertTrue(story_file.exists())
            self.assertIn(
                "semantic-anchors: Support Tickets; Close ticket; TicketClosed; toolchain agent",
                story_file.read_text(),
            )
            self.assertTrue((output_dir / "requests" / "example-30-story.json").exists())
            self.assertTrue((output_dir / "responses" / "example-30-story.json").exists())

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertIn("entry-count 123", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("checker-result-count rejected 9", report)
            self.assertIn("entry example-30 ", report)
            self.assertIn("entry example-30-story ", report)
        finally:
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(story_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)
            shutil.rmtree(transcript_dir, ignore_errors=True)

    def test_repair_promotion_import_demo_replays_promoted_entry(self):
        work_dir = Path(tempfile.mkdtemp(prefix="ail-repair-promotion-demo-work-"))
        examples_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-repair-promotion-demo-artifacts-")
        )
        capture_plan_dir = Path(
            tempfile.mkdtemp(prefix="ail-repair-promotion-demo-plan-")
        )
        output_corpus = Path(tempfile.mkdtemp(prefix="ail-repair-promotion-demo-corpus-"))
        output_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-repair-promotion-demo-output-artifacts-")
        )
        shutil.rmtree(output_corpus)
        shutil.rmtree(output_artifacts)
        try:
            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    str(examples_artifacts),
                    "--release-evidence",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_repair_promotion_capture_plan.py",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--entry-id",
                    "example-99",
                    "--output-dir",
                    str(capture_plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)
            demo = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_repair_promotion_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--source-entry-id",
                    "example-99",
                    "--work-dir",
                    str(work_dir),
                    "--output-corpus",
                    str(output_corpus),
                    "--output-artifacts",
                    str(output_artifacts),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(demo.returncode, 0, demo.stderr)
            report = (work_dir / "repair-promotion-import-demo-report.txt").read_text()
            report_fingerprint = (
                work_dir / "repair-promotion-import-demo-report.fingerprint.txt"
            ).read_text()
            self.assertIn("AIL-Repair-Promotion-Import-Demo:", report)
            self.assertIn("source-entry-id example-99", report)
            self.assertIn("proposed-entry-id example-99-repaired", report)
            self.assertIn("source-preserved true", report)
            self.assertIn("proposed-accepted true", report)
            self.assertIn("entry-count 123", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("checker-result-count rejected 9", report)
            self.assertEqual(report_fingerprint.strip(), fnv64(report))

            examples = (output_corpus / "examples.md").read_text()
            source_section = examples.split("## Example: example-99", 1)[1].split(
                "## Example:", 1
            )[0]
            promoted_section = examples.split("## Example: example-99-repaired", 1)[1]
            self.assertIn("checker-result: rejected", source_section)
            self.assertIn("checker-result: accepted", promoted_section)
            self.assertTrue(
                (output_artifacts / "examples" / "example-99-repaired" / "checked.ail-core.txt")
                .exists()
            )
        finally:
            shutil.rmtree(work_dir, ignore_errors=True)
            shutil.rmtree(examples_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)
            shutil.rmtree(output_corpus, ignore_errors=True)
            shutil.rmtree(output_artifacts, ignore_errors=True)

    def test_story_promotion_import_demo_replays_promoted_entry(self):
        work_dir = Path(tempfile.mkdtemp(prefix="ail-story-promotion-demo-work-"))
        story_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-story-promotion-demo-artifacts-")
        )
        capture_plan_dir = Path(
            tempfile.mkdtemp(prefix="ail-story-promotion-demo-plan-")
        )
        output_corpus = Path(tempfile.mkdtemp(prefix="ail-story-promotion-demo-corpus-"))
        output_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-story-promotion-demo-output-artifacts-")
        )
        shutil.rmtree(output_corpus)
        shutil.rmtree(output_artifacts)
        try:
            write_story_llm_review_fixture(story_artifacts)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    str(story_artifacts),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(review.returncode, 0, review.stderr)
            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_promotion_capture_plan.py",
                    "--story-artifacts",
                    str(story_artifacts),
                    "--output-dir",
                    str(capture_plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)
            demo = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_story_promotion_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--story-artifacts",
                    str(story_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--source-entry-id",
                    "example-30",
                    "--proposed-entry-id",
                    "example-30-story",
                    "--work-dir",
                    str(work_dir),
                    "--output-corpus",
                    str(output_corpus),
                    "--output-artifacts",
                    str(output_artifacts),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(demo.returncode, 0, demo.stderr)
            report = (work_dir / "story-promotion-import-demo-report.txt").read_text()
            report_fingerprint = (
                work_dir / "story-promotion-import-demo-report.fingerprint.txt"
            ).read_text()
            self.assertIn("AIL-Story-Promotion-Import-Demo:", report)
            self.assertIn("story-id support-ticket-agent-story", report)
            self.assertIn("source-entry-id example-30", report)
            self.assertIn("proposed-entry-id example-30-story", report)
            self.assertIn("source-preserved true", report)
            self.assertIn("proposed-accepted true", report)
            self.assertIn("story-artifacts-preserved true", report)
            self.assertIn("entry-count 123", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("checker-result-count rejected 9", report)
            self.assertEqual(report_fingerprint.strip(), fnv64(report))

            examples = (output_corpus / "examples.md").read_text()
            source_section = examples.split("## Example: example-30", 1)[1].split(
                "## Example:", 1
            )[0]
            promoted_section = examples.split("## Example: example-30-story", 1)[1]
            self.assertIn("checker-result: accepted", source_section)
            self.assertIn("checker-result: accepted", promoted_section)
            self.assertTrue(
                (output_artifacts / "examples" / "example-30-story" / "checked.ail-core.txt")
                .exists()
            )
        finally:
            shutil.rmtree(work_dir, ignore_errors=True)
            shutil.rmtree(story_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)
            shutil.rmtree(output_corpus, ignore_errors=True)
            shutil.rmtree(output_artifacts, ignore_errors=True)

    def test_ui_patch_import_demo_replays_promoted_entry(self):
        work_dir = Path(tempfile.mkdtemp(prefix="ail-ui-patch-demo-work-"))
        examples_artifacts = Path(tempfile.mkdtemp(prefix="ail-ui-patch-demo-artifacts-"))
        capture_plan_dir = Path(tempfile.mkdtemp(prefix="ail-ui-patch-demo-plan-"))
        output_corpus = Path(tempfile.mkdtemp(prefix="ail-ui-patch-demo-corpus-"))
        output_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-ui-patch-demo-output-artifacts-")
        )
        shutil.rmtree(output_corpus)
        shutil.rmtree(output_artifacts)
        try:
            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    str(examples_artifacts),
                    "--release-evidence",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_ui_patch_capture_plan.py",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--entry-id",
                    "example-108",
                    "--output-dir",
                    str(capture_plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)
            demo = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_ui_patch_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--source-entry-id",
                    "example-108",
                    "--work-dir",
                    str(work_dir),
                    "--output-corpus",
                    str(output_corpus),
                    "--output-artifacts",
                    str(output_artifacts),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(demo.returncode, 0, demo.stderr)
            report = (work_dir / "ui-patch-import-demo-report.txt").read_text()
            report_fingerprint = (
                work_dir / "ui-patch-import-demo-report.fingerprint.txt"
            ).read_text()
            self.assertIn("AIL-UI-Patch-Import-Demo:", report)
            self.assertIn("source-entry-id example-108", report)
            self.assertIn("proposed-entry-id example-108-ui-patch", report)
            self.assertIn("source-preserved true", report)
            self.assertIn("proposed-accepted true", report)
            self.assertIn("ui-review-patch-fingerprint-preserved true", report)
            self.assertIn("checked-core-fingerprint-preserved true", report)
            self.assertIn("flow-edit-applied true", report)
            self.assertIn("entry-count 123", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("checker-result-count rejected 9", report)
            self.assertEqual(report_fingerprint.strip(), fnv64(report))

            examples = (output_corpus / "examples.md").read_text()
            source_section = examples.split("## Example: example-108", 1)[1].split(
                "## Example:", 1
            )[0]
            promoted_section = examples.split("## Example: example-108-ui-patch", 1)[1]
            self.assertIn("checker-result: accepted", source_section)
            self.assertIn("checker-result: accepted", promoted_section)
            patched_core = (
                output_artifacts
                / "examples"
                / "example-108-ui-patch"
                / "checked.ail-core.txt"
            )
            self.assertTrue(patched_core.exists())
            self.assertIn("node Field Ticket.reviewStatus", patched_core.read_text())

            runtime_state = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_ui_patch_runtime_state_check.py",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--import-work-dir",
                    str(work_dir),
                    "--output-artifacts",
                    str(output_artifacts),
                    "--source-entry-id",
                    "example-108",
                    "--output-dir",
                    str(work_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(runtime_state.returncode, 0, runtime_state.stderr)
            runtime_report = (
                work_dir / "ui-patch-runtime-state-check-report.txt"
            ).read_text()
            runtime_report_fingerprint = (
                work_dir / "ui-patch-runtime-state-check-report.fingerprint.txt"
            ).read_text()
            self.assertIn("AIL-UI-Patch-Runtime-State-Check:", runtime_report)
            self.assertIn("source-entry-id example-108", runtime_report)
            self.assertIn("proposed-entry-id example-108-ui-patch", runtime_report)
            self.assertIn("visual-regression-baseline ui-review.txt", runtime_report)
            self.assertIn("visual-regression-patch ui-review-patch.txt", runtime_report)
            self.assertIn(
                "visual-regression-fingerprint-preserved true",
                runtime_report,
            )
            self.assertIn("runtime-ui-state-check target-report", runtime_report)
            self.assertIn("runtime-ui-state-anchor Ticket.reviewStatus", runtime_report)
            self.assertIn("flow-edit-applied true", runtime_report)
            self.assertIn("patched-core-replayed true", runtime_report)
            self.assertEqual(runtime_report_fingerprint.strip(), fnv64(runtime_report))
        finally:
            shutil.rmtree(work_dir, ignore_errors=True)
            shutil.rmtree(examples_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)
            shutil.rmtree(output_corpus, ignore_errors=True)
            shutil.rmtree(output_artifacts, ignore_errors=True)

    def test_agent_policy_import_demo_replays_promoted_entry(self):
        work_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-demo-work-"))
        examples_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-agent-policy-demo-artifacts-")
        )
        capture_plan_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-demo-plan-"))
        output_corpus = Path(tempfile.mkdtemp(prefix="ail-agent-policy-demo-corpus-"))
        output_artifacts = Path(
            tempfile.mkdtemp(prefix="ail-agent-policy-demo-output-artifacts-")
        )
        shutil.rmtree(output_corpus)
        shutil.rmtree(output_artifacts)
        try:
            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    str(examples_artifacts),
                    "--release-evidence",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_capture_plan.py",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--entry-id",
                    "example-40",
                    "--output-dir",
                    str(capture_plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)
            demo = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--source-entry-id",
                    "example-40",
                    "--work-dir",
                    str(work_dir),
                    "--output-corpus",
                    str(output_corpus),
                    "--output-artifacts",
                    str(output_artifacts),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(demo.returncode, 0, demo.stderr)
            report = (work_dir / "agent-policy-import-demo-report.txt").read_text()
            report_fingerprint = (
                work_dir / "agent-policy-import-demo-report.fingerprint.txt"
            ).read_text()
            self.assertIn("AIL-Agent-Policy-Import-Demo:", report)
            self.assertIn("source-entry-id example-40", report)
            self.assertIn("proposed-entry-id example-40-policy", report)
            self.assertIn("source-preserved true", report)
            self.assertIn("proposed-accepted true", report)
            self.assertIn("agent-policy-review-fingerprint-preserved true", report)
            self.assertIn("checked-core-fingerprint-preserved true", report)
            self.assertIn("policy-handoff-imported true", report)
            self.assertIn("entry-count 123", report)
            self.assertIn("checker-result-count accepted 114", report)
            self.assertIn("checker-result-count rejected 9", report)
            self.assertEqual(report_fingerprint.strip(), fnv64(report))

            examples = (output_corpus / "examples.md").read_text()
            source_section = examples.split("## Example: example-40", 1)[1].split(
                "## Example:", 1
            )[0]
            promoted_section = examples.split("## Example: example-40-policy", 1)[1]
            self.assertIn("checker-result: accepted", source_section)
            self.assertIn("checker-result: accepted", promoted_section)
            checked_core = (
                output_artifacts
                / "examples"
                / "example-40-policy"
                / "checked.ail-core.txt"
            )
            self.assertTrue(checked_core.exists())
            self.assertIn("node Trace PolicyHandoffApprovedScenario40", checked_core.read_text())
            handoff = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_multi_agent_handoff.py",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--capture-plan-dir",
                    str(capture_plan_dir),
                    "--import-work-dir",
                    str(work_dir),
                    "--output-artifacts",
                    str(output_artifacts),
                    "--source-entry-id",
                    "example-40",
                    "--output-dir",
                    str(work_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(handoff.returncode, 0, handoff.stderr)
            handoff_report = (
                work_dir / "agent-policy-multi-agent-handoff-report.txt"
            ).read_text()
            handoff_fingerprint = (
                work_dir / "agent-policy-multi-agent-handoff-report.fingerprint.txt"
            ).read_text()
            self.assertIn("AIL-Agent-Policy-Multi-Agent-Handoff:", handoff_report)
            self.assertIn("source-entry-id example-40", handoff_report)
            self.assertIn("proposed-entry-id example-40-policy", handoff_report)
            self.assertIn("separate-reviewer-role-count 5", handoff_report)
            self.assertIn("role requirements-writer contract codex-ail-requirements-writer", handoff_report)
            self.assertIn("role spec-writer contract codex-ail-spec-writer", handoff_report)
            self.assertIn("role diagnostic-repairer contract codex-ail-diagnostic-repairer", handoff_report)
            self.assertIn("role prompt-reviewer contract codex-ail-prompt-reviewer", handoff_report)
            self.assertIn("role agent-policy-reviewer contract codex-ail-agent-policy-reviewer", handoff_report)
            self.assertIn("agent-contracts-result accepted", handoff_report)
            self.assertIn("source-preserved true", handoff_report)
            self.assertIn("proposed-accepted true", handoff_report)
            self.assertIn("policy-handoff-imported true", handoff_report)
            self.assertIn("policy-handoff-replayed true", handoff_report)
            self.assertIn(
                "multi-agent-execution-evidence deterministic-role-handoff",
                handoff_report,
            )
            self.assertEqual(handoff_fingerprint.strip(), fnv64(handoff_report))
        finally:
            shutil.rmtree(work_dir, ignore_errors=True)
            shutil.rmtree(examples_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)
            shutil.rmtree(output_corpus, ignore_errors=True)
            shutil.rmtree(output_artifacts, ignore_errors=True)

    def test_agent_policy_capture_plan_rejects_stale_handoff_roles(self):
        examples_artifacts = Path(tempfile.mkdtemp(prefix="ail-agent-policy-stale-roles-"))
        capture_plan_dir = Path(tempfile.mkdtemp(prefix="ail-agent-policy-stale-plan-"))
        try:
            entry_dir = examples_artifacts / "examples" / "example-40"
            checked_core = "node Tool RefundCustomerPayment\n"
            bytecode = '{"artifact_kind":"AIL-Bytecode"}\n'
            review_text = "\n".join(
                [
                    "AIL-Agent-Policy-Review:",
                    "entry example-40",
                    "semantic-task refund-tool-live-codex-interview-40",
                    "profile AgentTool",
                    "program-domain agent-tool",
                    "executor-family codex-skill-agent",
                    "executor-label codex-ail-spec-writer",
                    "prompt-file docs/ail/prompts/interview.system.md",
                    "interacts-with payment.provider,policy.engine,audit.log",
                    "agent-policy-review-artifact deterministic-text",
                    "multi-agent-handoff-review required",
                    "agent-contract-check ail-agent-contracts examples/agents",
                    "handoff-roles requirements-writer,spec-writer,diagnostic-repairer,prompt-reviewer,repair-promotion-reviewer",
                    "tool-permission-review required",
                    "tool-approval-review required",
                    "external-call-review required",
                    "secret-redaction-review required",
                    "audit-trace-review required",
                    "human-approval-required true",
                    "policy-import-status proposed-only",
                    "runtime-evidence bytecode",
                    f"checked-core-fingerprint {fnv64(checked_core)}",
                    f"bytecode-fingerprint {fnv64(bytecode)}",
                    "",
                ]
            )
            write_text(entry_dir / "checked.ail-core.txt", checked_core)
            write_text(entry_dir / "artifact.ailbc.json", bytecode)
            write_text(entry_dir / "agent-policy-review.txt", review_text)
            write_text(
                entry_dir / "agent-policy-review.fingerprint.txt",
                fnv64(review_text) + "\n",
            )
            manifest_line = (
                "entry-artifact example-40 agent-policy-review "
                f"examples/example-40/agent-policy-review.txt {fnv64(review_text)}"
            )
            write_text(examples_artifacts / "examples-report.txt", manifest_line + "\n")
            write_text(
                examples_artifacts / "manifest.ail-examples.txt",
                manifest_line + "\n",
            )

            plan = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_agent_policy_capture_plan.py",
                    "--examples-artifacts",
                    str(examples_artifacts),
                    "--entry-id",
                    "example-40",
                    "--output-dir",
                    str(capture_plan_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(plan.returncode, 0, plan.stdout)
            self.assertIn("handoff-roles expected", plan.stderr)
            self.assertIn("agent-policy-reviewer", plan.stderr)
        finally:
            shutil.rmtree(examples_artifacts, ignore_errors=True)
            shutil.rmtree(capture_plan_dir, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
