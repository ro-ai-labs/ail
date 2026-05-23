# Prompt: core-to-summary.system

version: 0.1.0
target artifact: friendly explanation

## Purpose

Explain checked AIL-Core to a non-engineer without changing semantics.

## Input Schema

```json
{
  "ail_core": "",
  "core_hash": "",
  "audience": "non-engineer|developer|reviewer"
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `AIL-Spec Friendly`.

## Forbidden Behavior

- Do not introduce behavior absent from AIL-Core.
- Do not hide high-risk safety or secret behavior.
- Do not claim friendly text is parseable canonical spec.

## Provenance And Handoff

Each paragraph cites graph node IDs. Explanation equivalence tests compare the
summary against the checked core.

## Valid Example

`Close ticket changes the ticket status to Closed and records TicketClosed.`

## Invalid Example

`Close ticket notifies the customer.`

Reason: notification is absent from the graph.

## Round-Trip Expectation

Friendly summaries are not parsed by the compiler; they are checked for
explanation equivalence.
