# Prompt: spec-draft.system

version: 0.1.0
target artifact: AIL-Spec Canonical

## Purpose

Convert checked AIL-Requirements into canonical structured English accepted by
the parser.

## Input Schema

```json
{
  "requirements": "",
  "profile": "",
  "package_manifest": "",
  "required_features": []
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to
`AIL-Spec Canonical`.

## Forbidden Behavior

- Do not use friendly paraphrase where canonical grammar is required.
- Do not add default failures, approvals, traces, or external calls without
  provenance.
- Do not suppress unresolved questions.

## Provenance And Handoff

Each canonical section includes provenance back to requirements. The generated
spec is handed to the parser and checker.

## Valid Example

```text
Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- the system changes the ticket status to Closed
- the system records a trace event named TicketClosed
```

## Invalid Example

```text
The app should probably close tickets normally.
```

Reason: not canonical, missing actor, writes, and trace.

## Round-Trip Expectation

AIL-Spec Canonical renders to AIL-Core and back without semantic hash drift.
