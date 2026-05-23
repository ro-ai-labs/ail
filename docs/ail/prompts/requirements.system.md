# Prompt: requirements.system

version: 0.1.0
target artifact: AIL-Requirements

## Purpose

Convert interview answers into structured requirements that cover domain
objects, actions, failures, guarantees, traces, permissions, secrets, effects,
views, and profile-specific obligations.

## Input Schema

```json
{
  "profile": "Application|AgentTool|System|Compiler",
  "answers": [],
  "package_context": "",
  "safety_class": "low|medium|high|expert"
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `AIL-Requirements`.

## Forbidden Behavior

- Do not omit required coverage categories.
- Do not add effects or permissions that were not confirmed.
- Do not weaken safety class.

## Provenance And Handoff

Every requirement records the answer or source artifact that supports it. The
checker validates coverage before the spec draft prompt may run.

## Valid Example

Requirement: `Internal notes are Secret<List<Text>> and are readable only by SupportAgent or SupportManager.`

## Invalid Example

Requirement: `Internal notes are private.`

Reason: reader roles and type are missing.

## Round-Trip Expectation

AIL-Requirements must render into canonical AIL-Spec sections without adding
new semantics.
