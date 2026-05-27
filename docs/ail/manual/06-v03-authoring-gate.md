# AIL Manual: v0.3 Authoring Gate

## Purpose

The v0.3 authoring gate chapter runs the deterministic audit that ties the
manual together. It proves the current story-first workflow, examples replay,
roadmap printing, prompt interaction checks, agent entrypoint checks, and
repair promotion, UI patch import, and AgentTool policy import checks can be
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
run-repair-promotion-checks
run-ui-patch-import-checks
run-agent-policy-import-checks
```

These are wrappers around the individual chapters:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter v03-roadmap --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks
```

Live hosted evidence remains opt-in. When the llama.cpp server is reachable,
include the live User Story mode review, live prompt interaction review, and
live AgentTool policy reviewer evidence in the gate dry-run or execution:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --dry-run --include-live
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks --include-live
```

The live gate delegates to:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks --include-live
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
story-promotion-capture-plan.json
story-promotion-capture-plan.txt
story-promotion-capture-plan.fingerprint.txt
story-promotion-import-demo-report.txt
story-promotion-import-demo-report.fingerprint.txt
manifest.ail-story.txt
story-questions.ail-interview.md
agent-trace.txt
examples-report.txt
v03-roadmap.txt
manifest.ail-examples.txt
ui-review.txt
ui-review-fingerprint-observed-count
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
ui-patch-capture-plan.json
ui-patch-capture-plan.txt
ui-patch-capture-plan.fingerprint.txt
ui-patch-import-demo-report.txt
ui-patch-import-demo-report.fingerprint.txt
ui-patch-runtime-state-check-report.txt
ui-patch-runtime-state-check-report.fingerprint.txt
visual-regression-fingerprint-preserved true
runtime-ui-state-check target-report
runtime-ui-state-anchor Ticket.reviewStatus
flow-edit-applied true
patched-core-replayed true
agent-policy-capture-plan.json
agent-policy-capture-plan.txt
agent-policy-capture-plan.fingerprint.txt
agent-policy-import-demo-report.txt
agent-policy-import-demo-report.fingerprint.txt
agent-policy-multi-agent-handoff-report.txt
agent-policy-multi-agent-handoff-report.fingerprint.txt
agent-policy-live-review-report.txt
agent-policy-live-review-report.fingerprint.txt
manifest.v03-agent-policy-live-review.txt
agent-policy-live-review-review.txt
agent-policy-live-review-review.fingerprint.txt
reviewer-envelope-valid-count
reviewer-envelope-invalid-count
reviewer-decision-accept-count
reviewer-decision-needs-repair-count
reviewer-decision-reject-count
policy-handoff-imported true
policy-handoff-replayed true
multi-agent-execution-evidence deterministic-role-handoff
```

Passing this chapter is not the same as declaring AIL v0.3 complete. It is the
current deterministic authoring audit used to decide which missing behavior
should be implemented next.
