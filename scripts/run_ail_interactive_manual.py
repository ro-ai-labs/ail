#!/usr/bin/env python3
"""Run or print the AIL interactive manual chapters.

The manual runner is deterministic by default. It lists chapters, prints the
commands a reader can run, and only executes local verification commands when
`--run-checks` is supplied. Live LLM commands stay opt-in through
`--include-live`.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ManualCommand:
    label: str
    command: tuple[str, ...]
    live: bool = False
    evidence: tuple[str, ...] = ()

    def shell_line(self) -> str:
        return " ".join(self.command)


@dataclass(frozen=True)
class ManualChapter:
    chapter_id: str
    title: str
    doc: str
    purpose: str
    commands: tuple[ManualCommand, ...]


@dataclass(frozen=True)
class LiveOverrides:
    endpoint: str | None = None
    skip_model_check: bool = False
    artifact_root: str | None = None


BASE_CHAPTERS: tuple[ManualChapter, ...] = (
    ManualChapter(
        chapter_id="user-story-mode",
        title="User Story Mode",
        doc="docs/ail/manual/01-user-story-mode.md",
        purpose="Start from a story file, involve the toolchain agent, and produce checked artifacts.",
        commands=(
            ManualCommand(
                label="show-story-mode-live-command",
                command=("python3", "scripts/run_v03_story_llm_harness.py", "--dry-run"),
            ),
            ManualCommand(
                label="verify-story-mode-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_builds_checked_artifacts_from_story_file",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "story-mode-report.txt",
                    "manifest.ail-story.txt",
                    "llm/requirements.request.json",
                    "llm/requirements.response.json",
                    "llm/requirements.content.txt",
                    "llm/spec.request.json",
                    "llm/spec.response.json",
                    "llm/spec.content.txt",
                    "story-prompt-envelope-valid-count",
                    "story-prompt-envelope-invalid-count",
                ),
            ),
            ManualCommand(
                label="verify-story-agent-entrypoint-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_uses_default_toolchain_agent_entrypoint",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "agent.ailbc.json",
                    "agent-trace.txt",
                    "agent agent.ailbc.json",
                    "agent-trace agent-trace.txt",
                    "manifest.ail-story.txt",
                ),
            ),
            ManualCommand(
                label="verify-story-blocking-questions-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_surfaces_blocking_questions_as_story_artifact",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "story-questions.ail-interview.md",
                    "story-mode-report.txt",
                    "story-llm-transcript-count",
                    "story-prompt-envelope-valid-count",
                    "story-prompt-envelope-invalid-count",
                    "llm/requirements.request.json",
                    "llm/requirements.response.json",
                    "llm/requirements.content.txt",
                    "llm-requirements-request",
                    "llm-requirements-response",
                    "llm-requirements-content",
                    "manifest.ail-story.txt",
                ),
            ),
            ManualCommand(
                label="verify-story-runtime-trace-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_native_target_executes_story_runtime_trace",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "target.elf",
                    "native-bytecode-report.txt",
                    "dependency-report.txt",
                    "manifest.ail-build.txt",
                    "manifest.ail-story.txt",
                    "agent-trace.txt",
                    "ticket.status=Closed",
                    "trace TicketClosed",
                ),
            ),
            ManualCommand(
                label="verify-story-amendment-comparison-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_story_amendment_writes_comparison_artifact",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "story-amendment-comparison.txt",
                    "story-amendment-comparison.fingerprint.txt",
                    "story-amendment-comparison: present",
                    "comparison-result accepted",
                    "semantic-anchor-preserved-count 4",
                    "semantic-anchor-missing-count 0",
                ),
            ),
            ManualCommand(
                label="verify-incident-story-amendment-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_incident_response_story_amendment_preserves_application_anchors",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "examples/incident_response.ail",
                    "story-amendment-comparison.txt",
                    "story-amendment-comparison.fingerprint.txt",
                    "story-amendment-comparison: present",
                    "comparison-result accepted",
                    "IncidentEscalated",
                    "notification audit entry",
                    "public timeline subscribers",
                    "semantic-anchor-preserved-count 5",
                    "semantic-anchor-missing-count 0",
                ),
            ),
            ManualCommand(
                label="run-story-mode-live",
                command=("python3", "scripts/run_v03_story_llm_harness.py"),
                live=True,
            ),
            ManualCommand(
                label="review-story-mode-live-artifacts",
                command=(
                    "python3",
                    "scripts/run_v03_story_llm_harness.py",
                    "--review-artifacts",
                    "/tmp/ail-v03-story-llm",
                ),
                live=True,
                evidence=(
                    "story-llm-harness-report.txt",
                    "story-llm-harness-report.fingerprint.txt",
                    "story-mode-report.txt",
                    "manifest.ail-story.txt",
                    "model-check.json",
                    "model-check.fingerprint.txt",
                    "model-check",
                    "model-check-model-count",
                    "model-check-model-id",
                    "agent-trace.txt",
                    "agent-trace.fingerprint.txt",
                    "story-llm-transcript-check-count",
                    "story-prompt-envelope-valid-count",
                    "story-prompt-envelope-artifact-count",
                    "story-prompt-envelope-questions-count",
                    "story-prompt-envelope-invalid-count",
                ),
            ),
            ManualCommand(
                label="plan-story-promotion-capture",
                command=(
                    "python3",
                    "scripts/run_v03_story_promotion_capture_plan.py",
                    "--story-artifacts",
                    "/tmp/ail-v03-story-llm",
                    "--output-dir",
                    "/tmp/ail-v03-story-promotion-capture-plan",
                ),
                live=True,
                evidence=(
                    "story-promotion-capture-plan.json",
                    "story-promotion-capture-plan.txt",
                    "story-promotion-capture-plan.fingerprint.txt",
                    "promotion-decision accepted-for-promotion",
                    "human-approval-required true",
                ),
            ),
            ManualCommand(
                label="demo-story-promotion-import",
                command=(
                    "python3",
                    "scripts/run_v03_story_promotion_import_demo.py",
                    "--story-artifacts",
                    "/tmp/ail-v03-story-llm",
                    "--capture-plan-dir",
                    "/tmp/ail-v03-story-promotion-capture-plan",
                    "--work-dir",
                    "/tmp/ail-v03-story-promotion-import-work",
                    "--output-corpus",
                    "/tmp/ail-v03-story-promotion-import-corpus",
                    "--output-artifacts",
                    "/tmp/ail-v03-story-promotion-import-artifacts",
                ),
                live=True,
                evidence=(
                    "story-promotion-import-demo-report.txt",
                    "story-promotion-import-demo-report.fingerprint.txt",
                    "story-artifacts-preserved true",
                    "proposed-accepted true",
                    "capture-plan story-promotion-capture-plan.json",
                    "promotion-decision accepted-for-promotion",
                    "human-approval-required true",
                    "promotion-source human-approved-story-promotion-batch",
                    "human-approved-story-promotion-batch.fingerprint.txt",
                    "batch-plan-fingerprint",
                ),
            ),
            ManualCommand(
                label="direct-ail-story-live",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-story",
                    "examples/support_ticket.ail",
                    "--story-file",
                    "examples/stories/example-30.md",
                    "--agent",
                    "examples/ail_toolchain_agent.ail",
                    "--artifact-dir",
                    "/tmp/ail-user-story-mode",
                    "--llm-endpoint",
                    "http://inteligentia-pro-1:8080/v1/chat/completions",
                ),
                live=True,
            ),
        ),
    ),
    ManualChapter(
        chapter_id="examples-release",
        title="Examples Release Replay",
        doc="docs/ail/manual/02-examples-release.md",
        purpose="Replay the full examples catalog and inspect learning evidence.",
        commands=(
            ManualCommand(
                label="replay-examples-release",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-examples",
                    "--release-evidence",
                ),
                evidence=(
                    "examples-report.txt",
                    "v03-roadmap.txt",
                    "manifest.ail-examples.txt",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="v03-roadmap",
        title="v0.3 Roadmap",
        doc="docs/ail/manual/05-v03-roadmap.md",
        purpose="Print the examples-derived next-version backlog without mining the full replay report.",
        commands=(
            ManualCommand(
                label="print-v03-roadmap",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-v03-roadmap",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-v03-roadmap",
                    "--release-evidence",
                ),
                evidence=(
                    "AIL-v0.3-Roadmap",
                    "v03-roadmap.txt",
                    "v03-roadmap.fingerprint.txt",
                    "manifest.ail-examples.txt",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="prompt-interaction",
        title="Prompt Interaction",
        doc="docs/ail/manual/03-prompt-interaction.md",
        purpose="Inspect prompt-pack surfaces and stored request/response replay.",
        commands=(
            ManualCommand(
                label="list-prompts",
                command=("rg", "--files", "docs/ail/prompts"),
            ),
            ManualCommand(
                label="run-prompt-corpus",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-prompt-corpus",
                    "docs/ail/corpus/prompts",
                    "--artifact-dir",
                    "/tmp/ail-manual-prompt-corpus",
                ),
                evidence=(
                    "prompt-corpus-portability.txt",
                    "manifest.ail-prompt-corpus.txt",
                ),
            ),
            ManualCommand(
                label="show-prompt-pack-live-command",
                command=("python3", "scripts/run_v03_prompt_llm_harness.py", "--dry-run"),
            ),
            ManualCommand(
                label="replay-examples-prompt-surfaces",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-prompt-examples",
                    "--release-evidence",
                ),
                evidence=("AIL-Examples-Report", "prompt-count"),
            ),
            ManualCommand(
                label="inspect-capture-help",
                command=("python3", "scripts/capture_example_transcripts.py", "--help"),
            ),
            ManualCommand(
                label="run-prompt-pack-live",
                command=("python3", "scripts/run_v03_prompt_llm_harness.py"),
                live=True,
            ),
            ManualCommand(
                label="review-prompt-pack-live-artifacts",
                command=(
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    "/tmp/ail-v03-prompt-llm",
                ),
                live=True,
                evidence=(
                    "prompt-llm-harness-report.txt",
                    "manifest.v03-prompt-llm.txt",
                    "models.json",
                    "models.fingerprint.txt",
                    "model-check",
                    "model-check-model-count",
                    "model-check-model-id",
                    "prompt-envelope-valid-count",
                    "prompt-envelope-artifact-required-count",
                    "prompt-envelope-questions-expected-count",
                    "prompt-outcome-match-count",
                    "prompt-envelope-invalid-count",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="agent-entrypoint",
        title="Agent Entrypoint",
        doc="docs/ail/manual/04-agent-entrypoint.md",
        purpose="Inspect Codex agent roles and the AIL toolchain-agent package.",
        commands=(
            ManualCommand(
                label="show-agent-guides",
                command=("rg", "--files", "examples/agents"),
            ),
            ManualCommand(
                label="validate-agent-contracts",
                command=("cargo", "run", "--", "ail-agent-contracts", "examples/agents"),
                evidence=(
                    "AIL-Agent-Contracts-Report",
                    "codex-ail-prompt-reviewer.md",
                    "examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md",
                    "repair-promotion-import-demo-report.txt",
                    "agent-policy-import-demo-report.txt",
                    "source-preserved true",
                    "proposed-accepted true",
                    "policy-handoff-imported true",
                    "policy-handoff-replayed true",
                ),
            ),
            ManualCommand(
                label="check-toolchain-agent",
                command=("cargo", "run", "--", "ail-check", "examples/ail_toolchain_agent.ail"),
            ),
            ManualCommand(
                label="check-toolchain-agent-conformance",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-conformance",
                    "examples/ail_toolchain_agent.ail",
                    "--artifact-dir",
                    "/tmp/ail-manual-agent-entrypoint-conformance",
                ),
                evidence=(
                    "conformance-report.txt",
                    "manifest.ail-conformance.txt",
                    "accepted: bytecode-verification-minimal.ail-spec.md",
                    "rejected: bytecode-verification-without-fingerprint.ail-spec.md AIL-AGENT-001",
                    "ail conformance: ok",
                ),
            ),
            ManualCommand(
                label="verify-toolchain-agent-package",
                command=(
                    "cargo",
                    "test",
                    "ail_toolchain_agent_package_lowers_to_verified_bytecode",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=("agent.ailbc.json",),
            ),
            ManualCommand(
                label="verify-agent-build-entrypoint",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_build_runs_toolchain_agent_bytecode",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "agent.ailbc.json",
                    "agent-trace.txt",
                ),
            ),
            ManualCommand(
                label="verify-agent-bytecode-after-compile",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_build_agent_verifies_bytecode_artifact_after_compile",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "action CompileApplication started",
                    "action VerifyBytecodeArtifact started",
                    "agent.ailbc.json",
                    "artifact.ailbc.json",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="bootstrap-self-hosting",
        title="Bootstrap Self-Hosting",
        doc="docs/ail/manual/10-bootstrap-self-hosting.md",
        purpose=(
            "Run the AIL-authored toolchain agent and AIL-Meta compiler pass "
            "through a fixed-point bootstrap bundle."
        ),
        commands=(
            ManualCommand(
                label="run-bootstrap-self-hosting-bundle",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-bootstrap",
                    "examples/ail_toolchain_agent.ail",
                    "--pass",
                    "examples/compiler_pass.ail",
                    "--agent",
                    "examples/ail_toolchain_agent.ail",
                    "--target",
                    "linux-x86_64-elf",
                    "--artifact-dir",
                    "/tmp/ail-manual-bootstrap-self-hosting",
                ),
                evidence=(
                    "bootstrap-fixed-point-report.txt",
                    "fixed-point: ok",
                    "second-pass-changed false",
                    "bootstrap-pass-composition-report.txt",
                    "composition-pass-count 1",
                    "composition-pass 1 InferReadPermissions",
                    "pass-order-status ok",
                    "bootstrap-native-bytecode-report.txt",
                    "bootstrap-host-boundary-report.txt",
                    "no-host-backend-source true",
                    "bootstrap-dependency-report.txt",
                    "bootstrap-handoff-report.txt",
                    "handoff-native-role toolchain-agent all-actions ok count 18",
                    "handoff-native-role compiler-pass all-actions ok count 1",
                    "handoff-native-role agent all-actions ok count 18",
                    "manifest.ail-bootstrap.txt",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="systems-profile",
        title="Systems Profile",
        doc="docs/ail/manual/11-systems-profile.md",
        purpose=(
            "Check the low-level System profile package, scheduler and "
            "interrupt fixtures, native target contract, and runtime trace."
        ),
        commands=(
            ManualCommand(
                label="check-network-driver-conformance",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-conformance",
                    "examples/network_driver.ail",
                    "--artifact-dir",
                    "/tmp/ail-manual-systems-profile-conformance",
                ),
                evidence=(
                    "conformance-report.txt",
                    "manifest.ail-conformance.txt",
                    "accepted: scheduler-task-minimal.ail-spec.md",
                    "accepted: interrupt-context-minimal.ail-spec.md",
                    "rejected: interrupt-context-blocking-effect.ail-spec.md AIL033",
                    "rejected: scheduler-task-unknown-context.ail-spec.md AIL035",
                    "rejected: interrupt-mask-unknown-context.ail-spec.md AIL040",
                ),
            ),
            ManualCommand(
                label="compile-network-driver-native",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-compile",
                    "examples/network_driver.ail",
                    "--action",
                    "NetworkPacketReceiver",
                    "--target",
                    "linux-x86_64-elf",
                    "--out",
                    "/tmp/ail-manual-systems-profile-network-driver.elf",
                    "--artifact-dir",
                    "/tmp/ail-manual-systems-profile-native",
                ),
                evidence=(
                    "checked.ail-core.txt",
                    "artifact.ailbc.json",
                    "native-bytecode-report.txt",
                    "dependency-report.txt",
                    "manifest.ail-compile.txt",
                    "machine-bytecode-contract linux-x86_64-elf",
                ),
            ),
            ManualCommand(
                label="run-network-driver-native",
                command=(
                    "/tmp/ail-manual-systems-profile-network-driver.elf",
                ),
                evidence=(
                    "system component Network packet receiver started",
                    "system effect read network device",
                    "system effect release rx buffer",
                    "trace PacketReceived",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="application-baseline",
        title="Application Baseline",
        doc="docs/ail/manual/12-application-baseline.md",
        purpose=(
            "Check the high-level Application baselines with package-local "
            "accepted and rejected conformance fixtures."
        ),
        commands=(
            ManualCommand(
                label="check-support-ticket-conformance",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-conformance",
                    "examples/support_ticket.ail",
                    "--artifact-dir",
                    "/tmp/ail-manual-application-baseline-conformance",
                ),
                evidence=(
                    "conformance-report.txt",
                    "manifest.ail-conformance.txt",
                    "accepted: close-ticket-minimal.ail-spec.md",
                    "rejected: secret-leak.ail-spec.md AIL002",
                    "rejected: action-without-trace.ail-spec.md AIL-TRACE-001",
                    "rejected: failure-without-trace.ail-spec.md AIL-TRACE-002",
                    "rejected: unknown-field-type.ail-spec.md AIL-TYPE-001",
                    "rejected: assignment-without-role-requirement.ail-spec.md AIL-APP-001",
                    "rejected: overdue-without-time-requirement.ail-spec.md AIL-APP-002",
                    "rejected: status-change-without-public-update.ail-spec.md AIL-APP-003",
                    "ail conformance: ok",
                ),
            ),
            ManualCommand(
                label="check-incident-response-conformance",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-conformance",
                    "examples/incident_response.ail",
                    "--artifact-dir",
                    "/tmp/ail-manual-incident-response-conformance",
                ),
                evidence=(
                    "conformance-report.txt",
                    "manifest.ail-conformance.txt",
                    "accepted: incident-escalation-minimal.ail-spec.md",
                    "rejected: notification-without-responder-pager.ail-spec.md AIL-APP-004",
                    "rejected: resolve-without-mitigating-status.ail-spec.md AIL-APP-005",
                    "rejected: postmortem-without-resolved-status.ail-spec.md AIL-APP-005",
                    "rejected: private-notes-public-timeline-leak.ail-spec.md AIL-APP-006",
                    "rejected: escalation-without-commander-review.ail-spec.md AIL-APP-007",
                    "rejected: route-missing-permission.ail-spec.md AIL-UI-PERMISSION-002",
                    "rejected: dashboard-missing-permission.ail-spec.md AIL-UI-PERMISSION-001",
                    "rejected-repair-tutorial-count 7",
                    "rejected/private-notes-public-timeline-leak.ail-spec.md/repair-tutorial.txt",
                    "rejected-repair-proof-count 7",
                    "rejected/private-notes-public-timeline-leak.ail-spec.md/repair-proof.txt",
                    "rejected/private-notes-public-timeline-leak.ail-spec.md/repair-candidate.ail-spec.md",
                    "rejected/private-notes-public-timeline-leak.ail-spec.md/repair-checked.ail-core.txt",
                    "rejected/private-notes-public-timeline-leak.ail-spec.md/repair-artifact.ailbc.json",
                    "ail conformance: ok",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="stateful-runtime",
        title="Stateful Runtime",
        doc="docs/ail/manual/13-stateful-runtime.md",
        purpose=(
            "Check stateful Application runtime policies for persistence, "
            "idempotent retries, shared-state serialization, replay recovery, "
            "and counter VM execution."
        ),
        commands=(
            ManualCommand(
                label="check-stateful-counter-conformance",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-conformance",
                    "examples/stateful_counter.ail",
                    "--artifact-dir",
                    "/tmp/ail-manual-stateful-runtime-conformance",
                ),
                evidence=(
                    "conformance-report.txt",
                    "manifest.ail-conformance.txt",
                    "accepted: persistent-increment-minimal.ail-spec.md",
                    "accepted: idempotent-increment-request-minimal.ail-spec.md",
                    "accepted: locked-counter-increment-minimal.ail-spec.md",
                    "accepted: replay-after-failure-minimal.ail-spec.md",
                    "rejected: increment-without-persistence-guarantee.ail-spec.md AIL-STATE-001",
                    "rejected: retryable-increment-without-idempotency-key.ail-spec.md AIL-STATE-002",
                    "rejected: shared-counter-without-lock.ail-spec.md AIL-STATE-003",
                    "rejected: failure-after-write-without-replay-policy.ail-spec.md AIL-STATE-004",
                    "ail conformance: ok",
                ),
            ),
            ManualCommand(
                label="verify-stateful-counter-bytecode-runtime",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-run",
                    "examples/stateful_counter.ail",
                    "--action",
                    "IncrementCounter",
                    "counter.value=41",
                ),
                evidence=(
                    "ail-run succeeded",
                    "counter.value=42",
                    "add counter.value by 1 -> 42",
                    "trace CounterIncremented",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="repair-promotion",
        title="Repair Promotion Review",
        doc="docs/ail/manual/07-repair-promotion.md",
        purpose="Review rejected-example repair evidence before proposing a repaired artifact for corpus promotion.",
        commands=(
            ManualCommand(
                label="replay-repair-promotion-evidence",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-repair-promotion",
                    "--release-evidence",
                ),
                evidence=(
                    "examples-report.txt",
                    "manifest.ail-examples.txt",
                    "repair-promotion-review.txt",
                    "repair-promotion-review.fingerprint.txt",
                    "repair-promotion-review-fingerprint-observed-count",
                ),
            ),
            ManualCommand(
                label="inspect-repair-promotion-review-lines",
                command=(
                    "rg",
                    "-n",
                    "repair-promotion-review-fingerprint-observed-count|entry-artifact example-99 repair-promotion-review|entry-artifact example-107 repair-promotion-review",
                    "/tmp/ail-manual-repair-promotion/examples-report.txt",
                    "/tmp/ail-manual-repair-promotion/manifest.ail-examples.txt",
                ),
                evidence=(
                    "repair-promotion-review-fingerprint-observed-count",
                    "entry-artifact example-99 repair-promotion-review",
                    "entry-artifact example-107 repair-promotion-review",
                ),
            ),
            ManualCommand(
                label="plan-repair-promotion-capture",
                command=(
                    "python3",
                    "scripts/run_v03_repair_promotion_capture_plan.py",
                    "--examples-artifacts",
                    "/tmp/ail-manual-repair-promotion",
                    "--entry-id",
                    "example-99",
                    "--output-dir",
                    "/tmp/ail-manual-repair-promotion-capture-plan",
                ),
                evidence=(
                    "repair-promotion-capture-plan.json",
                    "repair-promotion-capture-plan.txt",
                    "repair-promotion-capture-plan.fingerprint.txt",
                ),
            ),
            ManualCommand(
                label="demo-repair-promotion-import",
                command=(
                    "python3",
                    "scripts/run_v03_repair_promotion_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--examples-artifacts",
                    "/tmp/ail-manual-repair-promotion",
                    "--capture-plan-dir",
                    "/tmp/ail-manual-repair-promotion-capture-plan",
                    "--source-entry-id",
                    "example-99",
                    "--work-dir",
                    "/tmp/ail-manual-repair-promotion-import-work",
                    "--output-corpus",
                    "/tmp/ail-manual-repair-promotion-import-corpus",
                    "--output-artifacts",
                    "/tmp/ail-manual-repair-promotion-import-artifacts",
                ),
                evidence=(
                    "repair-promotion-import-demo-report.txt",
                    "repair-promotion-import-demo-report.fingerprint.txt",
                    "source-preserved true",
                    "proposed-accepted true",
                    "entry-count 125",
                    "checker-result-count accepted 116",
                    "checker-result-count rejected 9",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="ui-patch-import",
        title="UI Patch Import",
        doc="docs/ail/manual/08-ui-patch-import.md",
        purpose="Review deterministic UI patch plans and import a human-approved ail-flow-edit candidate into a corpus copy.",
        commands=(
            ManualCommand(
                label="replay-ui-patch-evidence",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-ui-patch",
                    "--release-evidence",
                ),
                evidence=(
                    "examples-report.txt",
                    "manifest.ail-examples.txt",
                    "ui-review.txt",
                    "ui-review-patch.txt",
                    "ui-review-patch-fingerprint-observed-count",
                ),
            ),
            ManualCommand(
                label="inspect-ui-patch-lines",
                command=(
                    "rg",
                    "-n",
                    "ui-review-patch-fingerprint-observed-count|entry-artifact example-108 ui-review-patch",
                    "/tmp/ail-manual-ui-patch/examples-report.txt",
                    "/tmp/ail-manual-ui-patch/manifest.ail-examples.txt",
                ),
                evidence=(
                    "ui-review-patch-fingerprint-observed-count",
                    "entry-artifact example-108 ui-review-patch",
                ),
            ),
            ManualCommand(
                label="plan-ui-patch-capture",
                command=(
                    "python3",
                    "scripts/run_v03_ui_patch_capture_plan.py",
                    "--examples-artifacts",
                    "/tmp/ail-manual-ui-patch",
                    "--entry-id",
                    "example-108",
                    "--output-dir",
                    "/tmp/ail-manual-ui-patch-capture-plan",
                ),
                evidence=(
                    "ui-patch-capture-plan.json",
                    "ui-patch-capture-plan.txt",
                    "ui-patch-capture-plan.fingerprint.txt",
                    "patch-command ail-flow-edit",
                    "human-approval-required true",
                ),
            ),
            ManualCommand(
                label="demo-ui-patch-import",
                command=(
                    "python3",
                    "scripts/run_v03_ui_patch_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--examples-artifacts",
                    "/tmp/ail-manual-ui-patch",
                    "--capture-plan-dir",
                    "/tmp/ail-manual-ui-patch-capture-plan",
                    "--source-entry-id",
                    "example-108",
                    "--work-dir",
                    "/tmp/ail-manual-ui-patch-import-work",
                    "--output-corpus",
                    "/tmp/ail-manual-ui-patch-import-corpus",
                    "--output-artifacts",
                    "/tmp/ail-manual-ui-patch-import-artifacts",
                ),
                evidence=(
                    "ui-patch-import-demo-report.txt",
                    "ui-patch-import-demo-report.fingerprint.txt",
                    "source-preserved true",
                    "proposed-accepted true",
                    "flow-edit-applied true",
                    "patched-core-replayed true",
                    "entry-count 125",
                    "checker-result-count accepted 116",
                    "checker-result-count rejected 9",
                ),
            ),
            ManualCommand(
                label="check-ui-patch-runtime-state",
                command=(
                    "python3",
                    "scripts/run_v03_ui_patch_runtime_state_check.py",
                    "--examples-artifacts",
                    "/tmp/ail-manual-ui-patch",
                    "--capture-plan-dir",
                    "/tmp/ail-manual-ui-patch-capture-plan",
                    "--import-work-dir",
                    "/tmp/ail-manual-ui-patch-import-work",
                    "--output-artifacts",
                    "/tmp/ail-manual-ui-patch-import-artifacts",
                    "--source-entry-id",
                    "example-108",
                    "--output-dir",
                    "/tmp/ail-manual-ui-patch-import-work",
                ),
                evidence=(
                    "ui-patch-runtime-state-check-report.txt",
                    "ui-patch-runtime-state-check-report.fingerprint.txt",
                    "visual-regression-baseline ui-review.txt",
                    "visual-regression-patch ui-review-patch.txt",
                    "visual-regression-fingerprint-preserved true",
                    "runtime-ui-state-check target-report",
                    "runtime-ui-state-anchor Ticket.reviewStatus",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="agent-policy-import",
        title="Agent Policy Import",
        doc="docs/ail/manual/09-agent-policy-import.md",
        purpose="Review deterministic AgentTool policy handoff artifacts and import a human-approved policy trace amendment into a corpus copy.",
        commands=(
            ManualCommand(
                label="replay-agent-policy-evidence",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-agent-policy",
                    "--release-evidence",
                ),
                evidence=(
                    "examples-report.txt",
                    "manifest.ail-examples.txt",
                    "agent-policy-review.txt",
                    "agent-policy-review-fingerprint-observed-count",
                    "handoff-roles requirements-writer,spec-writer,diagnostic-repairer,prompt-reviewer,agent-policy-reviewer",
                ),
            ),
            ManualCommand(
                label="inspect-agent-policy-lines",
                command=(
                    "rg",
                    "-n",
                    "agent-policy-review-fingerprint-observed-count|entry-artifact example-40 agent-policy-review",
                    "/tmp/ail-manual-agent-policy/examples-report.txt",
                    "/tmp/ail-manual-agent-policy/manifest.ail-examples.txt",
                ),
                evidence=(
                    "agent-policy-review-fingerprint-observed-count",
                    "entry-artifact example-40 agent-policy-review",
                ),
            ),
            ManualCommand(
                label="plan-agent-policy-capture",
                command=(
                    "python3",
                    "scripts/run_v03_agent_policy_capture_plan.py",
                    "--examples-artifacts",
                    "/tmp/ail-manual-agent-policy",
                    "--entry-id",
                    "example-40",
                    "--output-dir",
                    "/tmp/ail-manual-agent-policy-capture-plan",
                ),
                evidence=(
                    "agent-policy-capture-plan.json",
                    "agent-policy-capture-plan.txt",
                    "agent-policy-capture-plan.fingerprint.txt",
                    "agent-contract-check ail-agent-contracts examples/agents",
                    "handoff-roles requirements-writer,spec-writer,diagnostic-repairer,prompt-reviewer,agent-policy-reviewer",
                    "human-approval-required true",
                ),
            ),
            ManualCommand(
                label="demo-agent-policy-import",
                command=(
                    "python3",
                    "scripts/run_v03_agent_policy_import_demo.py",
                    "--base-corpus",
                    "examples",
                    "--examples-artifacts",
                    "/tmp/ail-manual-agent-policy",
                    "--capture-plan-dir",
                    "/tmp/ail-manual-agent-policy-capture-plan",
                    "--source-entry-id",
                    "example-40",
                    "--work-dir",
                    "/tmp/ail-manual-agent-policy-import-work",
                    "--output-corpus",
                    "/tmp/ail-manual-agent-policy-import-corpus",
                    "--output-artifacts",
                    "/tmp/ail-manual-agent-policy-import-artifacts",
                ),
                evidence=(
                    "agent-policy-import-demo-report.txt",
                    "agent-policy-import-demo-report.fingerprint.txt",
                    "source-preserved true",
                    "proposed-accepted true",
                    "policy-handoff-imported true",
                    "policy-handoff-replayed true",
                    "entry-count 125",
                    "checker-result-count accepted 116",
                    "checker-result-count rejected 9",
                ),
            ),
            ManualCommand(
                label="demo-agent-policy-multi-agent-handoff",
                command=(
                    "python3",
                    "scripts/run_v03_agent_policy_multi_agent_handoff.py",
                    "--examples-artifacts",
                    "/tmp/ail-manual-agent-policy",
                    "--capture-plan-dir",
                    "/tmp/ail-manual-agent-policy-capture-plan",
                    "--import-work-dir",
                    "/tmp/ail-manual-agent-policy-import-work",
                    "--output-artifacts",
                    "/tmp/ail-manual-agent-policy-import-artifacts",
                    "--source-entry-id",
                    "example-40",
                    "--output-dir",
                    "/tmp/ail-manual-agent-policy-import-work",
                ),
                evidence=(
                    "agent-policy-multi-agent-handoff-report.txt",
                    "agent-policy-multi-agent-handoff-report.fingerprint.txt",
                    "separate-reviewer-role-count 5",
                    "role requirements-writer contract codex-ail-requirements-writer",
                    "role spec-writer contract codex-ail-spec-writer",
                    "role diagnostic-repairer contract codex-ail-diagnostic-repairer",
                    "role prompt-reviewer contract codex-ail-prompt-reviewer",
                    "role agent-policy-reviewer contract codex-ail-agent-policy-reviewer",
                    "multi-agent-execution-evidence deterministic-role-handoff",
                ),
            ),
            ManualCommand(
                label="show-agent-policy-live-reviewer-command",
                command=(
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                    "--dry-run",
                ),
                evidence=(
                    "AIL-Agent-Policy-Live-Reviewer-Harness",
                    "role-count 5",
                    "artifact-kind AIL-AgentTool-Live-Reviewer-Handoff",
                ),
            ),
            ManualCommand(
                label="run-agent-policy-live-reviewers",
                command=(
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                ),
                live=True,
            ),
            ManualCommand(
                label="review-agent-policy-live-reviewer-artifacts",
                command=(
                    "python3",
                    "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                    "--review-artifacts",
                    "/tmp/ail-v03-agent-policy-live-review",
                ),
                live=True,
                evidence=(
                    "agent-policy-live-review-report.txt",
                    "agent-policy-live-review-report.fingerprint.txt",
                    "manifest.v03-agent-policy-live-review.txt",
                    "models.json",
                    "models.fingerprint.txt",
                    "agent-policy-live-review-review.txt",
                    "agent-policy-live-review-review.fingerprint.txt",
                    "agent-policy-live-review-repair-backlog.txt",
                    "agent-policy-live-review-repair-backlog.fingerprint.txt",
                    "model-check",
                    "model-check-model-count",
                    "model-check-model-id",
                    "reviewer-envelope-valid-count",
                    "reviewer-envelope-invalid-count",
                    "evidence-bundle-present-count",
                    "reviewer-decision-accept-count",
                    "reviewer-decision-needs-repair-count",
                    "reviewer-decision-reject-count",
                    "repair-backlog-fingerprint",
                ),
            ),
        ),
    ),
)

V03_AUTHORING_GATE = ManualChapter(
    chapter_id="v03-authoring-gate",
    title="v0.3 Authoring Gate",
    doc="docs/ail/manual/06-v03-authoring-gate.md",
    purpose=(
        "Run the deterministic story, examples, roadmap, prompt, agent, "
        "self-hosting, Systems, stateful runtime, Application baseline, "
        "and promotion checks as one v0.3 audit."
    ),
    commands=(
        ManualCommand(
            label="run-user-story-mode-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "user-story-mode",
                "--run-checks",
            ),
            evidence=(
                "story-mode-report.txt",
                "manifest.ail-story.txt",
                "story-prompt-envelope-valid-count",
                "story-prompt-envelope-invalid-count",
                "story-questions.ail-interview.md",
                "agent-trace.txt",
                "target.elf",
                "native-bytecode-report.txt",
                "dependency-report.txt",
                "manifest.ail-build.txt",
                "story-amendment-comparison.txt",
                "story-amendment-comparison.fingerprint.txt",
                "semantic-anchor-preserved-count 4",
                "semantic-anchor-preserved-count 5",
                "semantic-anchor-missing-count 0",
                "examples/incident_response.ail",
                "IncidentEscalated",
                "notification audit entry",
                "public timeline subscribers",
                "ticket.status=Closed",
                "trace TicketClosed",
            ),
        ),
        ManualCommand(
            label="run-examples-release-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "examples-release",
                "--run-checks",
            ),
            evidence=(
                "examples-report.txt",
                "v03-roadmap.txt",
                "manifest.ail-examples.txt",
                "model-executor-manifest.txt",
            ),
        ),
        ManualCommand(
            label="run-v03-roadmap-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "v03-roadmap",
                "--run-checks",
            ),
            evidence=(
                "AIL-v0.3-Roadmap",
                "v03-roadmap.txt",
                "v03-roadmap.fingerprint.txt",
            ),
        ),
        ManualCommand(
            label="run-prompt-interaction-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "prompt-interaction",
                "--run-checks",
            ),
            evidence=(
                "prompt-corpus-portability.txt",
                "manifest.ail-prompt-corpus.txt",
                "examples-report.txt",
            ),
        ),
        ManualCommand(
            label="run-agent-entrypoint-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "agent-entrypoint",
                "--run-checks",
            ),
            evidence=(
                "conformance-report.txt",
                "manifest.ail-conformance.txt",
                "accepted: bytecode-verification-minimal.ail-spec.md",
                "rejected: bytecode-verification-without-fingerprint.ail-spec.md AIL-AGENT-001",
                "agent.ailbc.json",
                "agent-trace.txt",
            ),
        ),
        ManualCommand(
            label="run-bootstrap-self-hosting-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "bootstrap-self-hosting",
                "--run-checks",
            ),
            evidence=(
                "bootstrap-fixed-point-report.txt",
                "fixed-point: ok",
                "second-pass-changed false",
                "bootstrap-pass-composition-report.txt",
                "composition-pass-count 1",
                "composition-pass 1 InferReadPermissions",
                "pass-order-status ok",
                "bootstrap-native-bytecode-report.txt",
                "bootstrap-host-boundary-report.txt",
                "no-host-backend-source true",
                "bootstrap-dependency-report.txt",
                "bootstrap-handoff-report.txt",
                "manifest.ail-bootstrap.txt",
            ),
        ),
        ManualCommand(
            label="run-systems-profile-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "systems-profile",
                "--run-checks",
            ),
            evidence=(
                "conformance-report.txt",
                "accepted: scheduler-task-minimal.ail-spec.md",
                "accepted: interrupt-context-minimal.ail-spec.md",
                "rejected: interrupt-context-blocking-effect.ail-spec.md AIL033",
                "rejected: scheduler-task-unknown-context.ail-spec.md AIL035",
                "native-bytecode-report.txt",
                "machine-bytecode-contract linux-x86_64-elf",
                "system effect read network device",
                "trace PacketReceived",
            ),
        ),
        ManualCommand(
            label="run-stateful-runtime-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "stateful-runtime",
                "--run-checks",
            ),
            evidence=(
                "conformance-report.txt",
                "manifest.ail-conformance.txt",
                "accepted: persistent-increment-minimal.ail-spec.md",
                "accepted: idempotent-increment-request-minimal.ail-spec.md",
                "accepted: locked-counter-increment-minimal.ail-spec.md",
                "accepted: replay-after-failure-minimal.ail-spec.md",
                "rejected: increment-without-persistence-guarantee.ail-spec.md AIL-STATE-001",
                "rejected: retryable-increment-without-idempotency-key.ail-spec.md AIL-STATE-002",
                "rejected: shared-counter-without-lock.ail-spec.md AIL-STATE-003",
                "rejected: failure-after-write-without-replay-policy.ail-spec.md AIL-STATE-004",
                "counter.value=42",
                "add counter.value by 1 -> 42",
            ),
        ),
        ManualCommand(
            label="run-application-baseline-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "application-baseline",
                "--run-checks",
            ),
            evidence=(
                "conformance-report.txt",
                "manifest.ail-conformance.txt",
                "accepted: close-ticket-minimal.ail-spec.md",
                "accepted: incident-escalation-minimal.ail-spec.md",
                "rejected: secret-leak.ail-spec.md AIL002",
                "rejected: action-without-trace.ail-spec.md AIL-TRACE-001",
                "rejected: unknown-field-type.ail-spec.md AIL-TYPE-001",
                "rejected: assignment-without-role-requirement.ail-spec.md AIL-APP-001",
                "rejected: overdue-without-time-requirement.ail-spec.md AIL-APP-002",
                "rejected: status-change-without-public-update.ail-spec.md AIL-APP-003",
                "rejected: notification-without-responder-pager.ail-spec.md AIL-APP-004",
                "rejected: resolve-without-mitigating-status.ail-spec.md AIL-APP-005",
                "rejected: postmortem-without-resolved-status.ail-spec.md AIL-APP-005",
                "rejected: private-notes-public-timeline-leak.ail-spec.md AIL-APP-006",
                "rejected: escalation-without-commander-review.ail-spec.md AIL-APP-007",
                "rejected: route-missing-permission.ail-spec.md AIL-UI-PERMISSION-002",
                "rejected: dashboard-missing-permission.ail-spec.md AIL-UI-PERMISSION-001",
                "ail conformance: ok",
            ),
        ),
        ManualCommand(
            label="run-repair-promotion-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "repair-promotion",
                "--run-checks",
            ),
            evidence=(
                "repair-promotion-review.txt",
                "repair-promotion-review.fingerprint.txt",
                "repair-promotion-review-fingerprint-observed-count",
                "repair-promotion-capture-plan.json",
                "repair-promotion-import-demo-report.txt",
            ),
        ),
        ManualCommand(
            label="run-ui-patch-import-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "ui-patch-import",
                "--run-checks",
            ),
            evidence=(
                "ui-review-patch.txt",
                "ui-review-patch-fingerprint-observed-count",
                "ui-patch-capture-plan.json",
                "ui-patch-import-demo-report.txt",
                "ui-patch-runtime-state-check-report.txt",
                "visual-regression-fingerprint-preserved true",
                "runtime-ui-state-check target-report",
                "runtime-ui-state-anchor Ticket.reviewStatus",
                "flow-edit-applied true",
                "patched-core-replayed true",
            ),
        ),
        ManualCommand(
            label="run-agent-policy-import-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "agent-policy-import",
                "--run-checks",
            ),
            evidence=(
                "agent-policy-review.txt",
                "agent-policy-review-fingerprint-observed-count",
                "agent-policy-capture-plan.json",
                "agent-policy-import-demo-report.txt",
                "agent-policy-multi-agent-handoff-report.txt",
                "policy-handoff-imported true",
                "policy-handoff-replayed true",
                "multi-agent-execution-evidence deterministic-role-handoff",
            ),
        ),
        ManualCommand(
            label="run-user-story-mode-live-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "user-story-mode",
                "--run-checks",
                "--include-live",
            ),
            live=True,
            evidence=(
                "story-llm-harness-report.txt",
                "story-llm-harness-report.fingerprint.txt",
                "story-mode-report.txt",
                "manifest.ail-story.txt",
                "model-check.json",
                "model-check.fingerprint.txt",
                "model-check",
                "model-check-model-count",
                "model-check-model-id",
                "agent-trace.txt",
                "agent-trace.fingerprint.txt",
                "story-llm-transcript-check-count",
                "story-prompt-envelope-valid-count",
                "story-prompt-envelope-artifact-count",
                "story-prompt-envelope-questions-count",
                "story-prompt-envelope-invalid-count",
                "story-promotion-capture-plan.json",
                "story-promotion-capture-plan.fingerprint.txt",
                "story-promotion-import-demo-report.txt",
                "story-promotion-import-demo-report.fingerprint.txt",
                "story-artifacts-preserved true",
                "proposed-accepted true",
                "capture-plan story-promotion-capture-plan.json",
                "promotion-decision accepted-for-promotion",
                "human-approval-required true",
                "promotion-source human-approved-story-promotion-batch",
                "human-approved-story-promotion-batch.fingerprint.txt",
                "batch-plan-fingerprint",
            ),
        ),
        ManualCommand(
            label="run-prompt-interaction-live-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "prompt-interaction",
                "--run-checks",
                "--include-live",
            ),
            live=True,
            evidence=(
                "prompt-llm-harness-report.txt",
                "prompt-llm-harness-review.txt",
                "prompt-llm-harness-review.fingerprint.txt",
                "manifest.v03-prompt-llm.txt",
                "models.json",
                "models.fingerprint.txt",
                "model-check",
                "model-check-model-count",
                "model-check-model-id",
                "prompt-envelope-valid-count",
                "prompt-envelope-artifact-required-count",
                "prompt-envelope-questions-expected-count",
                "prompt-outcome-match-count",
                "prompt-envelope-invalid-count",
            ),
        ),
        ManualCommand(
            label="run-agent-policy-import-live-checks",
            command=(
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "agent-policy-import",
                "--run-checks",
                "--include-live",
            ),
            live=True,
            evidence=(
                "agent-policy-live-review-report.txt",
                "agent-policy-live-review-report.fingerprint.txt",
                "agent-policy-live-review-review.txt",
                "agent-policy-live-review-review.fingerprint.txt",
                "manifest.v03-agent-policy-live-review.txt",
                "models.json",
                "models.fingerprint.txt",
                "model-check",
                "model-check-model-count",
                "model-check-model-id",
                "agent-policy-live-review-repair-backlog.txt",
                "agent-policy-live-review-repair-backlog.fingerprint.txt",
                "reviewer-envelope-valid-count",
                "reviewer-envelope-invalid-count",
                "evidence-bundle-present-count",
                "reviewer-decision-accept-count",
                "reviewer-decision-needs-repair-count",
                "reviewer-decision-reject-count",
                "repair-backlog-fingerprint",
            ),
        ),
    ),
)

CHAPTERS: tuple[ManualChapter, ...] = BASE_CHAPTERS + (V03_AUTHORING_GATE,)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="List, print, or run deterministic AIL interactive manual chapters."
    )
    parser.add_argument("--list", action="store_true", help="List manual chapters")
    parser.add_argument(
        "--all",
        action="store_true",
        help="Print or run all deterministic authoring chapters",
    )
    parser.add_argument("--chapter", help="Chapter id to print or run")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print chapter commands without executing them",
    )
    parser.add_argument(
        "--run-checks",
        action="store_true",
        help="Run deterministic non-live commands for the selected chapter",
    )
    parser.add_argument(
        "--include-live",
        action="store_true",
        help="Include live LLM/network commands when printing or running a chapter",
    )
    parser.add_argument(
        "--live-endpoint",
        help=(
            "Override the live LLM endpoint for manual live commands. "
            "Useful with a local fake endpoint in tests."
        ),
    )
    parser.add_argument(
        "--skip-model-check",
        action="store_true",
        help="Pass --skip-model-check to live LLM harnesses",
    )
    parser.add_argument(
        "--live-artifact-root",
        help=(
            "Rewrite known /tmp live/manual artifact paths under this root "
            "before printing or running commands."
        ),
    )
    return parser.parse_args(argv)


def chapter_by_id(chapter_id: str) -> ManualChapter:
    for chapter in CHAPTERS:
        if chapter.chapter_id == chapter_id:
            return chapter
    valid = ", ".join(chapter.chapter_id for chapter in CHAPTERS)
    raise SystemExit(f"unknown chapter {chapter_id}; valid chapters: {valid}")


def print_chapter_list() -> None:
    print("AIL-Interactive-Manual:")
    for chapter in CHAPTERS:
        print(f"chapter {chapter.chapter_id} {chapter.title}")
        print(f"doc {chapter.doc}")
        print(f"purpose {chapter.purpose}")


def chapter_commands(chapter: ManualChapter, include_live: bool) -> list[ManualCommand]:
    return [
        command for command in chapter.commands if include_live or not command.live
    ]


LIVE_ARTIFACT_PREFIXES: tuple[tuple[str, str], ...] = (
    ("/tmp/ail-v03-story-promotion-import-artifacts", "story-promotion-import-artifacts"),
    ("/tmp/ail-v03-story-promotion-import-corpus", "story-promotion-import-corpus"),
    ("/tmp/ail-v03-story-promotion-capture-plan", "story-promotion-capture-plan"),
    ("/tmp/ail-v03-story-promotion-import-work", "story-promotion-import-work"),
    ("/tmp/ail-v03-agent-policy-live-review", "agent-policy-live-review"),
    ("/tmp/ail-v03-story-llm", "story-llm"),
    ("/tmp/ail-v03-prompt-llm", "prompt-llm"),
    ("/tmp/ail-user-story-mode", "user-story-direct"),
    ("/tmp/ail-manual-agent-policy-import-artifacts", "agent-policy-import-artifacts"),
    ("/tmp/ail-manual-agent-policy-import-corpus", "agent-policy-import-corpus"),
    ("/tmp/ail-manual-agent-policy-capture-plan", "agent-policy-capture-plan"),
    ("/tmp/ail-manual-agent-policy-import-work", "agent-policy-import-work"),
    ("/tmp/ail-manual-agent-policy", "agent-policy"),
)


def replace_artifact_path(value: str, artifact_root: str | None) -> str:
    if not artifact_root:
        return value
    for prefix, relative in sorted(
        LIVE_ARTIFACT_PREFIXES, key=lambda item: len(item[0]), reverse=True
    ):
        if value == prefix or value.startswith(prefix + "/"):
            suffix = value[len(prefix):].lstrip("/")
            rewritten = Path(artifact_root) / relative
            if suffix:
                rewritten = rewritten / suffix
            return str(rewritten)
    if value.startswith("/tmp/ail-manual-"):
        return str(Path(artifact_root) / value.removeprefix("/tmp/ail-manual-"))
    return value


def set_option(args: list[str], option: str, value: str) -> None:
    if option in args:
        index = args.index(option)
        if index + 1 >= len(args):
            raise SystemExit(f"{option} requires a value")
        args[index + 1] = value
    else:
        args.extend([option, value])


def set_flag(args: list[str], flag: str) -> None:
    if flag not in args:
        args.append(flag)


def add_live_harness_options(args: list[str], overrides: LiveOverrides) -> None:
    if overrides.endpoint:
        set_option(args, "--endpoint", overrides.endpoint)
    if overrides.skip_model_check:
        set_flag(args, "--skip-model-check")


def materialize_command(command: ManualCommand, overrides: LiveOverrides) -> tuple[str, ...]:
    args = [
        replace_artifact_path(part, overrides.artifact_root)
        for part in command.command
    ]
    if len(args) >= 2 and args[0] == "python3":
        script = args[1]
        if script == "scripts/run_ail_interactive_manual.py" and "--include-live" in args:
            if overrides.endpoint:
                set_option(args, "--live-endpoint", overrides.endpoint)
            if overrides.skip_model_check:
                set_flag(args, "--skip-model-check")
            if overrides.artifact_root:
                set_option(args, "--live-artifact-root", overrides.artifact_root)
        elif script == "scripts/run_v03_story_llm_harness.py":
            if "--review-artifacts" not in args:
                add_live_harness_options(args, overrides)
                if overrides.artifact_root:
                    set_option(
                        args,
                        "--artifact-dir",
                        str(Path(overrides.artifact_root) / "story-llm"),
                    )
        elif script == "scripts/run_v03_prompt_llm_harness.py":
            if "--review-artifacts" not in args:
                add_live_harness_options(args, overrides)
                if overrides.artifact_root:
                    set_option(
                        args,
                        "--artifact-dir",
                        str(Path(overrides.artifact_root) / "prompt-llm"),
                    )
            elif overrides.skip_model_check:
                set_flag(args, "--allow-skipped-model-check")
        elif script == "scripts/run_v03_agent_policy_live_reviewer_harness.py":
            if "--review-artifacts" not in args:
                add_live_harness_options(args, overrides)
                if overrides.artifact_root:
                    root = Path(overrides.artifact_root)
                    set_option(args, "--artifact-dir", str(root / "agent-policy-live-review"))
                    set_option(args, "--examples-artifacts", str(root / "agent-policy"))
                    set_option(args, "--capture-plan-dir", str(root / "agent-policy-capture-plan"))
                    set_option(args, "--import-work-dir", str(root / "agent-policy-import-work"))
            elif overrides.skip_model_check:
                set_flag(args, "--allow-skipped-model-check")
    elif command.live and len(args) >= 4 and args[:3] == ["cargo", "run", "--"]:
        if "ail-story" in args and overrides.endpoint:
            set_option(args, "--llm-endpoint", overrides.endpoint)
    return tuple(args)


def shell_line(command: ManualCommand, overrides: LiveOverrides) -> str:
    return " ".join(materialize_command(command, overrides))


def print_chapter(
    chapter: ManualChapter, include_live: bool, overrides: LiveOverrides
) -> None:
    print("AIL-Interactive-Manual-Chapter:")
    print(f"id {chapter.chapter_id}")
    print(f"title {chapter.title}")
    print(f"doc {chapter.doc}")
    print(f"purpose {chapter.purpose}")
    for index, command in enumerate(chapter_commands(chapter, include_live), start=1):
        print(f"step {index} {command.label}")
        print(f"live {str(command.live).lower()}")
        print(shell_line(command, overrides))
        for evidence in command.evidence:
            print(f"evidence {evidence}")


def print_runbook(include_live: bool, overrides: LiveOverrides) -> None:
    print("AIL-Interactive-Manual-Runbook:")
    for chapter in CHAPTERS:
        print(f"chapter {chapter.chapter_id} {chapter.title}")
        print(f"doc {chapter.doc}")
        print(f"purpose {chapter.purpose}")
        for index, command in enumerate(chapter_commands(chapter, include_live), start=1):
            print(f"step {chapter.chapter_id}.{index} {command.label}")
            print(f"live {str(command.live).lower()}")
            print(shell_line(command, overrides))
            for evidence in command.evidence:
                print(f"evidence {evidence}")


def run_chapter_checks(
    chapter: ManualChapter, include_live: bool, overrides: LiveOverrides
) -> int:
    commands = chapter_commands(chapter, include_live)
    if not commands:
        print(f"chapter {chapter.chapter_id} has no runnable commands")
        return 0
    for command in commands:
        materialized = materialize_command(command, overrides)
        print(f"running {command.label}: {' '.join(materialized)}")
        for evidence in command.evidence:
            print(f"evidence {evidence}")
        completed = subprocess.run(materialized, check=False)
        if completed.returncode != 0:
            return completed.returncode
    return 0


def run_all_chapter_checks(include_live: bool, overrides: LiveOverrides) -> int:
    for chapter in BASE_CHAPTERS:
        print(f"chapter {chapter.chapter_id}")
        status = run_chapter_checks(chapter, include_live, overrides)
        if status != 0:
            return status
    return 0


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    overrides = LiveOverrides(
        endpoint=args.live_endpoint,
        skip_model_check=args.skip_model_check,
        artifact_root=args.live_artifact_root,
    )
    if args.all and args.chapter:
        raise SystemExit("--all cannot be used with --chapter")
    if args.list or (not args.chapter and not args.all):
        print_chapter_list()
        if args.list and not (args.all or args.chapter):
            return 0
        if not args.chapter and not args.all:
            return 0
    if args.all:
        if args.run_checks:
            return run_all_chapter_checks(args.include_live, overrides)
        print_runbook(args.include_live or args.dry_run, overrides)
        return 0
    chapter = chapter_by_id(args.chapter)
    if args.run_checks:
        return run_chapter_checks(chapter, args.include_live, overrides)
    print_chapter(chapter, args.include_live or args.dry_run, overrides)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
