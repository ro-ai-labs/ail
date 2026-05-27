# AIL Diagnostics Catalog

## Purpose

Diagnostics are language artifacts. Each checker rule has a stable diagnostic
ID, a condition, affected graph item, message template, non-engineer
explanation, agent follow-up question, repair suggestion, AIL-Flow highlight,
severity, blocking behavior, and at least one invalid fixture.

## Severity Levels

- `error`: blocks artifact acceptance
- `warning`: accepted artifact with visible review warning
- `info`: non-blocking explanation or portability note

## Diagnostic Schema

```json
{
  "code": "AIL-TRACE-001",
  "severity": "error",
  "blocking": true,
  "condition": "executable action has no records_trace edge",
  "affected": "node:Action",
  "message": "{kind} {name} is missing trace coverage",
  "non_engineer_explanation": "This action can run, but reviewers would not be able to see that it happened.",
  "agent_follow_up": "Which trace event should be recorded when this action runs?",
  "repair": "Add a trace event and attach it with records_trace.",
  "flow_highlight": "ActionCard.trace-section",
  "invalid_fixture": "examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md"
}
```

## Catalog

| Code | Primary rule | Condition | Severity | Blocking | Repair |
| --- | --- | --- | --- | --- | --- |
| `AIL-TYPE-001` | `ail.core.type.resolves` | unknown type in field, value, input, or output | error | yes | declare or import the type |
| `AIL-SCHEMA-001` | `ail.core.node.kind-known` | unknown AIL-Core node kind | error | yes | use a node kind declared in `ail-core.schema.v0` |
| `AIL-SCHEMA-002` | `ail.core.edge.kind-known` | unknown AIL-Core edge kind | error | yes | use an edge kind declared in `ail-core.schema.v0` |
| `AIL-TRACE-001` | `ail.spec.action.requires-trace` | executable action or tool lacks trace coverage | error | yes | add `records_trace` |
| `AIL-TRACE-002` | `ail.spec.failure.requires-trace` | failure lacks trace coverage | error | yes | add failure trace |
| `AIL-FAILURE-001` | `ail.spec.failure.requires-handler` | declared blocking failure has no handler | error | yes | add handler or classify as propagated |
| `AIL-APP-001` | `ail.application.assignment.requires-support-role` | assignment changes assignee without support-role validation | error | yes | add assignee support-role requirement |
| `AIL-APP-002` | `ail.application.overdue.requires-current-time` | overdue status transition lacks due-time comparison | error | yes | add current-time requirement |
| `AIL-APP-003` | `ail.application.status.requires-public-update` | ticket status transition omits public update | error | yes | record a customer-visible public update |
| `AIL-STATE-001` | `ail.application.state.persistence-guaranteed` | persistent counter mutation lacks persistence guarantee | error | yes | add durable persistence guarantee |
| `AIL-STATE-002` | `ail.application.state.retry-idempotent` | retryable counter mutation lacks idempotency key | error | yes | add request id or dedupe state |
| `AIL-STATE-003` | `ail.application.state.shared-serialized` | shared counter mutation lacks lock or serialization rule | error | yes | add lock guard or serialization guarantee |
| `AIL-STATE-004` | `ail.application.state.replay-policy` | failure after counter write lacks replay recovery policy | error | yes | add rollback, resume, or idempotent replay guarantee |
| `AIL-WORKFLOW-001` | `ail.application.workflow.temporal-policy` | repeated action claims scheduler behavior without temporal policy | error | yes | add a temporal policy or remove the scheduler claim |
| `AIL-SECRET-READ-001` | `ail.core.secret-read.requires-protection` | secret read lacks explicit protection | error | yes | add permission and secret protection |
| `AIL-SECRET-ROLE-001` | `ail.application.secret-read.requires-support-role` | secret internal notes read lacks support-role requirement | error | yes | add SupportAgent or SupportManager role requirement |
| `AIL-SECRET-WRITE-001` | `ail.core.secret-write.requires-redaction` | secret write lacks redaction or protection | error | yes | add redaction policy |
| `AIL-SECRET-OUTPUT-001` | `ail.tool.output.secret-requires-approval` | tool output exposes a secret without reveal permission | error | yes | remove secret output or add reveal approval |
| `AIL-PERMISSION-001` | `ail.tool.permission.requires-rule` | permission reference has no rule or scope | error | yes | attach rule and scope |
| `AIL-APPROVAL-001` | `ail.tool.approval.requires-rule` | approval has no triggering rule | error | yes | attach rule and trace |
| `AIL-AGENT-AUDIT-001` | `ail.agent-tool.provider-call.audit-evidence` | AgentTool external provider call lacks audit evidence | error | yes | add an audit write or audit-trace guarantee |
| `AIL-AGENT-FAILURE-001` | `ail.agent-tool.provider-call.failure-policy` | AgentTool external provider call lacks a declared provider failure | error | yes | add a provider Failure section |
| `AIL-AGENT-RECOVERY-001` | `ail.agent-tool.provider-failure.recovery-policy` | provider failure lacks retry, fallback, queue, escalation, or human-review recovery | error | yes | add recovery handling to the provider Failure section |
| `AIL-CONTROL-001` | `ail.runtime.branch.exhaustive` | branch has no matching outcome or else | error | yes | add exhaustive outcome |
| `AIL-CONTROL-002` | `ail.runtime.match.exhaustive` | match over finite variants is non-exhaustive | error | yes | cover every variant |
| `AIL-CONTROL-003` | `ail.runtime.termination.proven` | termination-required profile has unproven recursion or loop | error | yes | add proof, bound, or profile policy |
| `AIL-FFI-OWNERSHIP-001` | `ail.ffi.pointer.borrowed-no-escape` | borrowed C pointer escapes call boundary | error | yes | use owned pointer or remove escape |
| `AIL-FFI-OWNERSHIP-002` | `ail.ffi.pointer.owned-release` | owned C pointer lacks release semantics | error | yes | add release semantics |
| `AIL-FFI-NULL-001` | `ail.ffi.pointer.non-null` | nullable value flows into NonNull pointer contract | error | yes | use nullable type or remove nullable marker |
| `AIL-FFI-ALIAS-001` | `ail.ffi.pointer.mutable-alias` | multiple mutable borrowed pointers share an alias group | error | yes | split alias group or pass one mutable pointer |
| `AIL-FFI-SECRET-001` | `ail.ffi.secret.boundary` | secret value crosses foreign boundary without redaction semantics | error | yes | remove secret type or mark boundary redacted |
| `AIL-FFI-ERRNO-001` | `ail.ffi.errno.mapped` | C error code or errno is unmapped | error | yes | map to declared failure |
| `AIL-SYSTEM-CAP-001` | `ail.system.effect.requires-capability` | system effect lacks matching capability | error | yes | add capability or remove effect |
| `AIL-SYSTEM-REGION-001` | `ail.system.resource.requires-region` | resource effect lacks region placement | error | yes | declare resource region |
| `AIL-BACKEND-001` | `ail.backend.effect.supported` | target does not support requested effect | error | yes | choose target support or remove effect |
| `AIL-BACKEND-002` | `ail.backend.target-support.status-known` | target-support status label is unknown | error | yes | use a known target-support status |
| `AIL-PROMPT-001` | `ail.prompt.envelope.valid` | agent output violates prompt envelope schema | error | yes | regenerate with required schema |
| `AILR011` | `ail.spec.requirements.permission-preserved` | spec draft drops a permission-like requirement from AIL-Requirements | error | yes | add an explicit action requirement |
| `AILR012` | `ail.spec.requirements.failure-preserved` | spec draft drops a required Failure section from AIL-Requirements | error | yes | add the named Failure section |
| `AILR013` | `ail.spec.requirements.trace-preserved` | spec draft drops a required trace event from AIL-Requirements | error | yes | add the named trace event |
| `AIL-ROUNDTRIP-001` | `ail.projection.roundtrip.preserves-core` | projection round trip changes graph hash | error | yes | repair projection or patch |
| `AIL-UI-A11Y-001` | `ail.ui.action.accessible-name` | reachable UI action lacks accessible name | error | yes | add accessible label |
| `AIL-UI-FORM-001` | `ail.ui.form.action-resolves` | form calls an undeclared action | error | yes | declare the action before the form calls it |
| `AIL-UI-PERMISSION-001` | `ail.ui.dashboard.permission-required` | dashboard reads data without permission | error | yes | add dashboard permission |
| `AIL-UI-CONFIRM-001` | `ail.ui.form.destructive-confirmation` | form exposes destructive action without confirmation | error | yes | add form confirmation |
| `AIL-UI-WORKFLOW-001` | `ail.ui.workflow.step-order` | blocked workflow step appears before its prerequisite | error | yes | move blocked step after prerequisite |

