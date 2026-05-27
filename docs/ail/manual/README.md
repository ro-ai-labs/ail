# AIL Interactive Manual

The AIL manual pairs prose chapters with a deterministic runner. The runner
lists the current authoring chapters, prints exact commands, and can run local
checks without contacting a live model by default.

List the available chapters:

```sh
python3 scripts/run_ail_interactive_manual.py --list
```

Print the User Story mode walkthrough:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --dry-run
```

Run deterministic User Story mode checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
```

Run every deterministic authoring chapter as one local audit:

```sh
python3 scripts/run_ail_interactive_manual.py --all --run-checks
```

Run the v0.3 authoring gate chapter directly:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks
```

The examples replay in this gate writes `v03-roadmap.txt`, which is the
machine-readable backlog of next-version language, prompt, checker, runtime,
target, and documentation improvements learned from the corpus.

Print only that roadmap view without reading the full examples report:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-roadmap --run-checks
```

Run the prompt interaction checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
```

Print the hosted prompt-pack harness without contacting the model:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --dry-run
```

Review completed hosted User Story mode artifacts before promotion:

```sh
python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm
```

The review writes `story-llm-harness-report.txt` and
`story-llm-harness-report.fingerprint.txt` in the reviewed artifact directory.
The report also records `story-llm-transcript-check-count`,
`story-prompt-envelope-valid-count`, and
`story-prompt-envelope-invalid-count` so prompt-pack conformance is reviewable
without re-contacting the hosted model.

Review completed hosted prompt-pack artifacts before promotion:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

The review writes `prompt-llm-harness-review.txt` and
`prompt-llm-harness-review.fingerprint.txt` in the reviewed artifact
directory.

Run deterministic local checks for a chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
```

Run deterministic agent-entrypoint checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
```

Run deterministic repair-promotion checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

Live LLM chapters are opt-in. Add `--include-live` only when
`http://inteligentia-pro-1:8080/` is reachable and the generated artifacts will
be reviewed before promotion into `./examples`.

For prompt-pack evidence, `--include-live` runs both the hosted harness and the
offline artifact review:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
```

## Chapters

- `user-story-mode`: story-first authoring with `ail-story`, checked
  requirements, blocking-question evidence, accepted spec, checked Core,
  bytecode, stored LLM request/response/content transcripts, prompt-envelope
  counts, agent trace, and a plan-only story promotion capture artifact.
  Prose: `01-user-story-mode.md`.
- `examples-release`: full `./examples` replay with release evidence and
  learning metadata. Prose: `02-examples-release.md`.
- `prompt-interaction`: prompt-pack and stored transcript inspection for system
  prompt interaction testing, plus an opt-in hosted llama.cpp prompt-pack
  harness. Prose: `03-prompt-interaction.md`.
- `agent-entrypoint`: Codex agent role files and the AIL toolchain-agent package
  that participates in the authoring pipeline. Prose:
  `04-agent-entrypoint.md`.
- `v03-roadmap`: direct next-version backlog generated from the examples
  corpus with `ail-v03-roadmap`. Prose: `05-v03-roadmap.md`.
- `repair-promotion`: deterministic review of rejected-example repair evidence
  before proposing a repaired artifact for accepted-corpus promotion. Prose:
  `07-repair-promotion.md`.
- `v03-authoring-gate`: the deterministic v0.3 audit that runs User Story
  mode, examples replay, roadmap printing, prompt interaction,
  agent-entrypoint, and repair-promotion checks from one manual chapter.
  Prose: `06-v03-authoring-gate.md`.
