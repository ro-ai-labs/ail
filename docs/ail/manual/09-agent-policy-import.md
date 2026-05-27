# AIL Manual: Agent Policy Import

## Purpose

The Agent Policy import chapter turns deterministic AgentTool policy review
artifacts into a human-approved corpus-copy import. It keeps the source
AgentTool example unchanged, validates the multi-agent handoff review, writes
approved request/response transcripts, appends a new accepted entry to a
scratch corpus, and replays that corpus through `ail-examples`.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks
```

## Workflow

First replay the release corpus to materialize AgentTool policy evidence:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-manual-agent-policy --release-evidence
```

The replay must include:

```text
agent-policy-review.txt
agent-policy-review.fingerprint.txt
agent-policy-review-fingerprint-observed-count
entry-artifact example-40 agent-policy-review
```

Then build the plan-only capture artifact:

```sh
python3 scripts/run_v03_agent_policy_capture_plan.py \
  --examples-artifacts /tmp/ail-manual-agent-policy \
  --entry-id example-40 \
  --output-dir /tmp/ail-manual-agent-policy-capture-plan
```

The plan writes:

```text
agent-policy-capture-plan.json
agent-policy-capture-plan.txt
agent-policy-capture-plan.fingerprint.txt
agent-contract-check ail-agent-contracts examples/agents
human-approval-required true
preserve-source-entry true
```

Finally run the deterministic import demo:

```sh
python3 scripts/run_v03_agent_policy_import_demo.py \
  --base-corpus examples \
  --examples-artifacts /tmp/ail-manual-agent-policy \
  --capture-plan-dir /tmp/ail-manual-agent-policy-capture-plan \
  --source-entry-id example-40 \
  --work-dir /tmp/ail-manual-agent-policy-import-work \
  --output-corpus /tmp/ail-manual-agent-policy-import-corpus \
  --output-artifacts /tmp/ail-manual-agent-policy-import-artifacts
```

The demo report must include:

```text
agent-policy-import-demo-report.txt
agent-policy-import-demo-report.fingerprint.txt
source-preserved true
proposed-accepted true
agent-policy-review-fingerprint-preserved true
checked-core-fingerprint-preserved true
policy-handoff-imported true
policy-handoff-replayed true
entry-count 118
checker-result-count accepted 109
checker-result-count rejected 9
```

The import appends `PolicyHandoffApprovedScenario40` to the human-approved
AgentTool spec so the promoted entry is distinct, replayable, and still bound
to the deterministic policy review artifact.

## Multi-Agent Handoff Evidence

After the import demo passes, write the role-by-role handoff witness:

```sh
python3 scripts/run_v03_agent_policy_multi_agent_handoff.py \
  --examples-artifacts /tmp/ail-manual-agent-policy \
  --capture-plan-dir /tmp/ail-manual-agent-policy-capture-plan \
  --import-work-dir /tmp/ail-manual-agent-policy-import-work \
  --output-artifacts /tmp/ail-manual-agent-policy-import-artifacts \
  --source-entry-id example-40 \
  --output-dir /tmp/ail-manual-agent-policy-import-work
```

The script validates `ail-agent-contracts examples/agents`, the policy capture
plan, the policy review fingerprint, the import report fingerprint, and the
promoted checked Core trace. It then writes:

```text
agent-policy-multi-agent-handoff-report.txt
agent-policy-multi-agent-handoff-report.fingerprint.txt
separate-reviewer-role-count 5
role requirements-writer contract codex-ail-requirements-writer
role spec-writer contract codex-ail-spec-writer
role diagnostic-repairer contract codex-ail-diagnostic-repairer
role prompt-reviewer contract codex-ail-prompt-reviewer
role agent-policy-reviewer contract codex-ail-agent-policy-reviewer
multi-agent-execution-evidence deterministic-role-handoff
```

This is still deterministic local evidence, not a live multi-agent execution
claim. It raises the AgentTool handoff from one import script to a reusable
role-separated review witness before the next v0.3 step adds live reviewer
execution.
