---
name: ail-agent-policy-reviewer
description: Use when reviewing AIL AgentTool policy handoff evidence before proposing a policy trace amendment for accepted-corpus promotion.
---

# AIL Agent Policy Reviewer

## Purpose

Use this skill to review accepted AgentTool policy handoff evidence before any
policy trace amendment is proposed for promotion into `./examples`.

This skill implements the evidence contract in
`examples/agents/codex-ail-agent-policy-reviewer.md`. The model may review
AgentTool handoff evidence, but deterministic replay remains the authority.

## Required Inputs

- Current examples corpus under `examples/`.
- Current AgentTool policy artifacts produced by `ail-examples`.
- `examples-report.txt`.
- `manifest.ail-examples.txt`.
- Accepted AgentTool source entry id and reviewer notes.

## Review Sequence

Run the deterministic contract gate first:

```sh
cargo run -- ail-agent-contracts examples/agents
```

Run the AgentTool policy import manual chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks
```

Print the optional hosted reviewer probes before contacting the live endpoint:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --dry-run
```

When hosted AgentTool reviewer execution is claimed, review the recorded
artifacts offline:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts /tmp/ail-v03-agent-policy-live-review
```

The full manual live path is:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks --include-live
```

Replay examples directly when reviewing a different artifact directory:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-agent-policy-review --release-evidence
```

## Required Evidence

The review report must include:

- `agent-policy-review.txt`
- `agent-policy-review.fingerprint.txt`
- `agent-policy-review-fingerprint-observed-count`
- `agent-policy-capture-plan.json`
- `agent-policy-capture-plan.fingerprint.txt`
- `agent-policy-import-demo-report.txt`
- `agent-policy-import-demo-report.fingerprint.txt`
- `agent-policy-multi-agent-handoff-report.txt`
- `agent-policy-multi-agent-handoff-report.fingerprint.txt`
- `agent-policy-live-review-report.txt`
- `agent-policy-live-review-report.fingerprint.txt`
- `manifest.v03-agent-policy-live-review.txt`
- `models.json`
- `models.fingerprint.txt`
- `agent-policy-live-review-review.txt`
- `agent-policy-live-review-review.fingerprint.txt`
- `agent-policy-live-review-repair-backlog.txt`
- `agent-policy-live-review-repair-backlog.fingerprint.txt`
- `model-check present`
- `model-check-model-count`
- `model-check-model-id`
- no `model-check skipped`; skipped model checks are local fake-server evidence only
- `reviewer-envelope-valid-count`
- `reviewer-envelope-invalid-count`
- `evidence-bundle-present-count`
- `default-max-tokens`
- `max-tokens`
- `token-budget-default`
- `token-budget-warning`
- `reviewer-decision-accept-count`
- `reviewer-decision-needs-repair-count`
- `reviewer-decision-reject-count`
- `repair-source hosted-reviewer-nonaccept`
- `accepted-for-import`, `needs-repair`, or `rejected-for-import`
- `human-approval-required true`
- `agent-contract-check ail-agent-contracts examples/agents`
- `multi-agent-handoff-review required`
- `tool-permission-review required`
- `tool-approval-review required`
- `external-call-review required`
- `secret-redaction-review required`
- `audit-trace-review required`
- `source-preserved true`
- `proposed-accepted true`
- `policy-handoff-imported true`
- `policy-handoff-replayed true`
- `must_supply_request_response_json: true`
- `batch_capture_script: scripts/capture_example_batch.py`

## Rejection Rules

Return `needs-repair` or `rejected-for-import` when:

- `agent-policy-review.txt` is missing or its fingerprint file does not match
- deterministic replay does not list the policy review in
  `manifest.ail-examples.txt`
- `scripts/run_v03_agent_policy_import_demo.py` has not produced
  `agent-policy-import-demo-report.txt`
- the import demo does not report `source-preserved true`,
  `proposed-accepted true`, `policy-handoff-imported true`, and
  `policy-handoff-replayed true`
- agent contract checks, permission review, approval review, external-call
  review, secret-redaction review, or audit-trace review is missing
- hosted reviewer evidence is claimed but
  `scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts`
  is missing or reports `review-result rejected`
- hosted reviewer evidence is claimed but the offline review reports
  `model-check missing`, `model-check skipped`, a hosted reviewer response
  omits `model`, or a response model is not listed in `models.json`
- hosted reviewer evidence is claimed but recorded reviewer requests do not
  include `Evidence bundle status: complete`, an `evidence-bundle-fingerprint`,
  every required artifact fingerprint, and bounded content excerpts from the
  policy review, capture plan, import report, and multi-agent handoff report
- hosted reviewer evidence is claimed but one or more valid reviewer envelopes
  return `needs-repair` or `reject`; preserve the bundle as
  `review-result needs-repair` evidence and
  `agent-policy-live-review-repair-backlog.txt` instead of promotion evidence
- the artifact implies automatic promotion without human approval

Do not promote generated content into `./examples` unless deterministic replay,
AgentTool policy review, and human approval all pass. When human approval is
available, use `scripts/capture_example_batch.py` with
`agent_policy_capture_plan_json`, `source_entry_id`, and the proposed
`entry_id` so the policy-amended accepted entry is appended to a corpus copy
while the AgentTool source entry remains intact.
The deterministic wrapper is:

```sh
python3 scripts/run_v03_agent_policy_import_demo.py
```