## Detailed Entries

### AIL-TYPE-001

- condition: field, input, output, value, or resource type is not known
- affected graph item: typed node
- message template: `{kind} {name} has unknown type '{type}'`
- non-engineer explanation: the program names a data type that AIL cannot
  validate or carry through the compiler
- agent follow-up question: `Should {type} be declared as a Thing, imported
  from a package, or replaced with a supported type?`
- repair suggestion: use a supported AIL type or declare/import the missing
  type
- AIL-Flow highlight: Data Table field, Tool input/output, Compiler value, or
  System resource type
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/unknown-field-type.ail-spec.md`

### AIL-TRACE-001

- condition: executable action or tool has no `records_trace` edge
- affected graph item: `Action` or `Tool`
- message template: `action {name} is missing trace coverage` or
  `tool {name} is missing audit trace coverage`
- non-engineer explanation: reviewers cannot tell that the action ran
- agent follow-up question: `What event should the system record when {name} runs?`
- repair suggestion: add a named trace event and attach it to the action
- AIL-Flow highlight: Action Card trace section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md`

### AIL-TRACE-002

- condition: declared failure has no `records_trace` edge
- affected graph item: `Failure`
- message template: `failure {name} is missing trace coverage`
- non-engineer explanation: reviewers cannot tell which failure happened
- agent follow-up question: `What event should the system record when {name} happens?`
- repair suggestion: add a trace bullet to the Failure section
- AIL-Flow highlight: Failure trace section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/failure-without-trace.ail-spec.md`

### AIL-FAILURE-001

- condition: declared blocking failure has no handling bullet
- affected graph item: `Failure`
- message template: `failure {name} is missing declared handling`
- non-engineer explanation: the program names a failure case but does not say
  what the system should do when it happens
- agent follow-up question: `How should the system handle {name}?`
- repair suggestion: add at least one handling bullet to the Failure section
- AIL-Flow highlight: Failure handling section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/failure-without-handling.ail-spec.md`

