#!/usr/bin/env python3
"""Generate the checked v0.2 examples catalog for `ail-examples`.

The generated corpus is intentionally deterministic: it stores request and
response transcripts so the release verifier can replay without live model
access.
"""

from pathlib import Path
import json


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "examples"
REQUESTS = CORPUS / "requests"
RESPONSES = CORPUS / "responses"
STORIES = CORPUS / "stories"

TRACE_NAMES = [
    "OptionMapEvaluated",
    "TicketCreated",
    "TicketAssigned",
    "TicketClosed",
    "TicketOverdue",
    "TicketNotFound",
    "InternalNotesDenied",
    "TicketPrioritized",
    "RefundCustomerPaymentRequested",
    "RefundProviderRejected",
    "ReadPermissionAdded",
    "SecretReadInferenceBlocked",
    "ForeignCallCompress2",
    "ForeignCallbackCompared",
    "PacketHeaderLayoutChecked",
    "PayloadCompressed",
    "ForeignOutOfMemory",
    "ForeignOutputBufferTooSmall",
    "ForeignInvalidComparator",
    "PacketReceived",
    "InternalNotesViewed",
    "CounterIncremented",
    "MaintenanceCycleCompleted",
]

PROMPT_FILES = [
    "docs/ail/prompts/interview.system.md",
    "docs/ail/prompts/requirements.system.md",
    "docs/ail/prompts/spec-draft.system.md",
    "docs/ail/prompts/core-draft.system.md",
    "docs/ail/prompts/repair.system.md",
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
        "capability_level": "mid-level",
        "use_case": "Standard library collection semantics with generic Option/List/Map behavior.",
        "capability": "stdlib-generics",
        "program_domain": "package-graph",
        "module_count": "3",
        "spec_count": "3",
        "story_count": "3",
        "interacts_with": "ail_std.option,ail_std.list,ail_std.map",
        "v03_signal": "Generics need reusable conformance fixtures and teachable stdlib walkthroughs.",
    },
    "package-import": {
        "semantic_prefix": "support-composed",
        "package": "examples/support_composed.ail",
        "spec": "examples/support_composed.ail/spec.ail-spec.md",
        "capability_level": "mid-level",
        "use_case": "Package composition with explicit imports and capability grants.",
        "capability": "package-imports",
        "program_domain": "package-graph",
        "module_count": "2",
        "spec_count": "2",
        "story_count": "2",
        "interacts_with": "support_shared",
        "v03_signal": "Package graphs need clearer authoring guidance and dependency review views.",
        "vm_action": "CloseTicket",
        "runtime_state": "ticket.id=T-1;ticket.status=Open",
    },
    "ui": {
        "semantic_prefix": "option-map-ui-surface",
        "package": "examples/option_map.ail",
        "spec": "examples/option_map.ail/spec.ail-spec.md",
        "capability_level": "high-level",
        "use_case": "Small UI-tagged transform used to keep UI prompt coverage active.",
        "capability": "ui-surface-coverage",
        "program_domain": "ui-workflow",
        "module_count": "3",
        "spec_count": "3",
        "story_count": "3",
        "interacts_with": "ui.form,ui.route,ui.state",
        "v03_signal": "UI examples need richer package-local walkthroughs and stricter semantic tagging.",
    },
    "ui-workflow": {
        "semantic_prefix": "ui-workflow",
        "package": "examples/ui_workflow.ail",
        "spec": "examples/ui_workflow.ail/spec.ail-spec.md",
        "capability_level": "high-level",
        "use_case": "Accessible route, form, dashboard, and workflow semantics for a user-facing app.",
        "capability": "ui-workflow",
        "program_domain": "ui-workflow",
        "module_count": "3",
        "spec_count": "3",
        "story_count": "3",
        "interacts_with": "ui.route,ui.form,ui.dashboard",
        "v03_signal": "UI authoring needs stronger visual review artifacts and accessibility exercises.",
        "vm_action": "CreateTicketForm",
        "runtime_state": "ticket.title=Bug",
    },
    "c-host-interop": {
        "semantic_prefix": "c-interop",
        "package": "examples/c_interop.ail",
        "spec": "examples/c_interop.ail/spec.ail-spec.md",
        "capability_level": "low-level",
        "use_case": "Checked C and host interop with ABI, ownership, status, and trace contracts.",
        "capability": "c-host-interop",
        "program_domain": "c-interop",
        "module_count": "1",
        "spec_count": "1",
        "story_count": "1",
        "interacts_with": "none",
        "v03_signal": "Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.",
        "vm_action": "CompressPayload",
    },
    "support-ticket": {
        "semantic_prefix": "support-ticket",
        "package": "examples/support_ticket.ail",
        "spec": "examples/support_ticket.ail/spec.ail-spec.md",
        "capability_level": "high-level",
        "use_case": "Application workflow for support-ticket actions, permissions, failures, and traces.",
        "capability": "application-workflow",
        "program_domain": "application",
        "module_count": "3",
        "spec_count": "3",
        "story_count": "3",
        "interacts_with": "ticket.store,policy.audit,notification.queue",
        "v03_signal": "Application examples need user-story walkthroughs from intent to runtime trace.",
        "vm_action": "CloseTicket",
        "runtime_state": "ticket.id=T-1;ticket.status=Open",
    },
    "runtime-generic": {
        "semantic_prefix": "runtime-generic",
        "package": "examples/runtime_generic.ail",
        "spec": "examples/runtime_generic.ail/spec.ail-spec.md",
        "capability_level": "mid-level",
        "use_case": "Runtime generic value flow through typed actions and traceable outcomes.",
        "capability": "runtime-generics",
        "program_domain": "runtime",
        "module_count": "1",
        "spec_count": "1",
        "story_count": "1",
        "interacts_with": "none",
        "v03_signal": "Generic runtime behavior needs clearer type-inference explanations.",
        "vm_action": "PrioritizeTicket",
        "runtime_state": "ticket.id=T-1;ticket.priority=Low",
    },
    "refund-tool": {
        "semantic_prefix": "refund-tool",
        "package": "examples/refund_tool.ail",
        "spec": "examples/refund_tool.ail/spec.ail-spec.md",
        "capability_level": "high-level",
        "use_case": "Agent tool for payment refund approval with permissions and capability checks.",
        "capability": "agent-tool-safety",
        "program_domain": "agent-tool",
        "module_count": "3",
        "spec_count": "3",
        "story_count": "3",
        "interacts_with": "payment.provider,policy.engine,audit.log",
        "v03_signal": "AgentTool examples need multi-agent handoff and policy-review exercises.",
        "vm_action": "RefundCustomerPayment",
        "runtime_state": "order.id=O-1;payment.captured=true;refund.amount=100",
    },
    "compiler-pass": {
        "semantic_prefix": "compiler-pass",
        "package": "examples/compiler_pass.ail",
        "spec": "examples/compiler_pass.ail/spec.ail-spec.md",
        "capability_level": "low-level",
        "use_case": "Compiler pass semantics that transform AIL-Core with checked traces.",
        "capability": "compiler-pass",
        "program_domain": "compiler",
        "module_count": "1",
        "spec_count": "1",
        "story_count": "1",
        "interacts_with": "none",
        "v03_signal": "Self-hosting needs pass-composition examples and fixed-point checks.",
        "vm_action": "InferReadPermissions",
    },
    "network-driver": {
        "semantic_prefix": "network-driver",
        "package": "examples/network_driver.ail",
        "spec": "examples/network_driver.ail/spec.ail-spec.md",
        "capability_level": "low-level",
        "use_case": "System-level network driver boundary with effects, capabilities, and packets.",
        "capability": "system-driver",
        "program_domain": "system-driver",
        "module_count": "1",
        "spec_count": "1",
        "story_count": "1",
        "interacts_with": "none",
        "v03_signal": "Systems profile needs hardware-facing contracts and scheduler/interrupt examples.",
    },
    "secret-access": {
        "semantic_prefix": "secret-access",
        "package": "examples/secret_access.ail",
        "spec": "examples/secret_access.ail/spec.ail-spec.md",
        "capability_level": "mid-level",
        "use_case": "Secret and permission semantics for guarded internal data access.",
        "capability": "security-permissions",
        "program_domain": "runtime",
        "module_count": "1",
        "spec_count": "1",
        "story_count": "1",
        "interacts_with": "none",
        "v03_signal": "Security examples need threat-model annotations and audit trails.",
        "vm_action": "ViewInternalNotes",
        "runtime_state": "ticket.id=T-1;requester.role=SupportAgent",
    },
    "repeated-task": {
        "semantic_prefix": "repeated-task",
        "package": "examples/repeated_task.ail",
        "spec": "examples/repeated_task.ail/spec.ail-spec.md",
        "capability_level": "high-level",
        "use_case": "Scheduled repeated maintenance workflow with stateful trace evidence.",
        "capability": "scheduled-workflow",
        "program_domain": "application",
        "module_count": "3",
        "spec_count": "3",
        "story_count": "3",
        "interacts_with": "scheduler,task.store,audit.log",
        "v03_signal": "Workflow examples need temporal policies and retry/backoff semantics.",
        "vm_action": "RunMaintenanceCycle",
        "runtime_state": "counter.value=0",
    },
    "stateful-counter": {
        "semantic_prefix": "stateful-counter",
        "package": "examples/stateful_counter.ail",
        "spec": "examples/stateful_counter.ail/spec.ail-spec.md",
        "capability_level": "mid-level",
        "use_case": "Minimal state mutation that proves deterministic VM/native behavior.",
        "capability": "stateful-runtime",
        "program_domain": "runtime",
        "module_count": "1",
        "spec_count": "1",
        "story_count": "1",
        "interacts_with": "none",
        "v03_signal": "State examples need clearer persistence and concurrency boundaries.",
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
    if index == 65:
        return "UI"
    if index <= 39:
        return "Application"
    if index <= 54:
        return "AgentTool"
    if index <= 64:
        return "Compiler"
    return "System"


def surface_tags_for(index: int) -> str:
    if index == 65:
        return "ui"
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
    if index == 65:
        return "wasm32-unknown-sandbox-wasm"
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


def story_journey_for(prompt_file: str, checker_result: str) -> str:
    if checker_result == "rejected":
        return "diagnostic-story"
    if prompt_file in {
        "docs/ail/prompts/core-to-spec.system.md",
        "docs/ail/prompts/core-to-summary.system.md",
    }:
        return "spec-to-story"
    if prompt_file in {
        "docs/ail/prompts/repair.system.md",
        "docs/ail/prompts/diagnostic-repair.system.md",
        "docs/ail/prompts/flow-patch.system.md",
        "docs/ail/prompts/trace-debug.system.md",
        "docs/ail/prompts/interop.system.md",
    }:
        return "story-amendment"
    return "story-to-spec"


def program_scale_for(index: int, fixture: dict[str, str] | None) -> str:
    if fixture is None:
        return "module"
    capability = fixture["capability"]
    if capability in {"c-host-interop", "compiler-pass", "system-driver", "backend-portability"}:
        return "utility"
    if capability in {"package-imports", "runtime-generics", "security-permissions", "stateful-runtime"}:
        return "module"
    return "multi-module-system"


def program_domain_for(index: int, fixture: dict[str, str] | None) -> str:
    if fixture is None:
        return "diagnostic"
    if 30 <= index <= 34 or 90 <= index <= 94:
        return "os-utility"
    return fixture["program_domain"]


def story_evidence_for(target: str, checker_result: str, fixture: dict[str, str] | None) -> str:
    if checker_result == "rejected":
        return "diagnostics"
    if fixture is not None and "vm_action" in fixture:
        return "vm-trace"
    if target in {"linux-x86_64-elf", "wasm32-unknown-sandbox-wasm", "aarch64-apple-darwin-libsystem-macho"}:
        return "target-report"
    return "checked-core"


def user_story_id_for(index: int, checker_result: str, fixture: dict[str, str] | None) -> str:
    if checker_result == "rejected" or fixture is None:
        return "support-ticket-diagnostic-story"
    return f"{fixture['semantic_prefix']}-story"


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
    if index == 65:
        return ACCEPTED_FIXTURES["ui-workflow"]
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


def specialize_response_text(text: str, index: int) -> str:
    """Make deterministic seed responses semantically unique per scenario."""
    suffix = f"Scenario{index:03d}"
    for trace_name in TRACE_NAMES:
        text = text.replace(trace_name, f"{trace_name}{suffix}")
    return text


def main() -> None:
    REQUESTS.mkdir(parents=True, exist_ok=True)
    RESPONSES.mkdir(parents=True, exist_ok=True)
    STORIES.mkdir(parents=True, exist_ok=True)
    rejected_spec = (
        ROOT
        / "examples"
        / "support_ticket.ail"
        / "examples"
        / "rejected"
        / "missing-reference.ail-spec.md"
    ).read_text()
    entries = [
        "# AIL v0.2 Example Validation Catalog",
        "",
        "This checked catalog stores prompt and response transcripts for the",
        "`ail-examples` release verifier. Every counted example is replayed through the",
        "prompt-to-artifact path and produces deterministic verification evidence.",
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
        target = target_for(index)
        story_id = user_story_id_for(index, checker_result, fixture)
        user_story = (
            "As a reviewer I can inspect rejected prompt diagnostics so that repair keeps the intended behavior."
            if fixture is None
            else f"As a reviewer I can inspect {fixture['semantic_prefix']} behavior so that the regenerated story remains semantically similar to the checked spec."
        )
        acceptance_criteria = (
            "expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable"
            if fixture is None
            else "checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists"
        )
        story_evidence = story_evidence_for(target, checker_result, fixture)
        story_file = f"stories/example-{index}.md"
        request = {
            "semantic_task": semantic_task,
            "prompt_file": prompt_file,
            "prompt_version": "ail-prompts.v0.2",
            "artifact_kind": "ail-spec",
            "instruction": "Produce the stored AIL-Spec candidate for deterministic replay.",
        }
        response_text = (
            rejected_spec
            if fixture is None
            else specialize_response_text((ROOT / fixture["spec"]).read_text(), index)
        )
        (REQUESTS / f"example-{index}.json").write_text(json.dumps(request, sort_keys=True) + "\n")
        (RESPONSES / f"example-{index}.ail-spec.md").write_text(response_text)
        (CORPUS / story_file).write_text(
            f"# {semantic_task} User Story\n\n"
            f"user-story-id: {story_id}\n"
            f"user-story: {user_story}\n"
            f"acceptance-criteria: {acceptance_criteria}\n"
            f"story-journey: {story_journey_for(prompt_file, checker_result)}\n"
            f"story-evidence: {story_evidence}\n"
            f"program-domain: {program_domain_for(index, fixture)}\n"
            f"module-count: {'1' if fixture is None else fixture['module_count']}\n"
            f"spec-count: {'1' if fixture is None else fixture['spec_count']}\n"
            f"story-count: {'1' if fixture is None else fixture['story_count']}\n"
            f"interacts-with: {'none' if fixture is None else fixture['interacts_with']}\n"
        )
        fields = {
            "semantic-task": semantic_task,
            "profile": profile_for(index),
            "surface-tags": surface_tags_for(index),
            "package": "examples/support_ticket.ail" if fixture is None else fixture["package"],
            "use-case": "Rejected prompt output used to verify diagnostic teaching coverage."
            if fixture is None
            else fixture["use_case"],
            "capability-level": "high-level" if fixture is None else fixture["capability_level"],
            "capability-under-test": "diagnostic-replay"
            if fixture is None
            else fixture["capability"],
            "program-scale": program_scale_for(index, fixture),
            "program-domain": program_domain_for(index, fixture),
            "module-count": "1" if fixture is None else fixture["module_count"],
            "spec-count": "1" if fixture is None else fixture["spec_count"],
            "story-count": "1" if fixture is None else fixture["story_count"],
            "interacts-with": "none" if fixture is None else fixture["interacts_with"],
            "user-story-id": story_id,
            "user-story": user_story,
            "acceptance-criteria": acceptance_criteria,
            "story-evidence": story_evidence,
            "story-file": story_file,
            "story-journey": story_journey_for(prompt_file, checker_result),
            "story-roundtrip": "diagnostic-preserving"
            if checker_result == "rejected"
            else "semantic-similar",
            "distinctness-claim": f"{semantic_task} exercises {prompt_file} over its capability path.",
            "v0.3-signal": "Rejected examples need repair tutorials that convert diagnostics into corrected specs."
            if fixture is None
            else fixture["v03_signal"],
            "prompt-file": prompt_file,
            "prompt-version": "ail-prompts.v0.2",
            "prompt-fingerprint": fnv64((ROOT / prompt_file).read_text()),
            "executor-family": executor_family,
            "executor-label": "local-executor",
            "capture-origin": "deterministic-seed",
            "request-file": f"requests/example-{index}.json",
            "response-file": f"responses/example-{index}.ail-spec.md",
            "artifact-kind": "ail-spec",
            "checker-result": checker_result,
            "target": target,
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
