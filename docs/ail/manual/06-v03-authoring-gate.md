# AIL Manual: v0.3 Authoring Gate

## Purpose

The v0.3 authoring gate chapter runs the deterministic audit that ties the
manual together. It proves the current story-first workflow, examples replay,
roadmap printing, prompt interaction checks, agent entrypoint checks, and
repair promotion checks can be executed from one command.

Run the gate:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks
```

## Checks

The gate runs these manual steps:

```text
run-user-story-mode-checks
run-examples-release-checks
run-v03-roadmap-checks
run-prompt-interaction-checks
run-agent-entrypoint-checks
run-repair-promotion-checks
```

These are wrappers around the individual chapters:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter v03-roadmap --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

Live hosted evidence remains opt-in. When the llama.cpp server is reachable,
include the live User Story mode review and live prompt interaction review in
the gate dry-run or execution:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --dry-run --include-live
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks --include-live
```

The live gate delegates to:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
```

## Evidence

The gate should surface:

```text
story-mode-report.txt
story-llm-harness-report.txt
story-llm-harness-report.fingerprint.txt
story-llm-transcript-check-count
story-prompt-envelope-valid-count
story-prompt-envelope-invalid-count
manifest.ail-story.txt
story-questions.ail-interview.md
agent-trace.txt
examples-report.txt
v03-roadmap.txt
manifest.ail-examples.txt
prompt-corpus-portability.txt
manifest.ail-prompt-corpus.txt
prompt-llm-harness-report.txt
prompt-llm-harness-review.txt
prompt-llm-harness-review.fingerprint.txt
manifest.v03-prompt-llm.txt
prompt-envelope-valid-count
prompt-envelope-artifact-required-count
prompt-envelope-questions-expected-count
prompt-outcome-match-count
prompt-envelope-invalid-count
agent.ailbc.json
repair-promotion-review.txt
repair-promotion-review.fingerprint.txt
repair-promotion-review-fingerprint-observed-count
repair-promotion-capture-plan.json
repair-promotion-capture-plan.txt
repair-promotion-capture-plan.fingerprint.txt
```

Passing this chapter is not the same as declaring AIL v0.3 complete. It is the
current deterministic authoring audit used to decide which missing behavior
should be implemented next.