### AIL-AGENT-AUDIT-001

- condition: an `AgentTool` tool calls an external provider but has no checker
  visible audit write or audit-trace guarantee
- affected graph item: `calls` edge from the tool to the external provider
  call
- message template: `tool {tool} calls {provider_call} without audit evidence`
- non-engineer explanation: the agent can ask an external service to notify or
  act, but reviewers would not have a durable audit record for that provider
  call
- agent follow-up question: `Which audit entry or trace guarantee proves that
  {provider_call} happened?`
- repair suggestion: add an audit write or audit-trace guarantee for the
  provider call
- AIL-Flow highlight: Tool Card effects and audit trace sections
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/incident_notifications.ail/examples/rejected/provider-call-without-audit-entry.ail-spec.md`

### AIL-AGENT-FAILURE-001

- condition: an `AgentTool` tool calls an external provider but has no
  declared provider failure section attached to the tool
- affected graph item: `calls` edge from the tool to the external provider
  call
- message template: `tool {tool} calls {provider_call} without provider
  failure policy`
- non-engineer explanation: the agent can ask an external service to act, but
  the spec does not say what happens if that provider rejects, times out, or
  fails delivery
- agent follow-up question: `What named failure should be recorded if
  {provider_call} rejects, times out, or fails?`
- repair suggestion: add a `Failure ... happens when ...` section for the
  provider call with handling and trace bullets
- AIL-Flow highlight: Tool Card failure section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/incident_notifications.ail/examples/rejected/provider-call-without-failure-policy.ail-spec.md`

### AIL-AGENT-RECOVERY-001

- condition: an `AgentTool` provider failure has handling and trace coverage,
  but the handling does not describe retry, fallback, queueing, escalation, or
  human-review recovery
- affected graph item: provider `Failure` node
- message template: `failure {failure} for tool {tool} has no recovery policy
  for {provider_call}`
- non-engineer explanation: reviewers can see that the provider failed, but
  they cannot see how the agent or system should recover
- agent follow-up question: `Should {failure} retry, queue work, fall back,
  escalate, or send the case for human review?`
- repair suggestion: add retry, fallback, queue, escalation, or human-review
  handling to the provider Failure section
