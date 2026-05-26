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

Run the prompt interaction checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks
```

Run deterministic local checks for a chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
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
  prompt interaction testing.
- `agent-entrypoint`: Codex agent role files and the AIL toolchain-agent package
  that participates in the authoring pipeline.
