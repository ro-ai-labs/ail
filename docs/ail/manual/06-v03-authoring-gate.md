# AIL Manual: v0.3 Authoring Gate

## Purpose

The v0.3 authoring gate chapter runs the deterministic audit that ties the
manual together. It proves the current story-first workflow, examples replay,
roadmap printing, prompt interaction checks, agent entrypoint checks, bootstrap
self-hosting, Turing Core recursion checks, Systems profile, stateful runtime,
Application baseline, repair promotion, UI patch import, and AgentTool policy
import checks can be executed from one command.

Run the gate:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks
```

## Checks

The gate runs these manual steps:

```text
run-user-story-mode-checks
run-examples-release-checks
run-v03-roadmap-checks
run-prompt-interaction-checks
run-agent-entrypoint-checks
run-bootstrap-self-hosting-checks
run-turing-core-checks
run-systems-profile-checks
run-stateful-runtime-checks
run-application-baseline-checks
run-repair-promotion-checks
run-ui-patch-import-checks
run-agent-policy-import-checks
```

These are wrappers around the individual chapters:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter v03-roadmap --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter bootstrap-self-hosting --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter turing-core --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter systems-profile --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter stateful-runtime --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter application-baseline --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks
```

Live hosted evidence remains opt-in. When the llama.cpp server is reachable,
include the live User Story mode review, live prompt interaction review, and
live AgentTool policy reviewer evidence in the gate dry-run or execution:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --dry-run --include-live
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks --include-live
```

For a local fake LLM endpoint or a non-default hosted server, keep the same
chapter shape and override the live transport at the manual-runner boundary:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --dry-run --include-live \
  --live-endpoint http://127.0.0.1:8081/v1/chat/completions \
  --skip-model-check \
  --live-artifact-root /tmp/ail-manual-live-local
```

The gate forwards those flags to its live User Story mode, prompt interaction,
and AgentTool policy chapters, and those chapters forward them to the concrete
story, prompt-pack, reviewer, and direct `ail-story` commands.

