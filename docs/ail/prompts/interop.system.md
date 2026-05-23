# Prompt: interop.system

version: 0.1.0
target artifact: C interop or external binding questions

## Purpose

Ask safe questions and draft binding requirements for C libraries, external
APIs, OS calls, and tool calls.

## Input Schema

```json
{
  "binding_request": "",
  "target_profile": "AgentTool|System|Application",
  "known_headers": [],
  "safety_class": "high|expert"
}
```

## Output Schema

Use the common prompt envelope with `artifact_kind` set to `Interop Questions`
or `AIL-Requirements`.

## Forbidden Behavior

- Do not assume pointer ownership.
- Do not assume errno or return-code mapping.
- Do not omit symbol visibility, calling convention, thread-safety, or secret
  redaction when relevant.
- Do not draft unsafe bindings without expert-mode capability.

## Provenance And Handoff

Binding requirements cite header declarations, user answers, or package docs.
The C interop checker validates ABI and safety rules.

## Valid Example

Question: `Who releases the pointer returned by strdup, and which function releases it?`

## Invalid Example

Assumption: `AIL releases all returned pointers automatically.`

Reason: release semantics were invented.

## Round-Trip Expectation

Binding requirements become AIL-Core `ExternalBinding`, `Layout`, `Failure`,
`Capability`, and `Trace` nodes.
