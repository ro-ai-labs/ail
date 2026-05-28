---
name: ail-story-promotion-reviewer
description: Use when reviewing AIL User Story mode promotion evidence before proposing a story-derived artifact for accepted-corpus promotion.
---

# AIL Story Promotion Reviewer

## Purpose

Use this skill to review User Story mode promotion evidence before any
story-derived generated artifact is proposed for promotion into `./examples`.

This skill implements the evidence contract in
`examples/agents/codex-ail-story-promotion-reviewer.md`. The model may review
promotion evidence, but deterministic replay remains the authority.

## Required Inputs

- Story harness artifacts under `/tmp/ail-v03-story-llm`.
- `story-llm-harness-report.txt`.
- `manifest.v03-story-llm.txt`.
- Story promotion capture plan artifacts.
- Optional hosted Story Promotion live reviewer artifacts.
- Human-approved request/response JSON for the proposed promoted story entry.
- Current examples replay and v0.3 roadmap artifacts.

## Review Sequence

Run the deterministic contract gate first:

```sh
cargo run -- ail-agent-contracts examples/agents
```

Review User Story mode artifacts:

```sh
python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm
```

Create the plan-only story promotion handoff:

```sh
python3 scripts/run_v03_story_promotion_capture_plan.py --story-artifacts /tmp/ail-v03-story-llm --output-dir /tmp/ail-v03-story-promotion-capture-plan
```

Run the deterministic story promotion import demo after human approval:

```sh
python3 scripts/run_v03_story_promotion_import_demo.py --story-artifacts /tmp/ail-v03-story-llm --capture-plan-dir /tmp/ail-v03-story-promotion-capture-plan
```

Review hosted Story Promotion reviewer evidence when it is claimed:

```sh
python3 scripts/run_v03_story_promotion_live_reviewer_harness.py --dry-run
python3 scripts/run_v03_story_promotion_live_reviewer_harness.py --review-artifacts /tmp/ail-v03-story-promotion-live-review
```

Replay examples and write the v0.3 roadmap evidence before promotion:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-story-promotion-review --release-evidence
python3 scripts/run_v03_story_promotion_batch_plan.py --base-corpus examples --examples-artifacts /tmp/ail-story-promotion-review --artifact-dir /tmp/ail-story-promotion-batch-plan
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-story-promotion-roadmap --release-evidence
```

## Required Evidence

The review report must include:

- `story-llm-harness-report.txt`
- `manifest.v03-story-llm.txt`
- `agent-trace`
- `semantic-anchor-missing-count 0`
- `story-promotion-capture-plan.json`
- `story-promotion-capture-plan.txt`
- `story-promotion-capture-plan.fingerprint.txt`
- `story-promotion-import-demo-report.txt`
- `story-promotion-import-demo-report.fingerprint.txt`
- `story-promotion-batch-plan.txt`
- `story-promotion-batch-plan.json`
- `story-promotion-batch-plan.fingerprint.txt`
- `manifest.v03-story-promotion-batch-plan.txt`
- `story-promotion-live-review-report.txt`
- `story-promotion-live-review-review.txt`
- `manifest.v03-story-promotion-live-review.txt`
- `models.json`
- `models.fingerprint.txt`
- `model-check-model-id`
- `AIL-Story-Promotion-Live-Reviewer-Decision`
- `reviewer-envelope-valid-count`
- `reviewer-envelope-invalid-count`
- `evidence-bundle-present-count`
- `reviewer-decision-accept-count`
- `repair-source hosted-reviewer-nonaccept` for non-accept envelopes
- `story-artifacts-preserved true`
- `proposed-accepted true`
- `capture-plan story-promotion-capture-plan.json`
- `promotion-decision accepted-for-promotion`
- `human-approval-required true`
- `promotion-source human-approved-story-promotion-batch`
- `human-approved-story-promotion-batch.fingerprint.txt`
- `batch-plan-fingerprint`
- `batch-entry-count 4`
- `story-promotion-review-fingerprint-count 4`
- `batch-entry example-30-story`
- `batch-entry example-65-story`
- `batch-entry example-80-story`
- `batch-entry example-90-story`
- `default-max-tokens`
- `max-tokens`
- `token-budget-default`
- `token-budget-warning` when present in the accepted story review
- `entry-count` from the corpus-copy replay
- `checker-result-count accepted` from the corpus-copy replay
- `checker-result-count rejected` from the corpus-copy replay
- `examples-report.txt`
- `v03-roadmap.txt`
- one decision: `accepted-for-promotion`, `needs-repair`, or
  `rejected-for-promotion`

## Rejection Rules

Return `needs-repair` or `rejected-for-promotion` when:

- story artifacts lose semantic anchors or agent trace evidence
- `story-promotion-capture-plan.json` is missing or its fingerprint file does
  not match
- `scripts/run_v03_story_promotion_import_demo.py` has not produced
  `story-promotion-import-demo-report.txt`
- the import demo does not report `story-artifacts-preserved true` and
  `proposed-accepted true`
- the import demo omits `promotion-decision accepted-for-promotion`,
  `human-approval-required true`, or
  `promotion-source human-approved-story-promotion-batch`
- the human-approved batch fingerprint is missing
- `scripts/run_v03_story_promotion_batch_plan.py` has not produced
  `story-promotion-batch-plan.txt` and
  `manifest.v03-story-promotion-batch-plan.txt`
- the batch plan does not report `batch-entry-count 4` and
  `story-promotion-review-fingerprint-count 4`
- replay counts are hardcoded instead of copied from the generated corpus
  replay report
- the visible hosted generation budget is missing
- hosted reviewer evidence is claimed but `models.json`,
  `model-check-model-id`, or the live reviewer review report is missing
- a hosted reviewer returns `needs-repair` or `reject` without a repair backlog
  that records `repair-source hosted-reviewer-nonaccept`
- `examples-report.txt` or `v03-roadmap.txt` is missing
- generated content was modified silently instead of preserving the original
  hosted output as evidence

Do not promote generated content into `./examples` unless deterministic replay,
story promotion review, and human approval all pass. When human approval is
available, use `scripts/capture_example_batch.py` with
`story_promotion_capture_plan_json`, `source_entry_id`, and the proposed
`entry_id` so the accepted story entry is appended to a corpus copy while the
source entry and reviewed story artifact bundle remain intact.
