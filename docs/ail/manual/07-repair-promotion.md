# AIL Manual: Repair Promotion Review

## Purpose

The repair promotion chapter reviews rejected-example repair evidence before a
repaired artifact is proposed as a new accepted corpus entry. It does not
promote files by itself. It proves that replay produced a deterministic review
decision that ties the rejected diagnostic, corrected spec, checked Core,
bytecode, runtime or target evidence, semantic anchors, and repair diff
together.

Run the chapter checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

## Release Replay

Generate the repair promotion evidence bundle:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-manual-repair-promotion --release-evidence
```

Each rejected entry writes:

```text
/tmp/ail-manual-repair-promotion/examples/<entry-id>/repair-promotion-review.txt
/tmp/ail-manual-repair-promotion/examples/<entry-id>/repair-promotion-review.fingerprint.txt
```

The report also records `repair-promotion-review-fingerprint-observed-count`
and lists each promotion review in `manifest.ail-examples.txt`.

## Review Rule

Treat `promotion-decision accepted-for-promotion` as a proposal for reviewer
approval, not as an automatic corpus edit. The artifact is acceptable only when
it records:

- `checker-result rejected-to-repaired`
- the original `failure-taxonomy`
- the original `expected-diagnostic`
- `expected-diagnostic-removed true`
- `repair-evidence-kind repair-vm-trace` or `repair-evidence-kind repair-target-report`
- fingerprints for diagnostics, repair tutorial, repair candidate, checked
  Core, bytecode, repair evidence, and repair diff
- `semantic-anchor-missing-count 0`
- `human-approval-required true`

Inspect representative report and manifest lines:

```sh
rg -n "repair-promotion-review-fingerprint-observed-count|entry-artifact example-99 repair-promotion-review|entry-artifact example-107 repair-promotion-review" \
  /tmp/ail-manual-repair-promotion/examples-report.txt \
  /tmp/ail-manual-repair-promotion/manifest.ail-examples.txt
```
