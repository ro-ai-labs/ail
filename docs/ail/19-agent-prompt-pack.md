# AIL Agent Prompt Pack

## Purpose

The prompt pack is a versioned set of system prompts and tool prompts that teach
an AI Agent to interview, draft, repair, render, explain, debug, and propose
patches for AIL artifacts without crossing the compiler trust boundary.

The prompt pack is an artifact, not a hidden implementation detail. Prompt
files live in `prompts/` and are referenced by package metadata and
conformance tests.

## Prompt Assets

Required prompt files:

- `prompts/interview.system.md`
- `prompts/requirements.system.md`
- `prompts/spec-draft.system.md`
- `prompts/core-draft.system.md`
- `prompts/repair.system.md`
- `prompts/diagnostic-repair.system.md`
- `prompts/core-to-spec.system.md`
- `prompts/core-to-summary.system.md`
- `prompts/flow-patch.system.md`
- `prompts/trace-debug.system.md`
- `prompts/interop.system.md`

Each prompt contains:

- purpose
- allowed input schema
- required output schema
- forbidden behavior
- provenance rules
- checker handoff rules
- one valid example
- one invalid example
- round-trip expectation

## Prompt Versioning

Prompt metadata:

```text
prompt: spec-draft.system
version: 0.1.0
schema: ail-prompt.v0
target_artifact: AIL-Spec Canonical
requires_checker: true
```

Prompt versions are immutable after release. A package declares the prompt pack
version used to create or repair an artifact. The checker does not trust the
prompt version; it only records the version as provenance and validates the
resulting deterministic artifact.

## Common Output Envelope

Agent outputs use this envelope:

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

If information is missing, the agent returns questions instead of inventing
semantics.

Normative rule `ail.prompt.envelope.handoff-matches-request`: when an output
uses the prompt-pack envelope, `artifact_kind` must match the requested
artifact, `checker_handoff.must_check` must be `true`, and
`checker_handoff.expected_profile` must match the package profile. An envelope
must contain either non-empty `artifact_text` or non-empty blocking
`questions`, not both. Violations are rejected with `AIL-PROMPT-001` before
artifact parsing, repair, checking, lowering, or compilation.

## Prompt Portability Harness

Prompt portability tests run the same task through at least two model
providers or model versions. Acceptance depends on normalized semantic output,
not matching wording.

Harness inputs:

- prompt pack version
- user request
- package profile
- expected unresolved questions
- accepted AIL-Core hash or accepted diagnostic codes

Harness outputs:

- raw model response
- extracted deterministic artifact
- normalized AIL-Core hash
- diagnostic list
- failure taxonomy
- portable prompt compatibility score

Compatibility score:

```text
score = accepted_semantic_outputs / total_model_outputs
```

A model output is accepted when it either normalizes to the expected AIL-Core or
asks the expected blocking questions without adding unsupported semantics.

## Failure Taxonomy

Prompt failures are classified as:

- `missing_question`: model guessed instead of asking
- `hallucinated_field`: model invented a field or effect
- `secret_disclosure`: model exposed a secret
- `hidden_external_call`: model added an undeclared external call
- `projection_drift`: rendered artifact changed graph meaning
- `trace_mismatch`: explanation contradicts runtime trace
- `schema_violation`: output envelope is invalid
- `checker_rejected`: deterministic artifact failed the checker

## Checker Handoff

Every prompt ends with the same rule: the AI Agent proposes artifacts, but the
checker decides acceptance. A prompt may never instruct the agent to treat its
own answer as compiled, trusted, deployed, or equivalent without checker
evidence.

## Conformance Fixtures

Every prompt has fixtures under `corpus/prompts/`:

- accepted output that passes the checker
- rejected output with expected diagnostic
- repair transcript that preserves provenance
- portability transcript across at least two model outputs

## Minimal Prompt Example

Input:

```json
{
  "profile": "Application",
  "user_request": "Build a support ticket app with private internal notes.",
  "known_requirements": []
}
```

Accepted response:

```json
{
  "artifact_kind": "AIL-Requirements",
  "artifact_text": "",
  "questions": [
    "Which user roles may read internal notes?",
    "Which actions must record trace events?"
  ],
  "assumptions": [],
  "provenance": ["user-request:0"],
  "checker_handoff": {
    "must_check": true,
    "expected_profile": "Application",
    "expected_features": ["things", "actions", "secrets", "traces"]
  }
}
```
