#!/usr/bin/env python3
"""Generate the checked v0.2 seed corpus for `ail-e2e-corpus`.

The generated corpus is intentionally deterministic: it stores request and
response transcripts so the release verifier can replay without live model
access.
"""

from pathlib import Path
import json


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "docs" / "ail" / "corpus" / "e2e"
REQUESTS = CORPUS / "requests"
RESPONSES = CORPUS / "responses"

PROMPT_FILES = [
    "docs/ail/prompts/interview.system.md",
    "docs/ail/prompts/requirements.system.md",
    "docs/ail/prompts/spec-draft.system.md",
    "docs/ail/prompts/core-draft.system.md",
    "docs/ail/prompts/diagnostic-repair.system.md",
    "docs/ail/prompts/core-to-spec.system.md",
    "docs/ail/prompts/core-to-summary.system.md",
    "docs/ail/prompts/flow-patch.system.md",
    "docs/ail/prompts/trace-debug.system.md",
    "docs/ail/prompts/interop.system.md",
]


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def profile_for(index: int) -> str:
    if index <= 39:
        return "Application"
    if index <= 54:
        return "AgentTool"
    if index <= 64:
        return "Compiler"
    return "System"


def surface_tags_for(index: int) -> str:
    if index <= 9:
        return "standard-library"
    if index <= 19:
        return "package-import"
    if index <= 24:
        return "ui"
    if index <= 29:
        return "c-host-interop"
    if index <= 34:
        return "backend-portability"
    return "core"


def target_for(index: int) -> str:
    if 85 <= index <= 89:
        return "wasm32-unknown-sandbox-wasm"
    if 90 <= index <= 94:
        return "aarch64-apple-darwin-libsystem-macho"
    if 95 <= index <= 99:
        return "vm"
    return "linux-x86_64-elf"


def executor_family_for(index: int) -> str:
    return "codex-skill-agent" if index == 99 else "llm-http"


def endpoint_label_for(index: int) -> str:
    return "local-endpoint-alt" if index == 1 else "local-endpoint"


def main() -> None:
    REQUESTS.mkdir(parents=True, exist_ok=True)
    RESPONSES.mkdir(parents=True, exist_ok=True)
    accepted_spec = (ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md").read_text()
    rejected_spec = (
        ROOT
        / "examples"
        / "support_ticket.ail"
        / "examples"
        / "rejected"
        / "missing-reference.ail-spec.md"
    ).read_text()
    entries = [
        "# AIL v0.2 End-To-End Seed Corpus",
        "",
        "This checked seed corpus stores deterministic prompt and response transcripts",
        "for the `ail-e2e-corpus` release verifier.",
        "",
    ]
    for index in range(100):
        prompt_file = PROMPT_FILES[index % len(PROMPT_FILES)]
        executor_family = executor_family_for(index)
        checker_result = "rejected" if index == 99 else "accepted"
        semantic_task = "support-ticket-rejected" if index == 99 else f"support-ticket-{index}"
        request = {
            "semantic_task": semantic_task,
            "prompt_file": prompt_file,
            "prompt_version": "ail-prompts.v0.2",
            "artifact_kind": "ail-spec",
            "instruction": "Produce the stored AIL-Spec candidate for deterministic replay.",
        }
        response_text = rejected_spec if index == 99 else accepted_spec
        (REQUESTS / f"example-{index}.json").write_text(json.dumps(request, sort_keys=True) + "\n")
        (RESPONSES / f"example-{index}.ail-spec.md").write_text(response_text)
        fields = {
            "semantic-task": semantic_task,
            "profile": profile_for(index),
            "surface-tags": surface_tags_for(index),
            "package": "examples/support_ticket.ail",
            "prompt-file": prompt_file,
            "prompt-version": "ail-prompts.v0.2",
            "prompt-fingerprint": fnv64((ROOT / prompt_file).read_text()),
            "executor-family": executor_family,
            "executor-label": "local-executor",
            "request-file": f"requests/example-{index}.json",
            "response-file": f"responses/example-{index}.ail-spec.md",
            "artifact-kind": "ail-spec",
            "checker-result": checker_result,
            "target": target_for(index),
            "vm-action": "CloseTicket",
            "runtime-state": "ticket.id=T-1;ticket.status=Open",
        }
        if executor_family == "llm-http":
            fields["endpoint-label"] = endpoint_label_for(index)
        if checker_result == "rejected":
            fields["expected-diagnostic"] = "AIL001"
            fields["failure-taxonomy"] = "semantic-drift"
        entries.append(f"## End-To-End Example: example-{index}")
        entries.extend(f"{key}: {value}" for key, value in fields.items())
        entries.append("")
    (CORPUS / "examples.md").write_text("\n".join(entries))


if __name__ == "__main__":
    main()
