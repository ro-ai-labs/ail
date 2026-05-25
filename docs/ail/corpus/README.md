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
