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

The dry run lists every required prompt with its task-specific `probe-label`
and `probe-fingerprint`. Those probes cover concrete interactions such as
interview clarification, requirements drafting, canonical spec drafting,
AIL-Core lowering, repair, diagnostic repair, round-trip rendering, human
summary, flow patching, trace debugging, and C interop questions.

For live chat-completion requests, the harness appends an inline envelope
contract to each user probe and asks the endpoint for JSON mode with
`response_format: {"type":"json_object"}`. The envelope contract is part of the
fingerprinted probe text, so review can detect stale or generic prompt
requests. The default live budget is `--max-tokens 768`; lower budgets are
useful for failure probes, but may cut off verbose valid envelopes before
`checker_handoff` is emitted.

Run it only when `http://inteligentia-pro-1:8080/` is reachable and the output
will be reviewed:

```sh
python3 scripts/run_v03_prompt_llm_harness.py
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

The review writes:

```text
/tmp/ail-v03-prompt-llm/prompt-llm-harness-review.txt
/tmp/ail-v03-prompt-llm/prompt-llm-harness-review.fingerprint.txt
```

The interactive manual includes both commands when live checks are requested:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
```

Review mode checks request, response, content, report, manifest, fingerprint
artifacts, prompt-specific probe metadata, expected `artifact_kind` values, and
prompt-pack envelope shape for each required system prompt. It prints
`prompt-envelope-valid-count`, `prompt-envelope-questions-count`, and
`prompt-envelope-invalid-count`, then persists the accepted/rejected review
text as a fingerprinted harness review artifact. A non-empty raw model
response is still rejected when it is not a valid prompt-pack envelope or
blocking-question envelope, a generic artifact kind is rejected, and a generic
probe is rejected when its `probe-label` or `probe-fingerprint` does not match
the expected task-specific probes.
