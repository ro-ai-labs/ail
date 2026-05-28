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

Write the plan-only system prompt harness inventory before running live checks:

```sh
python3 scripts/run_v03_system_prompt_harness_plan.py --artifact-dir /tmp/ail-manual-system-prompt-harness-plan
```

The plan writes `system-prompt-harness-plan.txt`,
`system-prompt-harness-plan.json`,
`system-prompt-harness-plan.fingerprint.txt`, and
`manifest.v03-system-prompt-harness-plan.txt`. It records the full
`prompt-count 11` inventory, one fingerprint per `docs/ail/prompts/*.system.md`
prompt, the hosted prompt and review commands, the interactive manual live
commands, the reviewer handoff to
`examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md`, and
`promotion-policy do-not-promote-generated-content`.

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
and `probe-fingerprint`, plus the `expected-content-kind` that review will
enforce. Those probes cover concrete interactions such as interview
clarification, requirements drafting, canonical spec drafting, AIL-Core
lowering, repair, diagnostic repair, round-trip rendering, human summary, flow
patching, trace debugging, and C interop questions.

For live chat-completion requests, the harness appends an inline envelope
contract to each user probe and asks the endpoint for JSON mode with
`response_format: {"type":"json_object"}`. The envelope contract is part of the
fingerprinted probe text, so review can detect stale or generic prompt
requests. The default live budget is `--max-tokens 768`; lower budgets are
useful for failure probes, but may cut off verbose valid envelopes before
`checker_handoff` is emitted. The run report records `default-max-tokens`, the
actual `max-tokens`, `token-budget-default`, and a `token-budget-warning` line
for non-default budgets so live evidence can explain whether a failure may be
budget-induced.

The Rust authoring entrypoints use the same chat shape for
`/v1/chat/completions`: the prompt-pack asset is sent as a `system message`,
the story or command request is sent as the `user` message, `stream` is
`false`, thinking is disabled, and JSON mode is requested with
`response_format: {"type":"json_object"}`. Root `/completion` endpoints remain
supported through the legacy single prompt body for local servers that do not
implement chat completions.

The harness writes and fingerprints `models.json` for the `/v1/models`
response before probing prompts. When `--skip-model-check` is used for a local
fake endpoint, `models.json` records the skipped check and endpoint so review
still proves the bypass was explicit. Review mode parses that model list,
prints `model-check-model-id`, and rejects any hosted response whose `model`
field is missing or not present in `models.json`.

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
artifacts, model-check evidence, response model identity, prompt-specific probe
metadata, expected `artifact_kind` values, and prompt-pack envelope shape for
each required system prompt. It also enforces the expected outcome for each
prompt: eight probes must produce `prompt-envelope-artifact`, while the
interview, diagnostic-repair, and interop probes must produce
`prompt-envelope-questions`. It prints
`prompt-envelope-valid-count`, `prompt-envelope-artifact-count`,
`prompt-envelope-questions-count`,
`prompt-envelope-artifact-required-count`,
`prompt-envelope-questions-expected-count`, `prompt-outcome-match-count`, and
`prompt-envelope-invalid-count`, plus `model-check`, `model-check-model-count`,
and `model-check-model-id`. It also repeats `default-max-tokens`, `max-tokens`,
`token-budget-default`, and any `token-budget-warning`, then
persists the accepted/rejected review text as a fingerprinted harness review
artifact. A non-empty raw model
response is still rejected when it is not a valid prompt-pack envelope or when
an artifact-required prompt returns only blocking questions. A generic artifact
kind is rejected, a response model outside the recorded model list is rejected,
and a generic probe is rejected when its `probe-label` or `probe-fingerprint`
does not match the expected task-specific probes.
