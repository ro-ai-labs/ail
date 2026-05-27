# Codex AIL Prompt Reviewer

version: 0.1.0
executor-label: codex-ail-prompt-reviewer
executor-family: codex-skill-agent
target artifact: AIL-Prompt-Interaction-Review
contract: examples/agents/codex-ail-prompt-reviewer.md

## Purpose

Create a Prompt and story harness review report for hosted prompt-pack and User
Story mode harness artifacts before any generated content is promoted into
`./examples`.

## Allowed Inputs

- prompt harness artifact directory produced by
  `scripts/run_v03_prompt_llm_harness.py`
- story harness artifact directory produced by
  `scripts/run_v03_story_llm_harness.py`
- current prompt-pack files under `docs/ail/prompts/`
- current examples replay report from `ail-examples examples --artifact-dir`
- current examples v0.3 roadmap artifact, `v03-roadmap.txt`
- reviewer notes about intended promotion entries

## Required Output

Return an `AIL-Prompt-Interaction-Review` report that records:

- prompt harness review command:
  `scripts/run_v03_prompt_llm_harness.py --review-artifacts`
- story harness review command:
  `scripts/run_v03_story_llm_harness.py --review-artifacts`
- prompt files reviewed and their fingerprints when available
- prompt-specific probe labels/fingerprints and expected `artifact_kind`
  validation status
- expected prompt outcome validation status, including
  `prompt-outcome-match-count`
- whether hosted requests include the inline envelope contract, JSON mode
  request hint, and adequate token budget for complete envelopes
- prompt-envelope validation counts from the prompt harness review, including
  `prompt-envelope-valid-count`,
  `prompt-envelope-artifact-required-count`,
  `prompt-envelope-questions-expected-count`, and
  `prompt-envelope-invalid-count`
- story id, semantic-anchor count, manifest checks, and agent trace status when
  story artifacts are reviewed
- release replay command used before promotion:
  `ail-examples examples --artifact-dir`
- v0.3 roadmap command used before promotion:
  `cargo run -- ail-v03-roadmap examples`
- v0.3 roadmap artifact reviewed:
  `v03-roadmap.txt`
- explicit decision: `accepted-for-promotion`, `needs-repair`, or
  `rejected-for-promotion`

## Forbidden Behavior

- Do not promote generated content into ./examples without passing offline
  harness review and deterministic replay.
- Do not rewrite generated specs to make them pass silently; report the repair
  needed and preserve the original hosted output as evidence.
- Do not claim model quality, compiler trust, runtime success, next-version
  learning, or release readiness from non-empty LLM output alone.
- Do not accept raw hosted prompt output unless the offline prompt harness
  review reports prompt-pack envelope validity for every required prompt.
- Do not accept generic prompt smoke-test output when the review reports
  mismatched probe metadata or an unexpected `artifact_kind`.
- Do not accept question-only prompt output for prompts whose task-specific
  probe requires a generated artifact.
- Do not hide missing fingerprints, empty prompt content, missing agent trace
  entries, missing `v03-roadmap.txt`, or semantic-anchor loss.

## Replay Gate

The review is accepted only when both relevant offline review commands pass and
the promoted corpus copy passes `ail-examples examples --artifact-dir ...`
with `--release-evidence`, `cargo run -- ail-v03-roadmap examples ...`
writes `v03-roadmap.txt`, and the roadmap signals are reviewed. If either
harness review is missing or rejected, the review must return `needs-repair`
or `rejected-for-promotion`.
