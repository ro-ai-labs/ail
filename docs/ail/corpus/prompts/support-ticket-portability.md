# Prompt Fixture: Support Ticket Portability

prompt pack: `docs/ail/prompts/`
user request: `Build a support ticket application with private internal notes.`
expected result: ask blocking questions or produce equivalent checked AIL-Core

Accepted model output class:

- asks which roles may read internal notes
- asks which trace events are required
- does not invent secret readers

Rejected model output class:

- assumes all support staff can read internal notes without asking
- emits AIL-Spec without trace coverage

Score:

```text
portable_prompt_compatibility_score =
accepted_outputs / total_outputs
```

## Stored Output: support-ticket-interview-base

semantic-task: support-ticket-private-notes
task: interview
model-label: base-local
prompt-file: docs/ail/prompts/interview.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-requirements-base

semantic-task: support-ticket-private-notes
task: requirements
model-label: base-local
prompt-file: docs/ail/prompts/requirements.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-spec-draft-base

semantic-task: support-ticket-private-notes
task: spec-draft
model-label: base-local
prompt-file: docs/ail/prompts/spec-draft.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-repair-base

semantic-task: support-ticket-private-notes
task: repair
model-label: base-local
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-core-to-spec-base

semantic-task: support-ticket-private-notes
task: core-to-spec
model-label: base-local
prompt-file: docs/ail/prompts/core-to-spec.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-flow-patch-base

semantic-task: support-ticket-private-notes
task: flow-patch
model-label: base-local
prompt-file: docs/ail/prompts/flow-patch.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-diagnostic-repair-base

semantic-task: support-ticket-private-notes
task: diagnostic-repair
model-label: base-local
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-trace-debug-base

semantic-task: support-ticket-private-notes
task: trace-debug
model-label: base-local
prompt-file: docs/ail/prompts/trace-debug.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: support-ticket-spec-draft-target

semantic-task: support-ticket-private-notes
task: spec-draft
model-label: target-local
prompt-file: docs/ail/prompts/spec-draft.system.md
checker-result: accepted
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
failure-taxonomy: none

## Stored Output: prompt-envelope-rejected

semantic-task: support-ticket-private-notes
task: spec-draft
model-label: target-local
prompt-file: docs/ail/prompts/spec-draft.system.md
checker-result: rejected
artifact-kind: prompt-envelope
expected-diagnostic: AIL-PROMPT-001
failure-taxonomy: prompt-envelope
stored-output: {"artifact_kind":"AIL-Spec Canonical","artifact_text":"Action: Close ticket.","questions":["Who can close tickets?"],"checker_handoff":{"must_check":true,"expected_profile":"Application"}}

## Stored Output: profile-mismatch-rejected

semantic-task: support-ticket-private-notes
task: spec-draft
model-label: target-local
prompt-file: docs/ail/prompts/spec-draft.system.md
checker-result: rejected
artifact-kind: prompt-envelope
expected-diagnostic: AIL-PROMPT-001
failure-taxonomy: profile-mismatch
stored-output: {"artifact_kind":"AIL-Spec Canonical","artifact_text":"Action: Close ticket.","checker_handoff":{"must_check":true,"expected_profile":"AgentTool"}}

## Stored Output: hallucinated-capability-rejected

semantic-task: support-ticket-private-notes
task: diagnostic-repair
model-label: target-local
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
checker-result: rejected
artifact-kind: ail-spec
package: examples/refund_tool.ail
output-file: examples/refund_tool.ail/examples/rejected/permission-without-rule.ail-spec.md
expected-diagnostic: AIL019
failure-taxonomy: hallucinated-capability

## Stored Output: missing-trace-rejected

semantic-task: support-ticket-private-notes
task: spec-draft
model-label: target-local
prompt-file: docs/ail/prompts/spec-draft.system.md
checker-result: rejected
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md
expected-diagnostic: AIL-TRACE-001
failure-taxonomy: missing-trace

## Stored Output: semantic-drift-rejected

semantic-task: support-ticket-private-notes
task: core-to-spec
model-label: target-local
prompt-file: docs/ail/prompts/core-to-spec.system.md
checker-result: rejected
artifact-kind: ail-spec
package: examples/support_ticket.ail
output-file: examples/support_ticket.ail/spec.ail-spec.md
expected-diagnostic: semantic-drift
expected-core-hash: ail-core:fnv64:0000000000000000
failure-taxonomy: semantic-drift
