# AIL v0.3 User Story Mode Design

## Status

Design for the first AIL v0.3 implementation slice.

## Context

AIL v0.2 has a replayable examples corpus, checked prompt-pack outputs, story
metadata, semantic anchors, package-local learning guides, and an `ail-build`
pipeline that can move from prompt or saved requirements to AIL-Spec,
AIL-Core, bytecode, VM traces, and target artifacts.

The v0.3 objective raises the bar from "examples replay" to "examples teach
and drive authoring." The first useful v0.3 slice therefore makes user
stories an entrypoint into the toolchain, not only metadata emitted by
`ail-examples`.

## Problem

The current command surface supports:

- `ail-interview`: user prompt to blocking questions or interview artifact.
- `ail-requirements`: user prompt plus optional interview answers to checked
  AIL-Requirements.
- `ail-spec`: checked requirements to checked AIL-Spec.
- `ail-build`: prompt or saved requirements/spec/Core to compile artifacts.
- `ail-examples`: stored corpus replay with story files and semantic anchors.

It does not yet provide a single first-class story-first command where a
reviewer can supply a story file and ask the AIL toolchain agent to turn that
story into requirements, checked spec, checked Core, bytecode, and optional
binary or target evidence.

## Goals

1. Add a first-class `ail-story` command.
2. Treat a story file as a checked authoring input with explicit metadata,
   acceptance criteria, and semantic anchors.
3. Reuse the existing prompt-pack and `ail-build` trust boundary: LLMs and
   agents may draft artifacts, but parser, checker, compiler, VM, and target
   artifact verification remain authoritative.
4. Write deterministic story-mode artifacts and a manifest so the workflow can
   become part of the v0.3 release evidence.
5. Give the future interactive manual a concrete story-first workflow to teach.
6. Give the future prompt/agent harness a concrete workflow to exercise against
   `http://inteligentia-pro-1:8080/`.

## Non-Goals

- Do not replace `ail-build`.
- Do not make free-form prose trusted compiler input.
- Do not require a live LLM for normal unit tests.
- Do not complete the full v0.3 manual or prompt harness in this first slice.
- Do not add a browser UI. The first interactive manual can be Markdown plus
  runnable commands and generated artifacts.

## Command Shape

```sh
cargo run -- ail-story <package-dir> \
  --story-file <story.md> \
  --artifact-dir <dir> \
  [--llm-endpoint <url>] \
  [--agent <agent-package-or-bytecode>] \
  [--target <target> --action <ActionName> --out <path>]
```

`ail-story` is a story-first wrapper around the existing trusted path:

```text
story file
  -> story-mode preflight
  -> AIL-Requirements prompt
  -> checked AIL-Requirements
  -> AIL-Spec prompt
  -> checked AIL-Spec
  -> checked AIL-Core
  -> verified bytecode
  -> VM trace or target artifact
  -> story-mode manifest
```

The command accepts the same local LLM endpoint shape already used by
`ail-interview`, `ail-requirements`, `ail-spec`, and `ail-build`.

## Story File Contract

The initial `ail-story` input is Markdown with frontmatter-like key-value
fields. It reuses the fields already enforced for corpus stories:

- `user-story-id`
- `user-story`
- `acceptance-criteria`
- `story-journey`
- `story-roundtrip`
- `story-evidence`
- `program-domain`
- `module-count`
- `spec-count`
- `story-count`
- `interacts-with`
- `semantic-anchors`

For story-first authoring, these fields are required:

- `user-story`
- `acceptance-criteria`
- `semantic-anchors`

`semantic-anchors` must contain at least three anchors. This preserves the
v0.3 rule that every counted example is tied to reviewer-visible semantics.

`story-journey` defaults to `story-to-spec` when absent. `story-roundtrip`
defaults to `semantic-similar` for accepted story mode. The defaults are
written into the artifact bundle so later replay is deterministic.

## Preflight Validation

`ail-story` must fail before contacting an LLM when:

- the story file is missing;
- `user-story` is empty;
- `acceptance-criteria` is empty;
- `semantic-anchors` has fewer than three anchors;
- a required numeric metadata field is present but not a positive integer;
- `story-roundtrip` or `story-journey` has an unknown value.

The diagnostic prefix is `AIL-STORY-`.

## Prompting Behavior

The requirements prompt receives:

- package manifest;
- source package spec, if present;
- story text;
- story metadata;
- semantic anchors;
- acceptance criteria;
- explicit instruction that the story is authoring input, not trusted code.

The spec prompt receives checked requirements plus the same story
metadata, so the generated AIL-Spec can preserve anchors and acceptance
criteria.

If the LLM returns blocking questions instead of an artifact, `ail-story`
writes `story-questions.ail-interview.md`, records the prompt envelope in the
manifest, and exits nonzero without compiling.

## Agent Entry Point

