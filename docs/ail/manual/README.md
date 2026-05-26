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

Run deterministic local checks for a chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
```

Run deterministic agent-entrypoint checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
```

Live LLM chapters are opt-in. Add `--include-live` only when
`http://inteligentia-pro-1:8080/` is reachable and the generated artifacts will
be reviewed before promotion into `./examples`.

## Chapters

- `user-story-mode`: story-first authoring with `ail-story`, checked
  requirements, accepted spec, checked Core, bytecode, and agent trace. Prose:
  `01-user-story-mode.md`.
- `examples-release`: full `./examples` replay with release evidence and
  learning metadata.
- `prompt-interaction`: prompt-pack and stored transcript inspection for system
  prompt interaction testing, plus an opt-in hosted llama.cpp prompt-pack
  harness.
- `agent-entrypoint`: Codex agent role files and the AIL toolchain-agent package
  that participates in the authoring pipeline.
- `v03-roadmap`: direct next-version backlog generated from the examples
  corpus with `ail-v03-roadmap`.
- `v03-authoring-gate`: the deterministic v0.3 audit that runs User Story
  mode, examples replay, roadmap printing, prompt interaction, and
  agent-entrypoint checks from one manual chapter.
