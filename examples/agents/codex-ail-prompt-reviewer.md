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
- reviewer notes about intended promotion entries

## Required Output

Return an `AIL-Prompt-Interaction-Review` report that records:

- prompt harness review command:
  `scripts/run_v03_prompt_llm_harness.py --review-artifacts`
- story harness review command:
  `scripts/run_v03_story_llm_harness.py --review-artifacts`
- prompt files reviewed and their fingerprints when available
- story id, semantic-anchor count, manifest checks, and agent trace status when
  story artifacts are reviewed
- release replay command used before promotion:
  `ail-examples examples --artifact-dir`
- explicit decision: `accepted-for-promotion`, `needs-repair`, or
  `rejected-for-promotion`

## Forbidden Behavior

- Do not promote generated content into ./examples without passing offline
  harness review and deterministic replay.
- Do not rewrite generated specs to make them pass silently; report the repair
  needed and preserve the original hosted output as evidence.
- Do not claim model quality, compiler trust, runtime success, or release
  readiness from non-empty LLM output alone.
- Do not hide missing fingerprints, empty prompt content, missing agent trace
  entries, or semantic-anchor loss.

## Replay Gate

The review is accepted only when both relevant offline review commands pass and
the promoted corpus copy passes `ail-examples examples --artifact-dir ...`
with `--release-evidence`. If either harness review is missing or rejected, the
review must return `needs-repair` or `rejected-for-promotion`.
