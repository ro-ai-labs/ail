---
name: ail-prompt-interaction-reviewer
description: Use when reviewing AIL v0.3 prompt-pack, User Story mode, hosted llama.cpp, or Codex skill-agent interaction artifacts before promoting generated content into examples.
---

# AIL Prompt Interaction Reviewer

## Purpose

Use this skill to review AIL prompt/system-interaction evidence before any
hosted LLM or Codex skill-agent output is promoted into `./examples`.

This skill implements the evidence contract in
`examples/agents/codex-ail-prompt-reviewer.md`. The model may draft or review
artifacts, but deterministic replay remains the authority.
Repair promotion decisions are covered by
`examples/agents/codex-ail-repair-promotion-reviewer.md`; prompt and story
reviewers must still verify that generated artifacts do not bypass
`repair-promotion-review.txt`.

## Required Inputs

- Prompt harness artifacts under `/tmp/ail-v03-prompt-llm`.
- Story harness artifacts under `/tmp/ail-v03-story-llm`.
- Current prompt files under `docs/ail/prompts/`.
- Current agent contracts under `examples/agents/`.
- Current examples replay and v0.3 roadmap artifacts.
- Current repair promotion artifacts when generated content repairs a rejected
  example.
- Optional hosted llama.cpp endpoint: `http://inteligentia-pro-1:8080/`.

## Review Sequence

Run the deterministic contract gate first:

```sh
cargo run -- ail-agent-contracts examples/agents
```

Inspect the prompt-pack dry run before using the hosted endpoint:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --dry-run
```

When the hosted endpoint is reachable, run the live prompt harness and review
the saved artifacts:

```sh
python3 scripts/run_v03_prompt_llm_harness.py
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

Review User Story mode artifacts:

```sh
python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm
```

Run the interactive manual live gate when prompt interaction evidence must be
reproduced end to end:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
```

Replay examples and write the v0.3 roadmap evidence before promotion:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-prompt-review-examples --release-evidence
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-prompt-review-roadmap --release-evidence
```

When generated output repairs a rejected example, run the repair promotion
chapter before promotion:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

## Required Evidence

The review report must include:

- `prompt-envelope-valid-count`
- `prompt-envelope-invalid-count`
- `manifest.v03-prompt-llm.txt`
- `prompt-llm-harness-report.txt`
- `prompt-llm-harness-review.txt`
- `prompt-llm-harness-review.fingerprint.txt`
- `story-llm-harness-report.txt`
- `examples-report.txt`
- `v03-roadmap.txt`
- `repair-promotion-review.txt`
- `repair-promotion-review.fingerprint.txt`
- `repair-promotion-review-fingerprint-observed-count`
- `repair-promotion-import-demo-report.txt`
- `source-preserved true`
- `proposed-accepted true`
- prompt file fingerprints when available
- probe labels and probe fingerprints
- expected `artifact_kind` validation for every prompt
- one decision: `accepted-for-promotion`, `needs-repair`, or
  `rejected-for-promotion`

## Rejection Rules

Return `needs-repair` or `rejected-for-promotion` when:

- any prompt envelope is invalid
- probe metadata is missing, stale, or generic
- hosted output is non-empty but fails offline review
- story artifacts lose semantic anchors or agent trace evidence
- `examples-report.txt` or `v03-roadmap.txt` is missing
- generated content was modified silently instead of preserving the original
  hosted output as evidence

Do not promote generated content into `./examples` unless deterministic replay
and roadmap review pass.
