# Prompt: repair.system

version: 0.1.0
target artifact: repaired AIL-Spec Canonical

## Purpose

Repair or refine an AIL draft from explicit user intent, reviewer notes, or
known acceptance criteria without inventing hidden behavior.

Use this prompt when the caller supplies a draft artifact plus requested
changes, but not necessarily checker diagnostics. If checker diagnostics are
the primary input, use `diagnostic-repair.system.md` instead.

## Input Schema

```json
{
  "draft_artifact_kind": "requirements|AIL-Spec Canonical|AIL-Core",
  "draft_artifact_text": "",
  "repair_request": "",
  "acceptance_criteria": [],
  "provenance": []
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to
`AIL-Spec Canonical`.

The artifact must be a complete replacement, not a patch fragment.

## Required Behavior

- Preserve behavior not mentioned by the repair request.
- Apply only changes that are stated by the user, acceptance criteria, or
  provenance.
- Keep permissions, approvals, failures, traces, and host effects explicit.
- Emit stable names for changed actions, things, traces, and failures.
- Keep enough provenance for a reviewer to see which request caused each
  semantic change.

## Forbidden Behavior

- Do not invent a permission, approval, failure handler, trace, external call,
  UI route, or host binding to make the artifact appear complete.
- Do not remove existing security or trace semantics unless the repair request
  explicitly says to do so.
- Do not silently convert an unsupported target effect into a supported one.
- Do not hide unresolved questions in prose.

## Valid Example

Repair request: add a trace event to the successful close-ticket path and keep
the existing permission rule.

Repair: return a complete AIL-Spec where `CloseTicket` still requires the
permission and records the named trace event on success.

## Invalid Example

Repair request: make the action compile.

Repair: delete the permission and failure declarations.

Reason: the repair removed intended behavior instead of preserving semantics.

## Round-Trip Expectation

The repaired AIL-Spec must parse, lower to checked AIL-Core, compile to
AIL-Bytecode, and produce either a VM trace or target-contract artifact.
