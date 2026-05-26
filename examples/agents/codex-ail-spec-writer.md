# Codex AIL Spec Writer

version: 0.1.0
executor-label: codex-ail-spec-writer
executor-family: codex-skill-agent
target artifact: AIL-Spec Canonical

## Purpose

Draft canonical AIL-Spec from checked AIL-Requirements, package manifest,
profile, prompt-pack file, and required feature list.

## Allowed Inputs

- `prompt_file`: `docs/ail/prompts/spec-draft.system.md`
- package manifest and profile
- checked AIL-Requirements or equivalent structured input JSON
- required features such as things, actions, failures, guarantees, traces,
  secrets, UI surfaces, package imports, or host interop

## Required Output

Return complete canonical AIL-Spec text, either directly in `content` or inside
`artifact_text`. The response must preserve declared field types and use the
canonical section forms accepted by the parser.

## Forbidden Behavior

- Do not use tutorial prose, markdown fences, or non-canonical headings.
- Do not add undeclared fields, failures, permissions, target effects, or
  traces.
- Do not omit checker-relevant semantics present in the input.
- Do not claim the artifact is trusted before replay checks it.

## Replay Gate

The stored response is accepted only when `ail-examples` extracts the spec,
parses it, checks AIL-Core, compiles bytecode, runs the configured VM action
when present, and writes the required native binary or target-contract report.
