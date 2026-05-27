# Codex AIL UI Patch Reviewer

version: 0.1.0
executor-label: codex-ail-ui-patch-reviewer
executor-family: codex-skill-agent
target artifact: AIL-UI-Patch-Import-Review
contract: examples/agents/codex-ail-ui-patch-reviewer.md

## Purpose

Create a UI patch import review report before any reviewed UI/flow patch is
proposed as a new accepted corpus entry.

## Allowed Inputs

- examples artifact directory produced by
  `ail-examples examples --artifact-dir ... --release-evidence`
- `examples-report.txt`
- `manifest.ail-examples.txt`
- accepted UI source entry id
- `ui-review.txt`
- `ui-review-patch.txt`
- `ui-review-patch.fingerprint.txt`
- `ui-patch-capture-plan.json`
- `ui-patch-capture-plan.fingerprint.txt`
- `ui-patch-import-demo-report.txt`
- `ui-patch-import-demo-report.fingerprint.txt`
- `ui-patch-runtime-state-check-report.txt`
- `ui-patch-runtime-state-check-report.fingerprint.txt`
- reviewer notes about intended UI patch promotion

## Required Output

Return an `AIL-UI-Patch-Import-Review` report that records:

- accepted UI source entry id
- proposed accepted entry id
- UI patch import decision: `accepted-for-import`, `needs-repair`, or
  `rejected-for-import`
- `human-approval-required true`
- `agent-contract-check ail-agent-contracts examples/agents`
- `visual-regression-review required`
- `runtime-ui-state-review required`
- `accessibility-review required`
- `flow-edit-review required`
- `ui-patch-import-status proposed-only`
- `ui-review-patch-fingerprint-observed-count`
- `ui-review-patch.txt`
- `ui-review-patch.fingerprint.txt`
- `ui-patch-capture-plan.json`
- `ui-patch-capture-plan.fingerprint.txt`
- `ui-patch-import-demo-report.txt`
- `ui-patch-import-demo-report.fingerprint.txt`
- `ui-patch-runtime-state-check-report.txt`
- `ui-patch-runtime-state-check-report.fingerprint.txt`
- `visual-regression-fingerprint-preserved true`
- `runtime-ui-state-check target-report`
- `runtime-ui-state-anchor Ticket.reviewStatus`
- `source-preserved true`
- `proposed-accepted true`
- `flow-edit-applied true`
- `patched-core-replayed true`

## Forbidden Behavior

- Do not promote generated content into ./examples without deterministic replay
  and human approval.
- Do not treat a UI patch review or capture plan as sufficient unless the
  import demo reports `source-preserved true`, `proposed-accepted true`,
  `flow-edit-applied true`, and `patched-core-replayed true`.
- Do not accept a visual/UI patch when the runtime state witness is missing
  `visual-regression-fingerprint-preserved true` or
  `runtime-ui-state-anchor Ticket.reviewStatus`.
- Do not rewrite the reviewed source entry during promotion; the source UI
  entry remains part of the learning corpus.
- Do not treat `accepted-for-import` as an automatic corpus edit.

## Replay Gate

The review is accepted only when:

```sh
cargo run -- ail-agent-contracts examples/agents
cargo run -- ail-examples examples --artifact-dir /tmp/ail-ui-patch-review --release-evidence
python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks
```

The resulting report must include `ui-review-patch.txt`,
`ui-review-patch.fingerprint.txt`, and
`ui-review-patch-fingerprint-observed-count`. The plan-only import bridge must
also be generated with:

```sh
python3 scripts/run_v03_ui_patch_capture_plan.py \
  --examples-artifacts /tmp/ail-ui-patch-review \
  --entry-id <ui-entry-id> \
  --output-dir /tmp/ail-ui-patch-capture-plan
```

After human approval, run the deterministic import demo and runtime witness:

```sh
python3 scripts/run_v03_ui_patch_import_demo.py \
  --base-corpus examples \
  --examples-artifacts /tmp/ail-ui-patch-review \
  --capture-plan-dir /tmp/ail-ui-patch-capture-plan \
  --source-entry-id <ui-entry-id> \
  --work-dir /tmp/ail-ui-patch-import-work \
  --output-corpus /tmp/ail-ui-patch-import-corpus \
  --output-artifacts /tmp/ail-ui-patch-import-artifacts

python3 scripts/run_v03_ui_patch_runtime_state_check.py \
  --examples-artifacts /tmp/ail-ui-patch-review \
  --capture-plan-dir /tmp/ail-ui-patch-capture-plan \
  --import-work-dir /tmp/ail-ui-patch-import-work \
  --output-artifacts /tmp/ail-ui-patch-import-artifacts \
  --source-entry-id <ui-entry-id> \
  --output-dir /tmp/ail-ui-patch-import-work
```

The review may prepare a batch entry only after the import demo and runtime
witness preserve source fingerprints, replay the patched Core, and bind the
visual review to the runtime UI state anchor.
