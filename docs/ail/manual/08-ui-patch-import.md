# AIL Manual: UI Patch Import

## Purpose

The UI patch import chapter turns deterministic UI review patch plans into a
human-approved corpus-copy import. It keeps the source UI example unchanged,
checks the proposed `ail-flow-edit`, writes approved request/response
transcripts, appends a new accepted entry to a scratch corpus, and replays that
corpus through `ail-examples`.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks
```

## Workflow

First replay the release corpus to materialize UI review evidence:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-manual-ui-patch --release-evidence
```

The replay must include:

```text
ui-review.txt
ui-review-patch.txt
ui-review-patch.fingerprint.txt
ui-review-patch-fingerprint-observed-count
entry-artifact example-108 ui-review-patch
```

Then build the plan-only capture artifact:

```sh
python3 scripts/run_v03_ui_patch_capture_plan.py \
  --examples-artifacts /tmp/ail-manual-ui-patch \
  --entry-id example-108 \
  --output-dir /tmp/ail-manual-ui-patch-capture-plan
```

The plan writes:

```text
ui-patch-capture-plan.json
ui-patch-capture-plan.txt
ui-patch-capture-plan.fingerprint.txt
patch-command ail-flow-edit
human-approval-required true
preserve-source-entry true
```

Finally run the deterministic import demo:

```sh
python3 scripts/run_v03_ui_patch_import_demo.py \
  --base-corpus examples \
  --examples-artifacts /tmp/ail-manual-ui-patch \
  --capture-plan-dir /tmp/ail-manual-ui-patch-capture-plan \
  --source-entry-id example-108 \
  --work-dir /tmp/ail-manual-ui-patch-import-work \
  --output-corpus /tmp/ail-manual-ui-patch-import-corpus \
  --output-artifacts /tmp/ail-manual-ui-patch-import-artifacts
```

The demo report must include:

```text
ui-patch-import-demo-report.txt
ui-patch-import-demo-report.fingerprint.txt
source-preserved true
proposed-accepted true
ui-review-patch-fingerprint-preserved true
checked-core-fingerprint-preserved true
flow-edit-applied true
patched-core-replayed true
entry-count 126
checker-result-count accepted 117
checker-result-count rejected 9
```

After the import demo, write the deterministic runtime UI-state witness:

```sh
python3 scripts/run_v03_ui_patch_runtime_state_check.py \
  --examples-artifacts /tmp/ail-manual-ui-patch \
  --capture-plan-dir /tmp/ail-manual-ui-patch-capture-plan \
  --import-work-dir /tmp/ail-manual-ui-patch-import-work \
  --output-artifacts /tmp/ail-manual-ui-patch-import-artifacts \
  --source-entry-id example-108 \
  --output-dir /tmp/ail-manual-ui-patch-import-work
```

The witness report must include:

```text
ui-patch-runtime-state-check-report.txt
ui-patch-runtime-state-check-report.fingerprint.txt
visual-regression-baseline ui-review.txt
visual-regression-patch ui-review-patch.txt
visual-regression-fingerprint-preserved true
runtime-ui-state-check target-report
runtime-ui-state-anchor Ticket.reviewStatus
flow-edit-applied true
patched-core-replayed true
```

The import uses `ail-flow-edit` to prove the reviewed patch can add
`Ticket.reviewStatus` to the checked UI Core. The promoted response is the
human-approved source spec with that reviewed field added, so existing action
trace wording remains parseable during corpus replay. The runtime state witness
then binds the visual review and patch fingerprints to the promoted target
report and the replayed Core anchor added by the patch.
