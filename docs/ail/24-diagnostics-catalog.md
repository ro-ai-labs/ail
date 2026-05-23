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
  "message": "action {action} is missing trace coverage",
  "non_engineer_explanation": "This action can run, but reviewers would not be able to see that it happened.",
  "agent_follow_up": "Which trace event should be recorded when this action runs?",
  "repair": "Add a trace event and attach it with records_trace.",
  "flow_highlight": "ActionCard.trace-section",
  "invalid_fixture": "examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md"
}
```

## Catalog

| Code | Condition | Severity | Blocking | Repair |
| --- | --- | --- | --- | --- |
| `AIL-TYPE-001` | unknown type in field, value, input, or output | error | yes | declare or import the type |
| `AIL-TRACE-001` | executable action or tool lacks trace coverage | error | yes | add `records_trace` |
| `AIL-TRACE-002` | failure lacks trace coverage | error | yes | add failure trace |
| `AIL-FAILURE-001` | declared blocking failure has no handler | error | yes | add handler or classify as propagated |
| `AIL-SECRET-READ-001` | secret read lacks explicit protection | error | yes | add permission and secret protection |
| `AIL-SECRET-WRITE-001` | secret write lacks redaction or protection | error | yes | add redaction policy |
| `AIL-SECRET-OUTPUT-001` | tool output exposes a secret without reveal permission | error | yes | remove secret output or add reveal approval |
| `AIL-PERMISSION-001` | permission reference has no rule or scope | error | yes | attach rule and scope |
| `AIL-APPROVAL-001` | approval has no triggering rule | error | yes | attach rule and trace |
| `AIL-CONTROL-001` | branch has no matching outcome or else | error | yes | add exhaustive outcome |
| `AIL-CONTROL-002` | match over finite variants is non-exhaustive | error | yes | cover every variant |
| `AIL-CONTROL-003` | termination-required profile has unproven recursion or loop | error | yes | add proof, bound, or profile policy |
| `AIL-FFI-OWNERSHIP-001` | borrowed C pointer escapes call boundary | error | yes | use owned pointer or remove escape |
| `AIL-FFI-ERRNO-001` | C error code or errno is unmapped | error | yes | map to declared failure |
| `AIL-SYSTEM-CAP-001` | system effect lacks matching capability | error | yes | add capability or remove effect |
| `AIL-SYSTEM-REGION-001` | resource effect lacks region placement | error | yes | declare resource region |
| `AIL-BACKEND-001` | target does not support requested effect | error | yes | choose target support or remove effect |
| `AIL-PROMPT-001` | agent output violates prompt envelope schema | error | yes | regenerate with required schema |
| `AIL-ROUNDTRIP-001` | projection round trip changes graph hash | error | yes | repair projection or patch |
| `AIL-UI-A11Y-001` | reachable UI action lacks accessible name | error | yes | add accessible label |

## Detailed Entries

### AIL-TRACE-001

- condition: executable action or tool has no `records_trace` edge
- affected graph item: `Action` or `Tool`
- message template: `action {name} is missing trace coverage`
- non-engineer explanation: reviewers cannot tell that the action ran
- agent follow-up question: `What event should the system record when {name} runs?`
- repair suggestion: add a named trace event and attach it to the action
- AIL-Flow highlight: Action Card trace section
- severity: error
- blocking behavior: blocks acceptance
- invalid fixture:
  `examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md`

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
