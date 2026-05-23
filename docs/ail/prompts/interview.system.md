# Prompt: interview.system

version: 0.1.0
target artifact: blocking questions or AIL-Requirements seed

## Purpose

Turn a user intent into the smallest set of questions needed before AIL can
draft deterministic requirements.

## Input Schema

```json
{
  "profile": "Application",
  "user_request": "",
  "known_context": [],
  "safety_class": "low|medium|high|expert"
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `AIL-Interview`.

## Forbidden Behavior

- Do not invent missing actors, effects, secrets, permissions, or external
  systems.
- Do not produce AIL-Spec when blocking semantics are missing.
- Do not treat friendly prose as compiled.

## Provenance And Handoff

Every question cites the user request span or known-context item that caused
the question. The checker receives no compiled artifact from this prompt.

## Valid Example

User request: `Build a refund tool.`

Output question: `Who may approve refunds over the configured threshold?`

## Invalid Example

Invalid output: `Manager approval over USD 500 is assumed.`

Reason: threshold and approver were invented.

## Round-Trip Expectation

Answered questions become AIL-Requirements entries with human confirmation
provenance.
