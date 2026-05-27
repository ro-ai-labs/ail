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

Those checks now include the story-amendment comparison branch, which writes
and fingerprints `story-amendment-comparison.txt` for
`story-journey: story-amendment` inputs.

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
The report also verifies `agent-trace.fingerprint.txt` before promotion, and
records `story-llm-transcript-check-count`,
`story-prompt-envelope-valid-count`,
`story-prompt-envelope-artifact-count`,
`story-prompt-envelope-questions-count`, and
`story-prompt-envelope-invalid-count` so prompt-pack conformance is reviewable
without re-contacting the hosted model.

Review completed hosted prompt-pack artifacts before promotion:

```sh
python3 scripts/run_v03_prompt_llm_harness.py --review-artifacts /tmp/ail-v03-prompt-llm
```

The review writes `prompt-llm-harness-review.txt` and
`prompt-llm-harness-review.fingerprint.txt` in the reviewed artifact
directory.

Print the hosted AgentTool policy reviewer harness without contacting the
model. The dry run also fingerprints the deterministic evidence bundle when
the default manual artifact paths already exist:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --dry-run
```

Review completed hosted AgentTool policy reviewer artifacts before promotion:

```sh
python3 scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts /tmp/ail-v03-agent-policy-live-review
```

The review writes `agent-policy-live-review-review.txt` and
`agent-policy-live-review-review.fingerprint.txt` in the reviewed artifact
directory, and checks `agent-policy-live-review-report.txt`,
`manifest.v03-agent-policy-live-review.txt`,
`reviewer-envelope-valid-count`, `reviewer-envelope-invalid-count`,
`evidence-bundle-present-count`,
`reviewer-decision-accept-count`,
`reviewer-decision-needs-repair-count`, and
`reviewer-decision-reject-count`. A live reviewer run is accepted only when
all five reviewer roles return `decision: accept` and every recorded request
contains the complete deterministic evidence bundle with artifact
fingerprints; valid non-accept decisions produce `review-result needs-repair`.

Run deterministic local checks for a chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
```

Run deterministic agent-entrypoint checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
```

Run deterministic bootstrap self-hosting checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter bootstrap-self-hosting --run-checks
```

Run deterministic Systems profile checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter systems-profile --run-checks
```

Run deterministic Application baseline checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter application-baseline --run-checks
```

Run deterministic repair-promotion checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter repair-promotion --run-checks
```

Run deterministic UI patch import checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter ui-patch-import --run-checks
```

Run deterministic AgentTool policy import checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks
```

Live LLM chapters are opt-in. Add `--include-live` only when
`http://inteligentia-pro-1:8080/` is reachable and the generated artifacts will
be reviewed before promotion into `./examples`.

Use the same manual commands against a local fake or alternate hosted endpoint
by threading the endpoint through the manual runner:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --dry-run --include-live \
  --live-endpoint http://127.0.0.1:8081/v1/chat/completions \
  --skip-model-check \
  --live-artifact-root /tmp/ail-manual-live-local
```

`--live-endpoint` is forwarded to nested live manual chapters, the story LLM
harness, prompt-pack harness, AgentTool policy reviewer harness, and the direct
`ail-story --llm-endpoint` command. `--skip-model-check` forwards the matching
harness option for fake endpoints without `/v1/models`. `--live-artifact-root`
rewrites the known manual `/tmp/ail-*` artifact paths under one root so local
live rehearsals do not overwrite hosted-run evidence.

For prompt-pack evidence, `--include-live` runs both the hosted harness and the
offline artifact review:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter prompt-interaction --run-checks --include-live
```

For AgentTool policy evidence, `--include-live` runs the hosted reviewer
harness and the offline artifact review:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-policy-import --run-checks --include-live
```

## Chapters

- `user-story-mode`: story-first authoring with `ail-story`, checked
  requirements, blocking-question evidence, accepted spec, checked Core,
  bytecode, stored LLM request/response/content transcripts, prompt-envelope
  counts, agent trace and fingerprint evidence, native target runtime-trace evidence, a
  story promotion capture artifact, and a corpus-copy import demo with
  `story-promotion-import-demo-report.txt`,
  `story-artifacts-preserved true`, and `proposed-accepted true`. Prose:
  `01-user-story-mode.md`.
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
- `bootstrap-self-hosting`: deterministic bootstrap bundle for the AIL
  toolchain agent and AIL-Meta compiler pass, with fixed-point,
  host-boundary, dependency, native handoff, and manifest evidence. Prose:
  `10-bootstrap-self-hosting.md`.
- `systems-profile`: deterministic conformance, native compile, and runtime
  trace evidence for the `network_driver.ail` low-level Systems profile package,
  including scheduler and interrupt accepted/rejected fixtures. Prose:
  `11-systems-profile.md`.
- `application-baseline`: deterministic conformance evidence for the
  `support_ticket.ail` high-level Application profile package, including
  package-local accepted and rejected fixtures. Prose:
  `12-application-baseline.md`.
- `repair-promotion`: deterministic review of rejected-example repair evidence
  before proposing a repaired artifact for accepted-corpus promotion. Prose:
  `07-repair-promotion.md`.
- `ui-patch-import`: deterministic review of UI patch plans before importing a
  human-approved `ail-flow-edit` candidate into a replayed corpus copy, then
  writing visual-regression and runtime UI-state evidence for the imported
  patch. Prose: `08-ui-patch-import.md`.
- `agent-policy-import`: deterministic review of AgentTool policy handoff
  artifacts before importing a human-approved policy trace amendment into a
  replayed corpus copy, then writing a role-separated deterministic
  multi-agent handoff witness and optional hosted reviewer evidence. Prose:
  `09-agent-policy-import.md`.
- `v03-authoring-gate`: the deterministic v0.3 audit that runs User Story
  mode, examples replay, roadmap printing, prompt interaction,
  agent-entrypoint, bootstrap self-hosting, Systems profile,
  Application baseline,
  repair-promotion, UI patch import, and AgentTool policy import checks from
  one manual chapter.
  Prose: `06-v03-authoring-gate.md`.
