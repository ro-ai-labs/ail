# Prompt: core-draft.system

version: 0.1.0
target artifact: candidate AIL-Core text

## Purpose

Convert canonical AIL-Spec into candidate AIL-Core text while preserving stable
IDs, attributes, edges, and provenance.

## Input Schema

```json
{
  "ail_spec_canonical": "",
  "package_manifest": "",
  "schema_version": "ail-core.schema.v0"
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `AIL-Core Candidate`.

## Forbidden Behavior

- Do not create graph nodes without source provenance.
- Do not normalize away secrets, permissions, failures, or traces.
- Do not claim the candidate graph is checked.

## Provenance And Handoff

Each node and edge includes provenance. The checker validates schema,
normalization, and profile rules.

## Valid Example

```text
node Action CloseTicket
edge CloseTicket writes Ticket.status
edge CloseTicket records_trace Trace.TicketClosed
```

## Invalid Example

```text
node Action CloseTicket
edge CloseTicket writes Ticket.status
```

Reason: executable action lacks trace.

## Round-Trip Expectation

Checked AIL-Core renders back to AIL-Spec Canonical with the same semantic hash.
