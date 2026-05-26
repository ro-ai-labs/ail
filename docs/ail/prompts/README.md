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
`ail-prompt-corpus docs/ail/corpus/prompts` requires at least one accepted
stored output for each prompt file above, and `ail-examples examples` requires
the same prompt-file surface in the end-to-end examples set.

For opt-in hosted model probing, use the v0.3 prompt LLM harness:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --dry-run
```

The live form contacts `http://inteligentia-pro-1:8080/v1/models`, then probes
each prompt file through `http://inteligentia-pro-1:8080/v1/chat/completions`
with task-specific probes. Each request records `probe-label` and
`probe-fingerprint` metadata so review can distinguish useful prompt-surface
coverage from a generic smoke prompt. The harness writes request, response,
extracted content, report, manifest, and fingerprint artifacts under
`/tmp/ail-v03-prompt-llm`. The output is evidence for prompt interaction review
only; generated text still has to pass the deterministic checker before it can
be promoted into `./examples`.

Review a completed hosted run before promotion:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

The review mode is offline. It checks the required prompt set, manifest,
report, request/response/content files, prompt fingerprints, probe labels,
probe fingerprints, expected `artifact_kind` values, artifact fingerprints, and
prompt-pack envelope shape. A hosted probe is not accepted only because it is
non-empty: extracted content must classify as either `prompt-envelope-artifact`
or `prompt-envelope-questions`. The review prints
`prompt-envelope-valid-count`, `prompt-envelope-questions-count`, and
`prompt-envelope-invalid-count`, and rejects empty output, raw non-envelope
output, generic artifact kinds, or artifacts captured with the wrong
task-specific probe.
