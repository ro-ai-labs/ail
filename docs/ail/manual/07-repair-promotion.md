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

## Capture Plan

After a review records `promotion-decision accepted-for-promotion`, generate a
plan-only capture artifact before creating any accepted corpus entry:

```sh
python3 scripts/run_v03_repair_promotion_capture_plan.py \
  --examples-artifacts /tmp/ail-manual-repair-promotion \
  --entry-id example-99 \
  --output-dir /tmp/ail-manual-repair-promotion-capture-plan
```

The script writes:

```text
/tmp/ail-manual-repair-promotion-capture-plan/repair-promotion-capture-plan.json
/tmp/ail-manual-repair-promotion-capture-plan/repair-promotion-capture-plan.txt
/tmp/ail-manual-repair-promotion-capture-plan/repair-promotion-capture-plan.fingerprint.txt
```

The capture plan is deliberately not a corpus edit. It verifies the promotion
review fingerprint, checks that report and manifest entries still point at the
rejected example evidence, records `preserve_rejected_entry: true`, and names
`scripts/capture_example_batch.py` as the human-approved batch capture path.

## Human-Approved Import

After a human reviewer supplies approved request and response JSON, append the
repaired entry into a corpus copy with the batch importer:

```sh
python3 scripts/capture_example_batch.py \
  --base-corpus examples \
  --output-dir /tmp/ail-repair-promotion-import-corpus \
  --plan-json /tmp/human-approved-repair-promotion-batch.json
```

The batch entry must name the rejected source and the proposed accepted entry:

```json
{
  "entries": [
    {
      "entry_id": "example-99-repaired",
      "source_entry_id": "example-99",
      "executor_family": "codex-skill-agent",
      "executor_label": "codex-ail-repair-promotion-reviewer",
      "semantic_task": "support-ticket-repair-promoted-99",
      "request_json_file": "/tmp/approved-request.json",
      "response_json_file": "/tmp/approved-response.json",
      "checker_result": "accepted",
      "repair_promotion_capture_plan_json": "/tmp/ail-manual-repair-promotion-capture-plan/repair-promotion-capture-plan.json"
    }
  ]
}
```

The importer validates the capture-plan fingerprint and required promotion
fields, keeps `example-99` rejected, appends `example-99-repaired`, writes new
request, response, and story files, and leaves final acceptance to
`cargo run -- ail-examples <corpus-copy> --artifact-dir <artifacts>`.

The manual can run a deterministic import demo after replay and capture-plan
generation:

```sh
python3 scripts/run_v03_repair_promotion_import_demo.py \
  --base-corpus examples \
  --examples-artifacts /tmp/ail-manual-repair-promotion \
  --capture-plan-dir /tmp/ail-manual-repair-promotion-capture-plan \
  --source-entry-id example-99 \
  --work-dir /tmp/ail-manual-repair-promotion-import-work \
  --output-corpus /tmp/ail-manual-repair-promotion-import-corpus \
  --output-artifacts /tmp/ail-manual-repair-promotion-import-artifacts
```

The demo uses the replayed repair candidate as deterministic human-approved
input, appends the proposed entry in a corpus copy, replays that copy, and
writes:

```text
/tmp/ail-manual-repair-promotion-import-work/repair-promotion-import-demo-report.txt
/tmp/ail-manual-repair-promotion-import-work/repair-promotion-import-demo-report.fingerprint.txt
```

The report must include `source-preserved true`, `proposed-accepted true`,
`entry-count 118`, `checker-result-count accepted 109`, and
`checker-result-count rejected 9`.
