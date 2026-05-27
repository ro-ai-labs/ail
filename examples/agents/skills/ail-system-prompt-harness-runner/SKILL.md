---
name: ail-system-prompt-harness-runner
description: Use when running AIL v0.3 system prompt, User Story mode, hosted llama.cpp, or interactive manual harness checks before review or corpus promotion.
---

# AIL System Prompt Harness Runner

## Purpose

Use this skill to execute the AIL v0.3 prompt-pack and interaction harnesses
before review. It runs the hosted llama.cpp probes, records model identity
evidence, and verifies the offline review reports before any generated content
is considered for promotion into `./examples`.

This is an execution skill, not a promotion reviewer. Use
`examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md` after these
artifacts exist.

## Inputs

- Current prompt files under `docs/ail/prompts/`.
- Current agent contracts under `examples/agents/`.
- Hosted llama.cpp endpoint: `http://inteligentia-pro-1:8080/`.
- Optional alternate endpoint for local fake-server tests.
- Optional artifact root for isolated runs.

## Run Sequence

Run the deterministic agent contract gate first:

```sh
cargo run -- ail-agent-contracts examples/agents
```

Print prompt-pack probes before contacting the hosted endpoint:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --dry-run
```

Run and review the hosted prompt-pack harness:

```sh
python3 scripts/run_v03_prompt_llm_harness.py
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

Review User Story mode artifacts when story harness output is present:

```sh
python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm
```

Run the interactive manual live checks when reproducing the full interaction
path:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks --include-live
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks --include-live
```

Replay corpus and roadmap evidence after harness review:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-system-prompt-examples --release-evidence
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-system-prompt-roadmap --release-evidence
```

## Required Evidence

The run is ready for reviewer handoff only when the artifacts include:

- `model-check present`
- `model-check-model-count`
- `model-check-model-id`
- `models.json` and `models.fingerprint.txt` for prompt-pack live runs
- `model-check.json` and `model-check.fingerprint.txt` for story live runs
- `prompt-llm-harness-report.txt`
- `prompt-llm-harness-review.txt`
- `prompt-llm-harness-review.fingerprint.txt`
- `prompt-envelope-valid-count`
- `prompt-envelope-artifact-required-count`
- `prompt-envelope-questions-expected-count`
- `prompt-outcome-match-count`
- `prompt-envelope-invalid-count`
- `story-llm-harness-report.txt`
- `story-mode-report.txt`
- `story-prompt-envelope-valid-count`
- `story-prompt-envelope-artifact-count`
- `story-prompt-envelope-questions-count`
- `story-prompt-envelope-invalid-count`
- `agent-trace present`
- `agent-trace.txt`
- `agent-trace.fingerprint.txt`
- `examples-report.txt`
- `v03-roadmap.txt`

## Failure Handling

Stop and preserve artifacts for review when:

- the hosted endpoint is unreachable
- `model-check missing`
- any response omits `model` or names a model absent from the model list
- prompt envelopes are invalid
- story artifacts omit `agent-trace.txt`
- interactive manual live checks fail
- corpus replay or roadmap evidence fails

Do not promote generated content into `./examples`; this skill only produces
reviewable evidence.
