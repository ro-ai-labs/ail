# Codex AIL Story Promotion Reviewer

version: 0.1.0
executor-label: codex-ail-story-promotion-reviewer
executor-family: codex-skill-agent
target artifact: AIL-Story-Promotion-Review
contract: examples/agents/codex-ail-story-promotion-reviewer.md

## Purpose

Create a Story promotion review report for reviewed User Story mode artifacts
before any story-derived generated content is proposed for promotion into
`./examples`.

## Allowed Inputs

- story harness artifact directory produced by
  `scripts/run_v03_story_llm_harness.py`
- `story-llm-harness-report.txt`
- `manifest.v03-story-llm.txt`
- story promotion capture plan artifacts
- human-approved request/response JSON for the proposed promoted story entry
- current examples replay report from `ail-examples examples --artifact-dir`
- current examples v0.3 roadmap artifact, `v03-roadmap.txt`
- reviewer notes about intended User Story mode promotion entries

## Required Output

Return an `AIL-Story-Promotion-Review` report that records:

- story harness review command:
  `scripts/run_v03_story_llm_harness.py --review-artifacts`
- story promotion capture-plan command:
  `scripts/run_v03_story_promotion_capture_plan.py --story-artifacts`
- story promotion import-demo command:
  `scripts/run_v03_story_promotion_import_demo.py`
- story id and normalized story id
- agent trace status, including `agent-trace` evidence
- semantic anchor review, including `semantic-anchor-missing-count 0`
- story promotion capture-plan artifacts:
  `story-promotion-capture-plan.json`,
  `story-promotion-capture-plan.txt`, and
  `story-promotion-capture-plan.fingerprint.txt`
- story promotion import-demo artifacts:
  `story-promotion-import-demo-report.txt` and
  `story-promotion-import-demo-report.fingerprint.txt`
- story promotion import-demo checks:
  `story-artifacts-preserved true`, `proposed-accepted true`,
  `capture-plan story-promotion-capture-plan.json`,
  `promotion-decision accepted-for-promotion`, `human-approval-required true`,
  `promotion-source human-approved-story-promotion-batch`, and
  `batch-plan-fingerprint`
- story promotion import-demo batch fingerprint:
  `human-approved-story-promotion-batch.fingerprint.txt`
- hosted generation budget checks:
  `default-max-tokens`, `max-tokens`, `token-budget-default`, and any
  `token-budget-warning` preserved by the capture plan and import-demo report
- release replay command used before promotion:
  `ail-examples examples --artifact-dir`
- v0.3 roadmap command used before promotion:
  `cargo run -- ail-v03-roadmap examples`
- v0.3 roadmap artifact reviewed:
  `v03-roadmap.txt`
- explicit decision: `accepted-for-promotion`, `needs-repair`, or
  `rejected-for-promotion`

## Forbidden Behavior

- Do not promote generated content into ./examples without a fingerprinted
  story-promotion capture plan, fingerprinted import-demo report, deterministic
  replay, and human approval.
- Do not treat User Story mode harness success as promotion approval.
- Do not rewrite generated story, spec, Core, bytecode, trace, request, or
  response artifacts to make them pass silently.
- Do not hide missing fingerprints, missing agent trace entries, missing
  semantic anchors, missing `v03-roadmap.txt`, or missing hosted token-budget
  evidence.
- Do not treat `accepted-for-promotion` as an automatic file edit.

## Replay Gate

The review is accepted only when:

```sh
python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm
python3 scripts/run_v03_story_promotion_capture_plan.py --story-artifacts /tmp/ail-v03-story-llm --output-dir /tmp/ail-v03-story-promotion-capture-plan
```

The capture plan must include `promotion_decision: accepted-for-promotion`,
`human_approval_required: true`, the reviewed story artifact directory, and the
visible hosted generation budget fields. It must write
`story-promotion-capture-plan.json`,
`story-promotion-capture-plan.txt`, and
`story-promotion-capture-plan.fingerprint.txt`.

After human approval, run the deterministic import demo:

```sh
python3 scripts/run_v03_story_promotion_import_demo.py \
  --story-artifacts /tmp/ail-v03-story-llm \
  --capture-plan-dir /tmp/ail-v03-story-promotion-capture-plan \
  --work-dir /tmp/ail-v03-story-promotion-import-work \
  --output-corpus /tmp/ail-v03-story-promotion-import-corpus \
  --output-artifacts /tmp/ail-v03-story-promotion-import-artifacts
```

The import report must include `story-promotion-import-demo-report.txt`,
`story-promotion-import-demo-report.fingerprint.txt`,
`story-artifacts-preserved true`, `proposed-accepted true`,
`promotion-decision accepted-for-promotion`, and
`human-approved-story-promotion-batch.fingerprint.txt`. The reviewer may then
prepare a batch entry with `source_entry_id`, `entry_id`,
`request_json_file`, `response_json_file`, and
`story_promotion_capture_plan_json`. The batch importer must append the
proposed accepted story entry in a corpus copy and must not rewrite the source
entry or the reviewed story artifact bundle.
