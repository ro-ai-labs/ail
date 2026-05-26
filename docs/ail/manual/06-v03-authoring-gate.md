# AIL Manual: v0.3 Authoring Gate

## Purpose

The v0.3 authoring gate chapter runs the deterministic audit that ties the
manual together. It proves the current story-first workflow, examples replay,
roadmap printing, prompt interaction checks, and agent entrypoint checks can be
executed from one command.

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
```

These are wrappers around the individual chapters:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter v03-roadmap --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
```

## Evidence

The gate should surface:

```text
story-mode-report.txt
manifest.ail-story.txt
story-questions.ail-interview.md
agent-trace.txt
examples-report.txt
v03-roadmap.txt
manifest.ail-examples.txt
prompt-corpus-portability.txt
manifest.ail-prompt-corpus.txt
agent.ailbc.json
```

Passing this chapter is not the same as declaring AIL v0.3 complete. It is the
current deterministic authoring audit used to decide which missing behavior
should be implemented next.
