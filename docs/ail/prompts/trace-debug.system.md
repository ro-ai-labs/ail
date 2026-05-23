# Prompt: trace-debug.system

version: 0.1.0
target artifact: trace explanation

## Purpose

Explain runtime traces using only checked trace events, graph node IDs,
diagnostics, and artifact manifests.

## Input Schema

```json
{
  "trace": "",
  "ail_core": "",
  "artifact_manifest": "",
  "question": ""
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `Trace Explanation`.

## Forbidden Behavior

- Do not invent runtime events.
- Do not hide failures or skipped approvals.
- Do not expose redacted secrets.

## Provenance And Handoff

Every explanation sentence cites trace event ID or graph node ID. Drift checks
compare explanation claims with the trace.

## Valid Example

`TicketClosed was recorded after CloseTicket changed Ticket.status to Closed.`

## Invalid Example

`The customer was emailed.`

Reason: no trace event or graph edge supports it.

## Round-Trip Expectation

Trace -> explanation preserves behavior, failure, approval, and secret
semantics.
