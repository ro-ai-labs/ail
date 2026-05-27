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
handoff-roles requirements-writer,spec-writer,diagnostic-repairer,prompt-reviewer,agent-policy-reviewer
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
entry-count
checker-result-count accepted
checker-result-count rejected
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

This is deterministic local evidence, not a live multi-agent execution claim.
It raises the AgentTool handoff from one import script to a reusable
role-separated review witness.

## Live Reviewer Evidence

Live reviewer execution stays opt-in because it contacts the hosted llama.cpp
server. Print the five reviewer probes without contacting the server first:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --dry-run
```

When `http://inteligentia-pro-1:8080/` is reachable, run the hosted reviewer
harness after the deterministic policy review, capture plan, import demo, and
multi-agent handoff commands above have written their artifacts:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py \
  --examples-artifacts /tmp/ail-manual-agent-policy \
  --capture-plan-dir /tmp/ail-manual-agent-policy-capture-plan \
  --import-work-dir /tmp/ail-manual-agent-policy-import-work
```

Those paths are the harness defaults. The live request sent to each reviewer
must include `Evidence bundle status: complete`, an
`evidence-bundle-fingerprint`, every required artifact fingerprint, and bounded
content excerpts from `agent-policy-review.txt`,
`agent-policy-capture-plan.json`, `agent-policy-import-demo-report.txt`, and
`agent-policy-multi-agent-handoff-report.txt`.

The default hosted reviewer budget is `--max-tokens 768`. The live report and
offline review repeat `default-max-tokens`, the actual `max-tokens`,
`token-budget-default`, and any `token-budget-warning`, so truncated or
over-budget reviewer evidence is visible before promotion decisions.
The live harness also records the endpoint model list in `models.json` and
`models.fingerprint.txt`; `--skip-model-check` must record an explicit skipped
model-check artifact instead of silently omitting the check. Offline review
rejects skipped model checks by default, so hosted AgentTool reviewer evidence
must show `model-check present`; use `--allow-skipped-model-check` only for
local fake-server tests.

Then review the recorded request, response, and content bundle offline:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts /tmp/ail-v03-agent-policy-live-review
```

The review must write and validate:

```text
agent-policy-live-review-report.txt
agent-policy-live-review-report.fingerprint.txt
manifest.v03-agent-policy-live-review.txt
models.json
models.fingerprint.txt
agent-policy-live-review-review.txt
agent-policy-live-review-review.fingerprint.txt
agent-policy-live-review-repair-backlog.txt
agent-policy-live-review-repair-backlog.fingerprint.txt
model-check
model-check-model-count
model-check-model-id
no model-check skipped
reviewer-envelope-valid-count
reviewer-envelope-invalid-count
evidence-bundle-present-count
default-max-tokens
max-tokens
token-budget-default
token-budget-warning
reviewer-decision-accept-count
reviewer-decision-needs-repair-count
reviewer-decision-reject-count
repair-backlog-fingerprint
```

The live reviewer report is accepted only when every reviewer envelope is
valid, `model-check present` names the hosted model list, each response `model`
is present in `models.json`, every recorded request contains the deterministic
evidence bundle,
and every role returns `decision: accept`. Valid `needs-repair` or `reject`
decisions produce `review-result needs-repair`, a nonzero exit, and
`agent-policy-live-review-repair-backlog.txt` with
`repair-source hosted-reviewer-nonaccept` so the hosted output becomes
fingerprinted repair backlog instead of promotion evidence. It still does not
edit `./examples`; promotion remains gated by deterministic replay, human
approval, and corpus-copy import evidence.
