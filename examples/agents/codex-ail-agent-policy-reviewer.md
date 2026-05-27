# Codex AIL Agent Policy Reviewer

version: 0.1.0
executor-label: codex-ail-agent-policy-reviewer
executor-family: codex-skill-agent
target artifact: AIL-Agent-Policy-Review
contract: examples/agents/codex-ail-agent-policy-reviewer.md

## Purpose

Create an AgentTool policy handoff review report for accepted AgentTool
examples before any reviewed policy handoff is proposed as a new accepted
corpus entry.

## Allowed Inputs

- examples artifact directory produced by
  `ail-examples examples --artifact-dir ... --release-evidence`
- `examples-report.txt`
- `manifest.ail-examples.txt`
- accepted AgentTool entry id
- `agent-policy-review.txt`
- `agent-policy-review.fingerprint.txt`
- `agent-policy-capture-plan.json`
- `agent-policy-capture-plan.fingerprint.txt`
- `agent-policy-import-demo-report.txt`
- `agent-policy-import-demo-report.fingerprint.txt`
- reviewer notes about intended AgentTool policy handoff promotion

## Required Output

Return an `AIL-Agent-Policy-Review` report that records:

- accepted AgentTool source entry id
- proposed accepted entry id
- policy import decision: `accepted-for-import`, `needs-repair`, or
  `rejected-for-import`
- `human-approval-required true`
- `agent-contract-check ail-agent-contracts examples/agents`
- `multi-agent-handoff-review required`
- `tool-permission-review required`
- `tool-approval-review required`
- `external-call-review required`
- `secret-redaction-review required`
- `audit-trace-review required`
- `policy-import-status proposed-only`
- `agent-policy-review-fingerprint-observed-count`
- `agent-policy-review.txt`
- `agent-policy-review.fingerprint.txt`
- `agent-policy-capture-plan.json`
- `agent-policy-capture-plan.fingerprint.txt`
- `agent-policy-import-demo-report.txt`
- `agent-policy-import-demo-report.fingerprint.txt`
- `source-preserved true`
- `proposed-accepted true`
- `policy-handoff-imported true`
- `policy-handoff-replayed true`

## Forbidden Behavior

- Do not promote generated content into ./examples without passing
  deterministic replay and human approval.
- Do not treat a policy review or capture plan as sufficient unless the import
  demo reports `source-preserved true`, `proposed-accepted true`,
  `policy-handoff-imported true`, and `policy-handoff-replayed true`.
- Do not accept an AgentTool policy handoff when permission review, approval
  review, external-call review, secret-redaction review, audit-trace review,
  or agent contract checks are missing.
- Do not rewrite the reviewed source entry during promotion; the source
  AgentTool entry remains part of the learning corpus.
- Do not treat `accepted-for-import` as an automatic corpus edit.

## Replay Gate

The review is accepted only when:

```sh
cargo run -- ail-agent-contracts examples/agents
cargo run -- ail-examples examples --artifact-dir /tmp/ail-agent-policy-review --release-evidence
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks
```

The resulting report must include `agent-policy-review.txt`,
`agent-policy-review.fingerprint.txt`, and
`agent-policy-review-fingerprint-observed-count`. The plan-only import bridge
must also be generated with:

```sh
python3 scripts/run_v03_agent_policy_capture_plan.py \
  --examples-artifacts /tmp/ail-agent-policy-review \
  --entry-id <agent-tool-entry-id> \
  --output-dir /tmp/ail-agent-policy-capture-plan
```

The capture plan must include `human_approval_required: true`,
`must_supply_request_response_json: true`, and
`batch_capture_script: scripts/capture_example_batch.py`.

After human approval, run the deterministic import demo:

```sh
python3 scripts/run_v03_agent_policy_import_demo.py \
  --base-corpus examples \
  --examples-artifacts /tmp/ail-agent-policy-review \
  --capture-plan-dir /tmp/ail-agent-policy-capture-plan \
  --source-entry-id <agent-tool-entry-id> \
  --work-dir /tmp/ail-agent-policy-import-work \
  --output-corpus /tmp/ail-agent-policy-import-corpus \
  --output-artifacts /tmp/ail-agent-policy-import-artifacts
```

The import report must include `agent-policy-import-demo-report.txt`,
`source-preserved true`, `proposed-accepted true`,
`policy-handoff-imported true`, and `policy-handoff-replayed true`. The
reviewer may then prepare a batch entry with `source_entry_id`, `entry_id`,
`request_json_file`, `response_json_file`, and
`agent_policy_capture_plan_json`. The batch importer must append the proposed
accepted entry in a corpus copy and must not rewrite or delete the AgentTool
source entry.