- AIL-Flow highlight: Failure Card handling section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/incident_notifications.ail/examples/rejected/provider-failure-without-retry-policy.ail-spec.md`

### AIL-CONTROL-003

- condition: a self-recursive function has no checker-visible base-case branch
  plus decreasing recursive argument, and no explicit numeric stack or
  termination bound
- affected graph item: `Function`
- message template: `function {name} has unproven recursive termination`
- non-engineer explanation: the checker cannot prove that the function will
  stop calling itself
- agent follow-up question: `What base case or bound proves that {name}
  terminates?`
- repair suggestion: add a base-case branch return, a decreasing recursive
  argument, or an explicit stack/termination bound such as
  `the function has a maximum recursion depth of 64`
- AIL-Flow highlight: Function recursion section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixtures:
  `examples/recursive_factorial.ail/examples/rejected/recursive-without-base-case.ail-spec.md`,
  `examples/recursive_factorial.ail/examples/rejected/recursive-without-decreasing-argument.ail-spec.md`

### AIL-APP-001

- condition: an Application action writes `Ticket.assignee` without a
  requirement that constrains the assignee support role
- affected graph item: `writes` edge from the action to `Ticket.assignee`
- message template: `action {name} writes Ticket.assignee without a
  support-role requirement`
- non-engineer explanation: the workflow can assign a ticket to someone who is
  not support staff
- agent follow-up question: `Which roles are allowed to receive assigned
  tickets?`
- repair suggestion: add an assignee role requirement such as
  `the assignee role to be SupportAgent or SupportManager`
- AIL-Flow highlight: Action Card requirements section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/assignment-without-role-requirement.ail-spec.md`

### AIL-APP-002

- condition: an Application action writes `Ticket.status` to `Overdue` without
  comparing current time to the ticket due time
- affected graph item: `writes` edge from the action to `Ticket.status`
- message template: `action {name} writes Ticket.status to Overdue without a
  current-time requirement`
- non-engineer explanation: the scheduler can mark a ticket overdue without
  checking whether the due time has passed
- agent follow-up question: `What time condition must be true before the ticket
  becomes overdue?`
- repair suggestion: add a requirement such as
  `the current time to be later than due_at`
- AIL-Flow highlight: Action Card requirements section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/overdue-without-time-requirement.ail-spec.md`

### AIL-APP-003

- condition: an Application action changes `Ticket.status` in a package with a
  public-update surface but does not record a public update
- affected graph item: `writes` edge from the action to `Ticket.status`
- message template: `action {name} changes Ticket.status without recording a
  public update`
- non-engineer explanation: customers can lose the visible history entry that
  explains a status change
- agent follow-up question: `What public update should be visible after this
  status change?`
- repair suggestion: add `the system records a public update`
- AIL-Flow highlight: Action Card write/effect section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/status-change-without-public-update.ail-spec.md`

### AIL-STATE-001

- condition: an Application action mutates `Counter.value` while claiming
  persistent or durable state, but has no persistence guarantee
- affected graph item: `writes` edge from the action to `Counter.value`
- message template: `action {name} mutates persistent counter state without a
  persistence guarantee`
- non-engineer explanation: replay could lose the counter update because the
  spec does not say when the new value becomes durable
- agent follow-up question: `What durable store, journal, snapshot, or replay
  boundary preserves the counter update?`
- repair suggestion: add a guarantee that the counter value is persisted before
  replay, or remove the persistent-state claim
- AIL-Flow highlight: Action Card guarantee section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/stateful_counter.ail/examples/rejected/increment-without-persistence-guarantee.ail-spec.md`

### AIL-STATE-002

- condition: an Application action is retryable and mutates `Counter.value`, but
  has no request id, idempotency key, dedupe rule, or processed-request state
- affected graph item: `writes` edge from the action to `Counter.value`
- message template: `action {name} is retryable but mutates counter state
  without an idempotency key`
- non-engineer explanation: a retry could increment the counter more than once
  for the same user request
- agent follow-up question: `Which request id or idempotency key identifies one
  logical increment?`
- repair suggestion: add a request id or idempotency key requirement and a
  processed-request write
- AIL-Flow highlight: Action Card requirements and writes sections
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/stateful_counter.ail/examples/rejected/retryable-increment-without-idempotency-key.ail-spec.md`

### AIL-STATE-003

- condition: an Application action mutates shared or concurrent
  `Counter.value`, but has no lock, serialization rule, or System lock guard