When `--agent` is supplied, `ail-story` uses the same toolchain-agent
trust boundary already used by `ail-build`.

The first implementation reuses existing agent actions where possible:

- `CaptureRequirements`: receives story metadata and acceptance criteria.
- `PrepareSpecDraft`: receives checked requirements and story anchors.
- `AcceptSpecDraft`: receives checked requirements and accepted spec.
- `AcceptCoreIR`, `CompileApplication`, `VerifyBytecodeArtifact`, and
  `VerifyTargetArtifact`: behave like `ail-build`.

The manifest identifies this as `entrypoint: ail-story` so later v0.3
agent harnesses can distinguish story-first runs from prompt-first runs.

## Artifacts

With `--artifact-dir`, `ail-story` writes:

- `source.ail-package.md`
- `source.ail-spec.md`, when the package has a source spec
- `story.source.md`
- `story.normalized.md`
- `story-mode-report.txt`
- `requirements.ail-requirements.md`
- `accepted.ail-spec.md`
- `checked.ail-core.txt`
- `artifact.ailbc.json`
- `vm-trace.txt`, when a VM action is run
- `target-<ActionName>.elf` or target contract artifact, when target output is
  requested
- `manifest.ail-story.txt`
- fingerprint files for every artifact above

The report records:

- package name and version;
- user-story-id;
- semantic anchors;
- prompt files and fingerprints used;
- LLM endpoint label when provided;
- agent package or bytecode fingerprint when provided;
- checker result;
- bytecode fingerprint;
- target and native artifact fingerprint when present.

## Interactive Manual Hook

The manual does not land before the workflow exists. The first manual
chapter can be added after `ail-story` lands:

```text
docs/ail/manual/01-user-story-mode.md
```

That chapter teaches one story-first path using an existing example story
from `examples/stories/`, then shows the generated requirements, spec, Core,
bytecode, and target evidence.

## Prompt And LLM Harness Hook

After `ail-story` lands, add a harness command or script that runs a small
matrix against the local llama.cpp server:

```sh
curl -sS http://inteligentia-pro-1:8080/v1/models
cargo run -- ail-story examples/support_ticket.ail \
  --story-file examples/stories/example-30.md \
  --artifact-dir /tmp/ail-v03-story-llm \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
```

The harness is not part of the default unit suite because it depends on
network availability and model behavior. It produces checked artifacts
and a portability report suitable for manual promotion into the corpus.

## Implementation Plan

### Slice 1: Story Parser And CLI Surface

- Extend `usage()` and command recognition for `ail-story`.
- Add `--story-file` parsing.
- Parse story metadata into a typed local struct.
- Add preflight diagnostics with `AIL-STORY-` codes.
- Tests:
  - `cli_ail_story_requires_story_file`
  - `cli_ail_story_rejects_missing_user_story`
  - `cli_ail_story_rejects_missing_semantic_anchors`

### Slice 2: Story To Build Pipeline

- Convert story metadata into the prompt currently accepted by
  `draft_checked_ail_requirements_for_package`.
- Reuse the `ail-build` requirements/spec/Core/bytecode path.
- Write `story.source.md`, `story.normalized.md`, `story-mode-report.txt`, and
  `manifest.ail-story.txt`.
- Tests:
  - `cli_ail_story_builds_checked_artifacts_from_story_file`
  - `cli_ail_story_writes_story_manifest`
  - `cli_ail_story_preserves_semantic_anchors_in_requirements_prompt`

### Slice 3: Agent Entry Point

- Thread story metadata through existing `ail-build --agent` handoff context.
- Mark agent trace with `entrypoint=ail-story`.
- Tests:
  - `cli_ail_story_agent_records_capture_requirements`
  - `cli_ail_story_agent_verifies_bytecode_artifact`

### Slice 4: Manual And Harness

- Add `docs/ail/manual/01-user-story-mode.md`.
- Add a small non-default script for live prompt testing against
  `http://inteligentia-pro-1:8080/v1/chat/completions`, with model discovery
  through `http://inteligentia-pro-1:8080/v1/models`.
- Tests:
  - manual link/path smoke test;
  - script help text smoke test;
  - no network dependency in default tests.

## Acceptance Gates

Before claiming this v0.3 slice complete:

```sh
cargo test cli_ail_story --test ail_toolchain
cargo test cli_ail_build_agent --test ail_toolchain
cargo test cli_ail_e2e_corpus_replays_checked_live_release_corpus --test ail_toolchain
cargo test -- --test-threads=1
git diff --check
```

For live LLM evidence, run separately:

```sh
curl -sS http://inteligentia-pro-1:8080/v1/models
cargo run -- ail-story examples/support_ticket.ail \
  --story-file examples/stories/example-30.md \
  --artifact-dir /tmp/ail-v03-story-llm \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
```

Live LLM evidence is useful for promotion, but the implementation must be
deterministically testable without it.
