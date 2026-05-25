# AIL-Agent Protocol

## Purpose

The AIL-Agent protocol defines how an AI Agent participates in authoring,
reviewing, patching, explaining, and debugging AIL programs.

The AI Agent is part of the toolchain but not part of the trusted compiler core.

## Agent Responsibilities

The AI Agent should:

- interview the human
- identify missing semantic information
- ask focused clarification questions
- produce AIL-Spec
- elaborate AIL-Spec into AIL-Core
- render AIL-Core back into AIL-Spec and plain English
- propose patches instead of silent rewrites
- explain diagnostics
- debug traces interactively
- generate examples and tests
- preserve provenance for every inferred fact
- run prompt-portability checks through the conformance harness

## Interview Loop

The interview loop starts from an English request. The agent extracts candidate
actors, things, actions, rules, failures, secrets, external systems, views, and
guarantees. It asks one focused question at a time when a missing detail affects
checking, safety, or user-visible behavior.

## Required Coverage

Before a behavior is accepted, the agent should attempt to capture:

- who uses it
- what data it stores
- what actions are possible
- what inputs each action needs
- what outputs each action produces
- what must be true before actions run
- what can fail
- what happens on each failure
- what data is secret
- what external systems can be called
- what data may be read or changed
- what approvals are required
- what guarantees must always hold
- what views or interfaces humans need
- what traces should be explainable
- which prompt-pack version and model output produced each draft artifact

## Patch Discipline

Patch Discipline requires future changes to be represented as explicit patches
with expected effects. An agent must not silently rewrite accepted behavior.

Each proposed patch includes:

- human request
- affected nodes and edges
- structured English explanation
- expected behavior change
- expected permission, effect, failure, and guarantee changes
- provenance
- validation result

## Conversion Tasks

The agent may convert:

- English request to draft AIL-Spec
- AIL-Spec to candidate AIL-Core
- diagnostics to explanation questions
- traces to debugging explanations
- no-code edits to graph patches
- examples to training corpus entries

Each conversion is untrusted until checked.

## Prompt Compatibility Standard

Prompt Compatibility means a capable LLM should be able to use a language
feature from a compact prompt, schema, and examples.

Every feature should have:

- one short English rule
- one canonical structured form
- one valid example
- one invalid example
- one diagnostic example
- one round-trip expectation
- one no-code rendering expectation
- one trace/debugging expectation

The complete versioned prompt pack is defined in
`19-agent-prompt-pack.md` and implemented as files under `prompts/`.

## Prompt Output Schemas

Every prompt output uses a structured envelope:

```json
{
  "artifact_kind": "AIL-Spec Canonical",
  "artifact_text": "",
  "questions": [],
  "assumptions": [],
  "provenance": [],
  "checker_handoff": {
    "must_check": true,
    "expected_profile": "Application",
    "expected_features": []
  }
}
```

Outputs that violate the envelope are rejected with `AIL-PROMPT-001`.
For envelope outputs, `artifact_kind` must match the prompt stage,
`checker_handoff.must_check` must be `true`, and
`checker_handoff.expected_profile` must match the package profile. The agent
returns either `artifact_text` or blocking `questions`, never both.

## Prompt Versioning

Prompt versions are immutable release artifacts. A package records the prompt
pack version used by the agent, but the checker treats prompt identity as
provenance only. The deterministic artifact must still parse, normalize, and
pass schema validation.

## Unresolved Ambiguity

When the agent cannot resolve a semantic question, it must return a blocking
question instead of guessing. This applies especially to permissions, effects,
secrets, money, safety, C interop, OS calls, failure handling, traces, and
approval rules.

## Model Portability Tests

The prompt portability harness runs the same user request through multiple
model outputs and accepts semantic equivalence, not identical wording. A model
output passes when it normalizes to equivalent checked AIL-Core or asks the
expected blocking questions without inventing semantics.

The offline corpus verifier is:

```bash
cargo run -- ail-prompt-corpus docs/ail/corpus/prompts --artifact-dir /tmp/ail-prompt-corpus
```

It consumes stored model outputs rather than live endpoints. Accepted `ail-spec`
entries must normalize to checked AIL-Core and rejected entries must produce the
stored prompt-envelope, profile mismatch, hallucinated capability,
missing-trace, or semantic-drift failure taxonomy. The verifier writes a
fingerprinted portability report and manifest so prompt-pack regressions can be
reviewed without trusting current model availability.

## Calibration Examples

Calibration examples must include accepted specs, rejected specs, diagnostic
repairs, patches, traces, and explanations. They train the agent to ask for
missing semantics instead of guessing.

## Trust Boundary

The agent is untrusted. It can request tools, draft specs, propose patches, and
explain results, but the trusted checker accepts or rejects deterministic
artifacts.

## Failure Modes

The protocol must detect and surface:

- hallucinated actions or fields
- unconfirmed permission changes
- secret disclosure
- hidden external calls
- incomplete failure handling
- projection drift
- trace explanations that do not match runtime events
- prompt output schemas that cannot be validated
- model outputs that are wording-compatible but semantically different