- affected graph item: `writes` edge from the action to `Counter.value`
- message template: `action {name} mutates shared counter state without a lock
  or serialization rule`
- non-engineer explanation: two actors could update the same counter at the
  same time and lose one increment
- agent follow-up question: `Which lock or serialization policy protects this
  shared counter?`
- repair suggestion: add a counter lock requirement, serialization guarantee,
  or System lock guard
- AIL-Flow highlight: Action Card guarantee section and System lock guard
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/stateful_counter.ail/examples/rejected/shared-counter-without-lock.ail-spec.md`

### AIL-STATE-004

- condition: an Application action can fail after writing `Counter.value`, but
  has no rollback, resume, or idempotent replay guarantee
- affected graph item: `writes` edge from the action to `Counter.value`
- message template: `action {name} can fail after a counter write without a
  replay recovery policy`
- non-engineer explanation: a failed operation could be retried without knowing
  whether the prior counter write already happened
- agent follow-up question: `Should replay roll back the write, resume from a
  request id, or dedupe the retry?`
- repair suggestion: add a rollback, resume, or idempotent replay guarantee
- AIL-Flow highlight: Action Card failures and guarantees sections
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/stateful_counter.ail/examples/rejected/failure-after-write-without-replay-policy.ail-spec.md`

### AIL-WORKFLOW-001

- condition: an Application action repeats another action and claims scheduler
  behavior, but has no temporal policy guarantee
- affected graph item: scheduler-behavior `Guarantee` node on the repeated
  action
- message template: `action {name} claims scheduler behavior without a
  temporal policy`
- non-engineer explanation: repeated execution can be compiled
  deterministically, but a schedule claim needs a named policy before reviewers
  can tell when or why the repeated work should run
- agent follow-up question: `Which temporal policy, window, cadence, or
  scheduler rule governs this repeated work?`
- repair suggestion: add a temporal policy guarantee to the repeated action or
  remove the scheduler behavior claim
- AIL-Flow highlight: Action Card guarantee section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/repeated_task.ail/examples/rejected/scheduler-without-temporal-policy.ail-spec.md`

### AIL-SECRET-READ-001

- condition: an action, tool, view, or compiler pass reads a secret without
  explicit protection
- affected graph item: `reads` edge and target `Secret` or secret `Field`
- message template: `secret read {target} requires explicit protection`
- non-engineer explanation: the program might expose private data without a
  visible rule
- agent follow-up question: `Who may read {target}, and how should it be redacted?`
- repair suggestion: add permission, redaction, and trace coverage
- AIL-Flow highlight: Permission View secret flow
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/secret-read-without-protection.ail-spec.md`

### AIL-SECRET-ROLE-001

- condition: an Application action reads secret `internal notes` without a
  support-role requirement
- affected graph item: `reads` edge from the action to `Ticket.internal notes`
- message template: `action {name} reads Ticket.internal notes without a
  support-role requirement`
- non-engineer explanation: a customer or non-support actor could reach secret
  internal notes even if the action still promises redaction
- agent follow-up question: `Which support roles may read these internal notes?`
- repair suggestion: add a requirement such as
  `the requester role to be SupportAgent or SupportManager`
- AIL-Flow highlight: Action Card requirements section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/secret_access.ail/examples/rejected/internal-notes-without-support-role.ail-spec.md`

### AIL-FFI-OWNERSHIP-001

- condition: a borrowed pointer is stored, returned, or used after the call
  boundary
- affected graph item: `ExternalBinding` pointer parameter
- message template: `borrowed pointer {name} cannot escape the C call boundary`
- non-engineer explanation: the C library could keep using memory after AIL no
  longer owns it
- agent follow-up question: `Should this pointer be owned by the C library, or should the call copy the data?`
- repair suggestion: change the pointer to owned with release semantics or
  remove the escape
- AIL-Flow highlight: C Interop block pointer ownership
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture: `docs/ail/21-c-interop-abi.md#accepted-fixtures`

## Checker Rule Coverage

Each checker rule contributes exactly one primary diagnostic code. Secondary
diagnostics may add context, but the primary code determines conformance
expectations. A checker rule without a diagnostic is not accepted into
AIL-Meta.

## Agent Repair Rules

The agent may propose repairs, but it must:

- quote the diagnostic code
- preserve the affected node and edge provenance
- ask a question when the repair changes semantics
- avoid inventing permissions, effects, failures, or external calls
- hand the repaired artifact back to the checker
