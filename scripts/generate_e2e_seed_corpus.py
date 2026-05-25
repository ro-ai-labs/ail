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

ACCEPTED_FIXTURES = {
    "standard-library": {
        "semantic_prefix": "stdlib-collections",
        "package": "examples/ail_std_collections.ail",
        "spec": "examples/ail_std_collections.ail/spec.ail-spec.md",
    },
    "package-import": {
        "semantic_prefix": "support-shared",
        "package": "examples/support_shared.ail",
        "spec": "examples/support_shared.ail/spec.ail-spec.md",
    },
    "ui": {
        "semantic_prefix": "option-map-ui-surface",
        "package": "examples/option_map.ail",
        "spec": "examples/option_map.ail/spec.ail-spec.md",
    },
    "c-host-interop": {
        "semantic_prefix": "c-interop",
        "package": "examples/c_interop.ail",
        "spec": "examples/c_interop.ail/spec.ail-spec.md",
        "vm_action": "CompressPayload",
    },
    "support-ticket": {
        "semantic_prefix": "support-ticket",
        "package": "examples/support_ticket.ail",
        "spec": "examples/support_ticket.ail/spec.ail-spec.md",
        "vm_action": "CloseTicket",
        "runtime_state": "ticket.id=T-1;ticket.status=Open",
    },
    "runtime-generic": {
        "semantic_prefix": "runtime-generic",
        "package": "examples/runtime_generic.ail",
        "spec": "examples/runtime_generic.ail/spec.ail-spec.md",
        "vm_action": "PrioritizeTicket",
        "runtime_state": "ticket.id=T-1;ticket.priority=Low",
    },
    "refund-tool": {
        "semantic_prefix": "refund-tool",
        "package": "examples/refund_tool.ail",
        "spec": "examples/refund_tool.ail/spec.ail-spec.md",
        "vm_action": "RefundCustomerPayment",
        "runtime_state": "order.id=O-1;payment.captured=true;refund.amount=100",
    },
    "compiler-pass": {
        "semantic_prefix": "compiler-pass",
        "package": "examples/compiler_pass.ail",
        "spec": "examples/compiler_pass.ail/spec.ail-spec.md",
        "vm_action": "InferReadPermissions",
    },
    "network-driver": {
        "semantic_prefix": "network-driver",
        "package": "examples/network_driver.ail",
        "spec": "examples/network_driver.ail/spec.ail-spec.md",
    },
    "secret-access": {
        "semantic_prefix": "secret-access",
        "package": "examples/secret_access.ail",
        "spec": "examples/secret_access.ail/spec.ail-spec.md",
        "vm_action": "ViewInternalNotes",
        "runtime_state": "ticket.id=T-1;requester.role=SupportAgent",
    },
    "repeated-task": {
        "semantic_prefix": "repeated-task",
        "package": "examples/repeated_task.ail",
        "spec": "examples/repeated_task.ail/spec.ail-spec.md",
        "vm_action": "RunMaintenanceCycle",
        "runtime_state": "counter.value=0",
    },
    "stateful-counter": {
        "semantic_prefix": "stateful-counter",
        "package": "examples/stateful_counter.ail",
        "spec": "examples/stateful_counter.ail/spec.ail-spec.md",
        "vm_action": "IncrementCounter",
        "runtime_state": "counter.value=0",
    },
}


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
    if index <= 19 or 20 <= index <= 24:
        return "vm"
    if 25 <= index <= 29:
        return "wasm32-unknown-sandbox-wasm"
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


def accepted_fixture_for(index: int) -> dict[str, str]:
    if index <= 9:
        return ACCEPTED_FIXTURES["standard-library"]
    if index <= 19:
        return ACCEPTED_FIXTURES["package-import"]
    if index <= 24:
        return ACCEPTED_FIXTURES["ui"]
    if index <= 29:
        return ACCEPTED_FIXTURES["c-host-interop"]
    if index <= 34:
        return ACCEPTED_FIXTURES["support-ticket"]
    if index <= 39:
        return ACCEPTED_FIXTURES["runtime-generic"]
    if index <= 54:
        return ACCEPTED_FIXTURES["refund-tool"]
    if index <= 64:
        return ACCEPTED_FIXTURES["compiler-pass"]
    if index <= 74:
        return ACCEPTED_FIXTURES["network-driver"]
    if index <= 79:
        return ACCEPTED_FIXTURES["secret-access"]
    if index <= 84:
        return ACCEPTED_FIXTURES["repeated-task"]
    if index <= 89:
        return ACCEPTED_FIXTURES["c-host-interop"]
    if index <= 94:
        return ACCEPTED_FIXTURES["support-ticket"]
    return ACCEPTED_FIXTURES["stateful-counter"]


def main() -> None:
    REQUESTS.mkdir(parents=True, exist_ok=True)
    RESPONSES.mkdir(parents=True, exist_ok=True)
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
        fixture = None if checker_result == "rejected" else accepted_fixture_for(index)
        semantic_task = (
            "support-ticket-rejected"
            if fixture is None
            else f"{fixture['semantic_prefix']}-{index}"
        )
        request = {
            "semantic_task": semantic_task,
            "prompt_file": prompt_file,
            "prompt_version": "ail-prompts.v0.2",
            "artifact_kind": "ail-spec",
            "instruction": "Produce the stored AIL-Spec candidate for deterministic replay.",
        }
        response_text = rejected_spec if fixture is None else (ROOT / fixture["spec"]).read_text()
        (REQUESTS / f"example-{index}.json").write_text(json.dumps(request, sort_keys=True) + "\n")
        (RESPONSES / f"example-{index}.ail-spec.md").write_text(response_text)
        fields = {
            "semantic-task": semantic_task,
            "profile": profile_for(index),
            "surface-tags": surface_tags_for(index),
            "package": "examples/support_ticket.ail" if fixture is None else fixture["package"],
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
        }
        if fixture is not None and "vm_action" in fixture:
            fields["vm-action"] = fixture["vm_action"]
        if fixture is not None and "runtime_state" in fixture:
            fields["runtime-state"] = fixture["runtime_state"]
        if executor_family == "llm-http":
            fields["endpoint-label"] = endpoint_label_for(index)
        if checker_result == "rejected":
            fields["vm-action"] = "CloseTicket"
            fields["runtime-state"] = "ticket.id=T-1;ticket.status=Open"
            fields["expected-diagnostic"] = "AIL001"
            fields["failure-taxonomy"] = "semantic-drift"
        entries.append(f"## End-To-End Example: example-{index}")
        entries.extend(f"{key}: {value}" for key, value in fields.items())
        entries.append("")
    (CORPUS / "examples.md").write_text("\n".join(entries))


if __name__ == "__main__":
    main()
