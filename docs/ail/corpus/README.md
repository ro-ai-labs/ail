# AIL Corpus

The corpus is a versioned conformance and training asset. Every accepted
language feature contributes paired accepted and rejected examples, diagnostics,
runtime traces, prompt outputs, round-trip fixtures, interop fixtures,
self-hosting fixtures, and human-review artifacts before the feature is
accepted.

Fixture metadata:

- `language_reference`: `ail-reference.draft` unless overridden
- `core_schema`: `ail-core.schema.v0` unless overridden
- `prompt_pack`: draft prompt-pack with `AIL-PROMPT-001` when prompt output is
  present
- `bytecode`: stage-0 VM JSON plus native Linux x86_64 ELF target when
  executable
- `conformance_suite`: `first-slice` package fixtures and profile fixtures

Each fixture added after the reference-style guide must carry this metadata in
the fixture file, companion manifest, or inventory row.

Directory contract:

- `interviews/`: user intent and agent question transcripts
- `specs/accepted/`: canonical AIL-Spec accepted by the checker
- `specs/rejected/`: canonical or candidate specs rejected with diagnostics
- `core/accepted/`: accepted AIL-Core fixtures
- `core/rejected/`: rejected AIL-Core fixtures
- `flow/`: AIL-Flow views and graph patches
- `traces/`: runtime and debugging traces
- `prompts/`: prompt portability and repair fixtures
- `roundtrip/`: equivalence fixtures
- `interop/`: C/ABI and external binding fixtures
- `selfhost/`: AIL-Meta and fixed-point fixtures
- `e2e/`: prompt-to-artifact examples that replay model or agent outputs
  through checked requirements or AIL-Spec, checked AIL-Core, bytecode,
  execution or target-contract artifacts, manifests, and fingerprints

Corpus acceptance metrics:

- every fixture has expected status
- every rejected fixture has expected diagnostic codes
- every accepted executable fixture has trace coverage
- every prompt fixture either normalizes to expected AIL-Core or asks expected
  blocking questions
- every round trip records the before and after semantic hash

Prompt portability corpus entries are checked offline with:

```bash
cargo run -- ail-prompt-corpus docs/ail/corpus/prompts --artifact-dir /tmp/ail-prompt-corpus
```

The command reads `## Stored Output:` blocks, verifies accepted `ail-spec`
outputs by normalizing them to checked AIL-Core, verifies rejected outputs
against expected diagnostics or semantic drift, and writes:

- `prompt-corpus-portability.txt`
- `prompt-corpus-portability.fingerprint.txt`
- `manifest.ail-prompt-corpus.txt`
- `manifest.fingerprint.txt`
- `accepted/<entry>.ail-core.txt`
- `accepted/<entry>.ail-core.fingerprint.txt`

The portability report records semantic task labels, model labels, prompt
fingerprints, artifact fingerprints, checker results, and failure taxonomy.

End-to-end release corpus entries are checked offline with:

```bash
cargo run -- ail-e2e-corpus docs/ail/corpus/e2e --artifact-dir /tmp/ail-e2e-corpus
```

Seed replay uses the default mode above. Final v0.2 release evidence must use
the stricter release mode:

```bash
cargo run -- ail-e2e-corpus docs/ail/corpus/e2e --artifact-dir /tmp/ail-e2e-corpus --release-evidence
```

`--release-evidence` refuses deterministic seed entries and requires both
`live-llm` and `live-codex` capture origins. This keeps the checked seed corpus
useful for verifier development without letting it satisfy the live
prompt-to-artifact release gate.

To capture live LLM evidence without changing the offline replay contract, copy
the seed corpus and replace one entry with a stored HTTP completion transcript:

```bash
python3 scripts/capture_e2e_transcripts.py \
  --base-corpus docs/ail/corpus/e2e \
  --output-dir /tmp/ail-e2e-live-corpus \
  --entry-id example-30 \
  --endpoint http://inteligentia-pro-1:8080/completion \
  --endpoint-label inteligentia-pro-1-qwen3.6-35b \
  --executor-label unsloth-qwen3.6-35b-a3b-gguf \
  --semantic-task support-ticket-live-capture-30 \
  --prompt "Produce the Support Ticket AIL-Spec for live capture replay."
```

The captured corpus is then replayed with `ail-e2e-corpus`; replay must remain
offline and must read only the stored request/response transcripts.

The `ail-e2e-corpus` verifier is the v0.2 release gate for prompt reliability.
It must not call a live model endpoint in replay mode. It reads stored
transcripts produced by HTTP LLM executors, AIL toolchain agents, or
Codex-style skill agents, extracts the deterministic AIL artifact, validates
the prompt envelope, checks requirements or AIL-Spec, lowers to checked
AIL-Core, emits bytecode, verifies VM behavior, and writes either a Linux
native artifact or a target-contract report.

The v0.2 release corpus must contain at least 100 distinct end-to-end examples.
Each counted example records:

- semantic task id
- profile and package path
- prompt file, prompt version, and prompt fingerprint
- executor family and executor label
- capture origin: `deterministic-seed`, `live-llm`, or `live-codex`
- endpoint label for HTTP LLM evidence
- raw request and raw response fingerprints
- extracted artifact and checker result
- checked AIL-Core fingerprint
- bytecode fingerprint
- VM trace fingerprint when executable
- native binary or target-contract fingerprint
- manifest fingerprint
- failure taxonomy for rejected examples

The 100-example count is semantic, not cosmetic. Re-running the same stored
output under a different label does not count unless the executor transcript is
real, replayable, and the semantic task or target evidence changes.

The e2e report also records duplicate-fingerprint counts for stored requests,
responses, extracted artifacts, checked Core, bytecode, VM traces, native
artifacts, target reports, diagnostics, and capture-origin buckets. These
counts make seed-corpus reuse auditable. Final v0.2 release evidence must drive
duplicate response, extracted-artifact, and target-report counts to zero, and
must replace broad `deterministic-seed` coverage with `live-llm` and
`live-codex` transcript captures except for any explicitly documented shared
artifact that is not counted as semantic-release coverage.