The live gate delegates to:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks --include-live
```

## Evidence

The gate should surface:

```text
story-mode-report.txt
story-llm-harness-report.txt
story-llm-harness-report.fingerprint.txt
story-llm-transcript-check-count
story-prompt-envelope-valid-count
story-prompt-envelope-artifact-count
story-prompt-envelope-questions-count
story-prompt-envelope-invalid-count
codex-ail-story-promotion-reviewer.md
examples/agents/skills/ail-story-promotion-reviewer/SKILL.md
story-promotion-capture-plan.json
story-promotion-capture-plan.txt
story-promotion-capture-plan.fingerprint.txt
story-promotion-import-demo-report.txt
story-promotion-import-demo-report.fingerprint.txt
capture-plan story-promotion-capture-plan.json
promotion-decision accepted-for-promotion
human-approval-required true
promotion-source human-approved-story-promotion-batch
human-approved-story-promotion-batch.fingerprint.txt
batch-plan-fingerprint
story-amendment-comparison.txt
story-amendment-comparison.fingerprint.txt
story-amendment-comparison: present
semantic-anchor-preserved-count 4
semantic-anchor-missing-count 0
target.elf
native-bytecode-report.txt
dependency-report.txt
manifest.ail-build.txt
ticket.status=Closed
trace TicketClosed
manifest.ail-story.txt
story-questions.ail-interview.md
agent-trace.txt
agent-trace.fingerprint.txt
examples-report.txt
v03-roadmap.txt
manifest.ail-examples.txt
ui-review.txt
ui-review-fingerprint-observed-count
prompt-corpus-portability.txt
manifest.ail-prompt-corpus.txt
prompt-llm-harness-report.txt
prompt-llm-harness-review.txt
prompt-llm-harness-review.fingerprint.txt
manifest.v03-prompt-llm.txt
prompt-envelope-valid-count
prompt-envelope-artifact-required-count
prompt-envelope-questions-expected-count
prompt-outcome-match-count
prompt-envelope-invalid-count
agent.ailbc.json
accepted: bytecode-verification-minimal.ail-spec.md
rejected: bytecode-verification-without-fingerprint.ail-spec.md AIL-AGENT-001
bootstrap-fixed-point-report.txt
bootstrap-fixed-point-report.fingerprint.txt
fixed-point: ok
second-pass-changed false
bootstrap-pass-composition-report.txt
bootstrap-pass-composition-report.fingerprint.txt
composition-pass-count 1
composition-pass 1 InferReadPermissions
pass-order-status ok
bootstrap-native-bytecode-report.txt
bootstrap-host-boundary-report.txt
no-host-backend-source true
bootstrap-dependency-report.txt
bootstrap-handoff-report.txt
manifest.ail-bootstrap.txt
accepted: recursive-with-stack-bound.ail-spec.md
accepted: recursive-with-well-founded-measure.ail-spec.md
rejected: recursive-without-base-case.ail-spec.md AIL-CONTROL-003
rejected: recursive-without-decreasing-argument.ail-spec.md AIL-CONTROL-003
conformance-report.txt
manifest.ail-conformance.txt
accepted: scheduler-task-minimal.ail-spec.md
accepted: interrupt-context-minimal.ail-spec.md
rejected: interrupt-context-blocking-effect.ail-spec.md AIL033
rejected: scheduler-task-unknown-context.ail-spec.md AIL035
accepted: close-ticket-minimal.ail-spec.md
accepted: incident-escalation-minimal.ail-spec.md
rejected: secret-leak.ail-spec.md AIL002
rejected: action-without-trace.ail-spec.md AIL-TRACE-001
rejected: failure-without-trace.ail-spec.md AIL-TRACE-002
rejected: unknown-field-type.ail-spec.md AIL-TYPE-001
rejected: assignment-without-role-requirement.ail-spec.md AIL-APP-001
rejected: overdue-without-time-requirement.ail-spec.md AIL-APP-002
rejected: status-change-without-public-update.ail-spec.md AIL-APP-003
rejected: notification-without-responder-pager.ail-spec.md AIL-APP-004
rejected: resolve-without-mitigating-status.ail-spec.md AIL-APP-005
rejected: postmortem-without-resolved-status.ail-spec.md AIL-APP-005
rejected: private-notes-public-timeline-leak.ail-spec.md AIL-APP-006
rejected: escalation-without-commander-review.ail-spec.md AIL-APP-007
rejected: route-missing-permission.ail-spec.md AIL-UI-PERMISSION-002
rejected: dashboard-missing-permission.ail-spec.md AIL-UI-PERMISSION-001
checked.ail-core.txt
artifact.ailbc.json
native-bytecode-report.txt
dependency-report.txt
manifest.ail-compile.txt
machine-bytecode-contract linux-x86_64-elf
system effect read network device
trace PacketReceived
accepted: persistent-increment-minimal.ail-spec.md
accepted: idempotent-increment-request-minimal.ail-spec.md
accepted: locked-counter-increment-minimal.ail-spec.md
accepted: replay-after-failure-minimal.ail-spec.md
rejected: increment-without-persistence-guarantee.ail-spec.md AIL-STATE-001
rejected: retryable-increment-without-idempotency-key.ail-spec.md AIL-STATE-002
rejected: shared-counter-without-lock.ail-spec.md AIL-STATE-003
rejected: failure-after-write-without-replay-policy.ail-spec.md AIL-STATE-004
counter.value=42
add counter.value by 1 -> 42
repair-promotion-review.txt
repair-promotion-review.fingerprint.txt
repair-promotion-review-fingerprint-observed-count
repair-promotion-capture-plan.json
repair-promotion-capture-plan.txt
repair-promotion-capture-plan.fingerprint.txt
ui-patch-capture-plan.json
ui-patch-capture-plan.txt
ui-patch-capture-plan.fingerprint.txt
ui-patch-import-demo-report.txt
ui-patch-import-demo-report.fingerprint.txt
ui-patch-runtime-state-check-report.txt
ui-patch-runtime-state-check-report.fingerprint.txt
visual-regression-fingerprint-preserved true
runtime-ui-state-check target-report
runtime-ui-state-anchor Ticket.reviewStatus
flow-edit-applied true
patched-core-replayed true
agent-policy-capture-plan.json
agent-policy-capture-plan.txt
agent-policy-capture-plan.fingerprint.txt
agent-policy-import-demo-report.txt
agent-policy-import-demo-report.fingerprint.txt
agent-policy-multi-agent-handoff-report.txt
agent-policy-multi-agent-handoff-report.fingerprint.txt
agent-policy-live-review-report.txt
agent-policy-live-review-report.fingerprint.txt
manifest.v03-agent-policy-live-review.txt
agent-policy-live-review-review.txt
agent-policy-live-review-review.fingerprint.txt
reviewer-envelope-valid-count
reviewer-envelope-invalid-count
evidence-bundle-present-count
reviewer-decision-accept-count
reviewer-decision-needs-repair-count
reviewer-decision-reject-count
policy-handoff-imported true
policy-handoff-replayed true
multi-agent-execution-evidence deterministic-role-handoff
```

Passing this chapter is not the same as declaring AIL v0.3 complete. It is the
current deterministic authoring audit used to decide which missing behavior
should be implemented next.
