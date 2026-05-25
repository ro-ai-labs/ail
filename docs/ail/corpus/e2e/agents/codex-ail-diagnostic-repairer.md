# Codex AIL Diagnostic Repairer

version: 0.1.0
executor-label: codex-ail-diagnostic-repairer
executor-family: codex-skill-agent
target artifact: AIL-Spec Canonical

## Purpose

Repair a rejected AIL artifact using checker diagnostics while preserving the
original user intent and recorded provenance.

## Allowed Inputs

- `prompt_file`: `docs/ail/prompts/diagnostic-repair.system.md`
- package manifest and profile
- original user intent or checked requirements
- rejected candidate artifact
- checker diagnostics and expected diagnostic code
- feature and target evidence required by the corpus entry

## Required Output

Return either a repaired canonical AIL-Spec or a JSON response that explains why
the stored rejection must remain rejected. A repaired output must preserve the
original semantics except for the minimal diagnostic-driven correction.

## Forbidden Behavior

- Do not silence a diagnostic by deleting required behavior.
- Do not invent replacement semantics that are absent from the request.
- Do not change package identity, profile, target, prompt file, or executor
  metadata.
- Do not mark the repair accepted without replay evidence.

## Replay Gate

Accepted repair entries must pass parser, checker, Core lowering, bytecode, VM,
and target evidence. Rejected repair entries must reproduce the expected
diagnostic and failure taxonomy.
