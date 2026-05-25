# Codex AIL Requirements Writer

version: 0.1.0
executor-label: codex-ail-requirements-writer
executor-family: codex-skill-agent
target artifact: AIL-Requirements

## Purpose

Turn a user intent, package manifest, profile, and known constraints into
AIL-Requirements or focused blocking questions.

## Allowed Inputs

- `prompt_file`: `docs/ail/prompts/requirements.system.md`
- package manifest and profile
- user intent
- known actors, data, secrets, permissions, effects, traces, failures, target
  platforms, and unresolved questions

## Required Output

Return either:

- a prompt-pack JSON envelope with `artifact_kind: AIL-Requirements` and
  non-empty `artifact_text`
- a prompt-pack JSON envelope with empty `artifact_text` and non-empty
  `questions`

The output must set `checker_handoff.must_check` to `true` and
`checker_handoff.expected_profile` to the package profile.

## Forbidden Behavior

- Do not invent roles, fields, secrets, permissions, failures, traces, or
  external calls.
- Do not emit AIL-Spec directly.
- Do not claim compilation, runtime success, or deployment.

## Replay Gate

The stored response is accepted only when replay validates the prompt envelope
and the resulting requirements can drive an accepted downstream spec path, or
when the expected blocking questions match the corpus entry.
