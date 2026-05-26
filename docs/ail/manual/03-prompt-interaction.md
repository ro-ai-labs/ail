# AIL Manual: Prompt Interaction

## Purpose

The prompt interaction chapter validates the system prompts that agents and
hosted models use to draft, repair, explain, and translate AIL artifacts. It
combines the offline prompt corpus, examples replay, capture tooling, and the
opt-in hosted llama.cpp harness.

Run deterministic checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
```

## Offline Prompt Corpus

Replay the prompt portability corpus:

```sh
cargo run -- ail-prompt-corpus docs/ail/corpus/prompts --artifact-dir /tmp/ail-manual-prompt-corpus
```

The expected evidence is:

```text
/tmp/ail-manual-prompt-corpus/prompt-corpus-portability.txt
/tmp/ail-manual-prompt-corpus/manifest.ail-prompt-corpus.txt
```

## Examples Prompt Surfaces

Replay examples with release evidence to confirm that required prompt files
produce accepted artifacts, not only rejected diagnostics:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-manual-prompt-examples --release-evidence
```

Review `examples-report.txt` for `prompt-count` and `accepted-prompt-count`
lines.

## Hosted Harness

Print the hosted prompt-pack run without contacting the model:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --dry-run
```

Run it only when `http://inteligentia-pro-1:8080/` is reachable and the output
will be reviewed:

```sh
python3 scripts/run_v03_prompt_llm_harness.py
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

Review mode checks request, response, content, report, manifest, fingerprint
artifacts, and prompt-pack envelope shape for each required system prompt. It
prints `prompt-envelope-valid-count`, `prompt-envelope-questions-count`, and
`prompt-envelope-invalid-count`; a non-empty raw model response is still
rejected when it is not a valid prompt-pack envelope or blocking-question
envelope.
