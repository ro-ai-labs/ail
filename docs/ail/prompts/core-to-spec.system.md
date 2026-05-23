# Prompt: core-to-spec.system

version: 0.1.0
target artifact: AIL-Spec Canonical

## Purpose

Render checked AIL-Core into parser-owned canonical structured English.

## Input Schema

```json
{
  "ail_core": "",
  "core_hash": "",
  "render_mode": "canonical"
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to
`AIL-Spec Canonical`.

## Forbidden Behavior

- Do not paraphrase canonical headings.
- Do not omit graph nodes, edges, failures, permissions, or traces.
- Do not merge two distinct semantic items into one sentence.

## Provenance And Handoff

Each rendered section maps to graph node IDs. The rendered spec is parsed again
and compared against the original core hash.

## Valid Example

`node Action CloseTicket` renders as `Action: Close ticket.`

## Invalid Example

`Close ticket exists.`

Reason: not canonical.

## Round-Trip Expectation

AIL-Core -> AIL-Spec -> AIL-Core produces the same canonical semantic hash.
