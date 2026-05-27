# Codex AIL Repair Promotion Reviewer

version: 0.1.0
executor-label: codex-ail-repair-promotion-reviewer
executor-family: codex-skill-agent
target artifact: AIL-Repair-Promotion-Review
contract: examples/agents/codex-ail-repair-promotion-reviewer.md

## Purpose

Create a Repair promotion review report for rejected examples that already
replay through diagnostics, repair tutorial, corrected spec, checked Core,
verified bytecode, VM or target evidence, and repair diff.

## Allowed Inputs

- examples artifact directory produced by
  `ail-examples examples --artifact-dir ... --release-evidence`
- `examples-report.txt`
- `manifest.ail-examples.txt`
- rejected entry id
- `repair-promotion-review.txt`
- `repair-promotion-review.fingerprint.txt`
- `repair-promotion-capture-plan.json`
- `repair-promotion-capture-plan.fingerprint.txt`
- reviewer notes about intended accepted-corpus promotion

## Required Output

Return an `AIL-Repair-Promotion-Review` report that records:

- rejected entry id
- proposed accepted entry id
- promotion decision: `accepted-for-promotion`, `needs-repair`, or
  `rejected-for-promotion`
- `human-approval-required true`
- original failure taxonomy
- original expected diagnostic
- `expected-diagnostic-removed true`
- `repair-evidence-kind repair-vm-trace` or
  `repair-evidence-kind repair-target-report`
- fingerprints for diagnostics, repair tutorial, repair candidate, checked
  Core, bytecode, repair evidence, and repair diff
- `semantic-anchor-missing-count 0`
- `repair-promotion-review-fingerprint-observed-count`

## Forbidden Behavior

- Do not promote generated content into ./examples without passing deterministic
  replay and human approval.
- Do not delete or rewrite rejected evidence after promotion; rejected evidence
  remains part of the learning corpus.
- Do not accept a repaired artifact when `expected-diagnostic-removed true`,
  checked Core, bytecode verification, VM or target evidence, or
  `semantic-anchor-missing-count 0` is missing.
- Do not treat `accepted-for-promotion` as an automatic file edit.

## Replay Gate

The review is accepted only when:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-repair-promotion-review --release-evidence
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

The resulting report must include `repair-promotion-review.txt`,
`repair-promotion-review.fingerprint.txt`, and
`repair-promotion-review-fingerprint-observed-count`. The plan-only promotion
bridge must also be generated with:

```sh
python3 scripts/run_v03_repair_promotion_capture_plan.py \
  --examples-artifacts /tmp/ail-repair-promotion-review \
  --entry-id <rejected-entry-id> \
  --output-dir /tmp/ail-repair-promotion-capture-plan
```

The capture plan must include `preserve_rejected_entry: true`,
`must_supply_request_response_json: true`, and
`batch_capture_script: scripts/capture_example_batch.py`.
