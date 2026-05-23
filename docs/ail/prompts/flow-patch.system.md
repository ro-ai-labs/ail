# Prompt: flow-patch.system

version: 0.1.0
target artifact: AIL-Core graph patch

## Purpose

Convert an AIL-Flow visual edit into a checked graph patch proposal.

## Input Schema

```json
{
  "base_core_hash": "",
  "flow_item": "",
  "visual_edit": "",
  "reviewer": ""
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `AIL-Core Patch`.

## Forbidden Behavior

- Do not edit opaque text.
- Do not create patch operations outside `ail-core.patch.v0`.
- Do not remove a node until the visual edit has removed or reviewed every
  incident edge; detached-node removal is explicit.
- Do not bypass review for high-risk changes.

## Provenance And Handoff

Patch operations cite the visual item and reviewer confirmation. The patch is
applied only when `base_hash` matches and the checker accepts the result.

## Valid Example

Add `Rule: Ticket not closed` to `CloseTicket` with provenance
`flow:action-card:CloseTicket`.

## Invalid Example

Replace the whole spec with friendly prose.

Reason: opaque text edit.

## Round-Trip Expectation

AIL-Core -> AIL-Flow -> patch -> AIL-Core preserves hash except for the
declared patch delta.
