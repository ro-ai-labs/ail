---
name: ail-ui-patch-reviewer
description: Use when reviewing AIL UI patch import evidence before proposing a reviewed UI or flow patch for accepted-corpus promotion.
---

# AIL UI Patch Reviewer

## Purpose

Use this skill to review UI patch import evidence before any visual or flow
patch is proposed for promotion into `./examples`.

This skill implements the evidence contract in
`examples/agents/codex-ail-ui-patch-reviewer.md`. The model may review UI patch
evidence, but deterministic replay remains the authority.

## Required Inputs

- Current examples corpus under `examples/`.
- Current UI review artifacts produced by `ail-examples`.
- `examples-report.txt`.
- `manifest.ail-examples.txt`.
- Accepted UI source entry id and reviewer notes.

## Review Sequence

Run the deterministic contract gate first:

```sh
cargo run -- ail-agent-contracts examples/agents
```

Run the UI patch import manual chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks
```

Replay examples directly when reviewing a different artifact directory:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-ui-patch-review --release-evidence
```

## Required Evidence

The review report must include:

- `ui-review-patch.txt`
- `ui-review-patch.fingerprint.txt`
- `ui-review-patch-fingerprint-observed-count`
- `ui-patch-capture-plan.json`
- `ui-patch-capture-plan.fingerprint.txt`
- `ui-patch-import-demo-report.txt`
- `ui-patch-import-demo-report.fingerprint.txt`
- `ui-patch-runtime-state-check-report.txt`
- `ui-patch-runtime-state-check-report.fingerprint.txt`
- `accepted-for-import`, `needs-repair`, or `rejected-for-import`
- `human-approval-required true`
- `visual-regression-fingerprint-preserved true`
- `runtime-ui-state-anchor Ticket.reviewStatus`
- `source-preserved true`
- `proposed-accepted true`
- `flow-edit-applied true`
- `patched-core-replayed true`
- UI review, reviewed patch, checked Core, bytecode, target report, and
  runtime state witness fingerprints
- `preserve_source_entry: true`
- `must_supply_request_response_json: true`
- `batch_capture_script: scripts/capture_example_batch.py`

## Rejection Rules

Return `needs-repair` or `rejected-for-import` when:

- `ui-review-patch.txt` is missing or its fingerprint file does not match
- deterministic replay does not list the UI patch review in
  `manifest.ail-examples.txt`
- `scripts/run_v03_ui_patch_import_demo.py` has not produced
  `ui-patch-import-demo-report.txt`
- the import demo does not report `source-preserved true`,
  `proposed-accepted true`, `flow-edit-applied true`, and
  `patched-core-replayed true`
- `scripts/run_v03_ui_patch_runtime_state_check.py` has not produced
  `ui-patch-runtime-state-check-report.txt`
- the runtime state witness does not report
  `visual-regression-fingerprint-preserved true` and
  `runtime-ui-state-anchor Ticket.reviewStatus`
- checked Core, bytecode, target evidence, accessibility evidence, or runtime
  state evidence is missing
- the artifact implies automatic promotion without human approval

Do not promote generated content into `./examples` unless deterministic replay,
UI patch review, runtime UI-state evidence, and human approval all pass. When
human approval is available, use `scripts/capture_example_batch.py` with the
reviewed request/response pair and the proposed `entry_id` so the UI-patched
accepted entry is appended to a corpus copy while the source UI entry remains
intact.
