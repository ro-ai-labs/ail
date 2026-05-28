---
name: ail-repair-promotion-reviewer
description: Use when reviewing AIL rejected-example repair evidence before proposing a repaired artifact for accepted-corpus promotion.
---

# AIL Repair Promotion Reviewer

## Purpose

Use this skill to review rejected-example repair evidence before any repaired
artifact is proposed for promotion into `./examples`.

This skill implements the evidence contract in
`examples/agents/codex-ail-repair-promotion-reviewer.md`. The model may review
promotion evidence, but deterministic replay remains the authority.

## Required Inputs

- Current examples corpus under `examples/`.
- Current repair artifacts produced by `ail-examples`.
- `examples-report.txt`.
- `manifest.ail-examples.txt`.
- Rejected entry id and reviewer notes.

## Review Sequence

Run the deterministic contract gate first:

```sh
cargo run -- ail-agent-contracts examples/agents
```

Run the repair promotion manual chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

Replay examples directly when reviewing a different artifact directory:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-repair-promotion-review --release-evidence
```

Run the batch repair audit before judging the roadmap signal:

```sh
python3 scripts/run_v03_rejected_repair_audit.py \
  --base-corpus examples \
  --examples-artifacts /tmp/ail-repair-promotion-review \
  --artifact-dir /tmp/ail-rejected-repair-audit
```

## Required Evidence

The review report must include:

- `repair-promotion-review.txt`
- `repair-promotion-review.fingerprint.txt`
- `repair-promotion-review-fingerprint-observed-count`
- `rejected-repair-audit-report.txt`
- `manifest.v03-rejected-repair-audit.txt`
- `signal-entry-count 8`
- `promotion-ready-count 8`
- `repair-promotion-capture-plan.json`
- `repair-promotion-capture-plan.fingerprint.txt`
- `repair-promotion-import-demo-report.txt`
- `repair-promotion-import-demo-report.fingerprint.txt`
- `accepted-for-promotion`, `needs-repair`, or `rejected-for-promotion`
- `human-approval-required true`
- `expected-diagnostic-removed true`
- `semantic-anchor-missing-count 0`
- `source-preserved true`
- `proposed-accepted true`
- diagnostics, repair tutorial, repair candidate, checked Core, bytecode,
  repair evidence, and repair diff fingerprints
- `preserve_rejected_entry: true`
- `must_supply_request_response_json: true`
- `batch_capture_script: scripts/capture_example_batch.py`

## Rejection Rules

Return `needs-repair` or `rejected-for-promotion` when:

- `repair-promotion-review.txt` is missing or its fingerprint file does not
  match
- deterministic replay does not list the promotion review in
  `manifest.ail-examples.txt`
- `scripts/run_v03_repair_promotion_import_demo.py` has not produced
  `repair-promotion-import-demo-report.txt`
- the import demo does not report `source-preserved true` and
  `proposed-accepted true`
- the expected diagnostic is not removed
- checked Core, bytecode, VM evidence, or target evidence is missing
- semantic anchors are missing
- the artifact implies automatic promotion without human approval

Do not promote generated content into `./examples` unless deterministic replay,
repair promotion review, and human approval all pass.
When human approval is available, use `scripts/capture_example_batch.py` with
`repair_promotion_capture_plan_json`, `source_entry_id`, and the proposed
`entry_id` so the repaired accepted entry is appended to a corpus copy while
the rejected source entry remains intact.
The deterministic wrapper is:

```sh
python3 scripts/run_v03_repair_promotion_import_demo.py
```
