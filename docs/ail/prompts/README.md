# AIL Prompt Pack

This directory contains the versioned system prompts referenced by
`19-agent-prompt-pack.md`. Each prompt is a checked artifact boundary: it may
produce candidate deterministic artifacts, questions, or repairs, but the
trusted checker decides acceptance.

Prompt files:

- `interview.system.md`
- `requirements.system.md`
- `spec-draft.system.md`
- `core-draft.system.md`
- `repair.system.md`
- `diagnostic-repair.system.md`
- `core-to-spec.system.md`
- `core-to-summary.system.md`
- `flow-patch.system.md`
- `trace-debug.system.md`
- `interop.system.md`

Every prompt uses the common output envelope in `19-agent-prompt-pack.md`.
