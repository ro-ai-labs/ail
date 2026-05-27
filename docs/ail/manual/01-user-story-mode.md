# AIL Manual: User Story Mode

## Purpose

User Story mode makes a story file the first-class entry point for authoring.
The story is reviewed as intent, not trusted code. The trusted path still runs
through checked requirements, accepted AIL-Spec, checked AIL-Core, verified
bytecode, and optional agent or target evidence.

Use this chapter when validating the first AIL v0.3 authoring workflow.

Run the deterministic chapter checks without contacting a live model:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
```

These checks exercise the local `ail-story` path with a stubbed chat endpoint
and verify both the plain story authoring path and the toolchain-agent
entrypoint path. They also verify the blocking-question branch where the model
needs clarification before requirements can be trusted.

## Story-First Run

Start with an existing support-ticket story and write all generated evidence to
a temporary artifact directory:

```sh
cargo run -- ail-story examples/support_ticket.ail \
  --story-file examples/stories/example-30.md \
  --agent examples/ail_toolchain_agent.ail \
  --artifact-dir /tmp/ail-user-story-mode \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
```

The story file must include at least:

```text
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
semantic-anchors: Support Tickets; Close ticket; TicketClosed; internal notes; linux-x86_64-elf; interview.system.md
```

If a story is missing `user-story`, `acceptance-criteria`, or at least three
semantic anchors, `ail-story` prints `AIL-STORY-` diagnostics and exits before
contacting an LLM.

## Artifact Walkthrough

After a successful compile run, inspect these files:

```text
/tmp/ail-user-story-mode/story.source.md
/tmp/ail-user-story-mode/story.normalized.md
/tmp/ail-user-story-mode/story-mode-report.txt
/tmp/ail-user-story-mode/requirements.ail-requirements.md
/tmp/ail-user-story-mode/accepted.ail-spec.md
/tmp/ail-user-story-mode/checked.ail-core.txt
/tmp/ail-user-story-mode/review.ail-flow.json
/tmp/ail-user-story-mode/artifact.ailbc.json
/tmp/ail-user-story-mode/manifest.ail-story.txt
/tmp/ail-user-story-mode/agent-trace.txt
/tmp/ail-user-story-mode/llm/requirements.request.json
/tmp/ail-user-story-mode/llm/requirements.response.json
/tmp/ail-user-story-mode/llm/requirements.content.txt
/tmp/ail-user-story-mode/llm/spec.request.json
/tmp/ail-user-story-mode/llm/spec.response.json
/tmp/ail-user-story-mode/llm/spec.content.txt
```

`story.normalized.md` records defaulted story metadata such as
`story-journey: story-to-spec` and `story-roundtrip: semantic-similar`.
`story-mode-report.txt` records package identity, story identity, endpoint, and
semantic-anchor count. It also records `story-llm-transcript-count`,
`story-prompt-envelope-valid-count`, and
`story-prompt-envelope-invalid-count` when LLM transcripts are present.
`manifest.ail-story.txt` fingerprints story, generated requirements, accepted
spec, checked Core, bytecode, each stored LLM request/response/content
transcript, and the underlying `ail-build` manifest.

When the requirements prompt returns blocking questions instead of an
`AIL-Requirements` artifact, `ail-story` prints `ail-story blocking questions:`,
writes `story-questions.ail-interview.md`, fingerprints it, records it in
`manifest.ail-story.txt`, and exits before `checked.ail-core.txt` or
`artifact.ailbc.json` can be emitted.

`agent-trace.txt` should include:

```text
entrypoint=ail-story
buildrequest.story-id=<story id>
buildrequest.semantic-anchors=<anchor list>
action CaptureRequirements started
action PrepareSpecDraft started
action AcceptSpecDraft started
action CompileApplication started
action VerifyBytecodeArtifact started
```

This proves the AI Agent entry point received the story identity and semantic
anchors before the LLM prompts and kept participating after bytecode emission,
while the compiler and verifier remain the authority.

## Native Target Variant

To request native output, name the action and target:

```sh
cargo run -- ail-story examples/support_ticket.ail \
  --story-file examples/stories/example-30.md \
  --agent examples/ail_toolchain_agent.ail \
  --artifact-dir /tmp/ail-user-story-mode-native \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions \
  --target linux-x86_64-elf \
  --action CloseTicket \
  --out /tmp/ail-user-story-mode-native/CloseTicket
```

The native path writes the same story evidence and delegates target artifact
checks to the existing build-agent verification path.

## Live Harness

Use the harness in dry-run mode first:

```sh
python3 scripts/run_v03_story_llm_harness.py --dry-run
```

Then run it live when `http://inteligentia-pro-1:8080/` is reachable:

```sh
python3 scripts/run_v03_story_llm_harness.py
```

Review the completed live artifact directory before promotion:

```sh
python3 scripts/run_v03_story_llm_harness.py --review-artifacts /tmp/ail-v03-story-llm
```

That review writes:

```text
/tmp/ail-v03-story-llm/story-llm-harness-report.txt
/tmp/ail-v03-story-llm/story-llm-harness-report.fingerprint.txt
```

The harness is intentionally outside the default test suite because it depends
on the hosted llama.cpp server and model behavior. Promote a live run into the
examples corpus only after the generated requirements, spec, Core, bytecode,
agent trace, and manifest have been reviewed.

The review mode is offline. It checks story source and normalized story
fingerprints, story-mode report metadata, generated requirements, accepted
spec, checked Core, flow review, bytecode, story manifest fingerprints,
stored LLM request/response/content transcripts, prompt-envelope validity
counts, and toolchain-agent trace order. It then persists the same
accepted/rejected review text as a fingerprinted harness report before a live
run can be treated as promotion candidate evidence.

The harness probes `http://inteligentia-pro-1:8080/v1/models` and runs
`ail-story` against `http://inteligentia-pro-1:8080/v1/chat/completions` by
default. That path has the strongest artifact-format behavior for the hosted
llama.cpp model; root `/completion` endpoints remain supported for other
servers but may require prompt/model tuning.
