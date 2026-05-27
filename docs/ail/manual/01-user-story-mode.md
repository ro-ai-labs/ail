# AIL Manual: User Story Mode

## Purpose

User Story mode makes a story file the first-class entry point for authoring.
The story is reviewed as intent, not trusted code. The trusted path still runs
through checked requirements, accepted AIL-Spec, checked AIL-Core, verified
bytecode, default toolchain-agent evidence, and optional target evidence.

Use this chapter when validating the first AIL v0.3 authoring workflow.

Run the deterministic chapter checks without contacting a live model:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks
```

These checks exercise the local `ail-story` path with a stubbed chat endpoint
and verify the default toolchain-agent entrypoint path. They also verify the
blocking-question branch where the model needs clarification before
requirements can be trusted, explicit agent compatibility, and the native target
branch where a story-authored `CloseTicket` executable is run to produce a
runtime trace.

Run the same chapter with live-compatible fake or alternate endpoint evidence
by passing the endpoint through the manual runner:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter user-story-mode --run-checks --include-live \
  --live-endpoint http://127.0.0.1:8081/v1/chat/completions \
  --skip-model-check \
  --live-artifact-root /tmp/ail-manual-live-local
```

The runner forwards that endpoint to the story LLM harness and the direct
`ail-story --llm-endpoint` command, then rewrites the story promotion artifact
paths under `/tmp/ail-manual-live-local`.

## Story-First Run

Start with an existing support-ticket story and write all generated evidence to
a temporary artifact directory:

```sh
cargo run -- ail-story examples/support_ticket.ail \
  --story-file examples/stories/example-30.md \
  --artifact-dir /tmp/ail-user-story-mode \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
```

When `examples/ail_toolchain_agent.ail` is discoverable beside the example
package or from the repository root, `ail-story` uses it as the default
toolchain agent. Pass `--agent <path>` only when overriding that default.

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
/tmp/ail-user-story-mode/agent.ailbc.json
/tmp/ail-user-story-mode/manifest.ail-story.txt
/tmp/ail-user-story-mode/agent-trace.txt
/tmp/ail-user-story-mode/agent-trace.fingerprint.txt
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
transcript, the default agent bytecode and trace, and the underlying
`ail-build` manifest.

Default-agent story manifests include these direct evidence entries:

```text
agent agent.ailbc.json <fingerprint>
agent-trace agent-trace.txt <fingerprint>
```

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
  --artifact-dir /tmp/ail-user-story-mode-native \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions \
  --target linux-x86_64-elf \
  --action CloseTicket \
  --out /tmp/ail-user-story-mode-native/CloseTicket
```

The native path writes the same story evidence and delegates target artifact
checks to the existing build-agent verification path.

The deterministic local check for this branch is:

```sh
cargo test cli_ail_story_native_target_executes_story_runtime_trace --test ail_toolchain
```

It uses a stubbed chat endpoint, writes `target.elf`,
`native-bytecode-report.txt`, `dependency-report.txt`,
`manifest.ail-build.txt`, `manifest.ail-story.txt`, and `agent-trace.txt`,
then runs the generated native executable with:

```text
ticket.id=T-1 ticket.status=Open
```

The runtime evidence must include:

```text
ticket.status=Closed
trace TicketClosed
```

## Story Amendment Comparison

For `story-journey: story-amendment`, `ail-story` writes a comparison artifact
after checked requirements, accepted spec, checked Core, and bytecode exist:

```text
/tmp/ail-user-story-mode/story-amendment-comparison.txt
/tmp/ail-user-story-mode/story-amendment-comparison.fingerprint.txt
```

The deterministic local check for this branch is:

```sh
cargo test cli_ail_story_story_amendment_writes_comparison_artifact --test ail_toolchain
```

The comparison must include:

```text
AIL-Story-Amendment-Comparison:
story-journey story-amendment
story-roundtrip semantic-similar
comparison-result accepted
requirements-fingerprint fnv64:
accepted-spec-fingerprint fnv64:
checked-core-fingerprint fnv64:
bytecode-fingerprint fnv64:
semantic-anchor-preserved-count 4
semantic-anchor-missing-count 0
```

`story-mode-report.txt` records `story-amendment-comparison: present`, and
`manifest.ail-story.txt` records
`story-amendment-comparison story-amendment-comparison.txt <fingerprint>`.

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

The review also rejects the bundle if `agent-trace.fingerprint.txt` is
missing or does not match `agent-trace.txt`; this keeps promotion import from
accepting a trace that cannot be independently checked.
It also rejects question-only `llm/requirements.content.txt` or
`llm/spec.content.txt` envelopes during promotion review. Promotion evidence
must contain generated `artifact_text` for both the requirements and spec
stages, reported as `story-prompt-envelope-artifact-count 2` and
`story-prompt-envelope-questions-count 0`.

After the review is accepted, create a plan-only promotion capture artifact:

```sh
python3 scripts/run_v03_story_promotion_capture_plan.py \
  --story-artifacts /tmp/ail-v03-story-llm \
  --output-dir /tmp/ail-v03-story-promotion-capture-plan
```

That writes:

```text
/tmp/ail-v03-story-promotion-capture-plan/story-promotion-capture-plan.json
/tmp/ail-v03-story-promotion-capture-plan/story-promotion-capture-plan.txt
/tmp/ail-v03-story-promotion-capture-plan/story-promotion-capture-plan.fingerprint.txt
```

The plan records `promotion-decision accepted-for-promotion`,
`human-approval-required true`, the story review/report/manifest
fingerprints, transcript check count, and prompt-envelope counts. It does not
mutate `./examples`; it is the durable handoff for a later human-approved
batch capture.

After human approval, run the deterministic import demo against a corpus copy:

```sh
python3 scripts/run_v03_story_promotion_import_demo.py \
  --story-artifacts /tmp/ail-v03-story-llm \
  --capture-plan-dir /tmp/ail-v03-story-promotion-capture-plan \
  --work-dir /tmp/ail-v03-story-promotion-import-work \
  --output-corpus /tmp/ail-v03-story-promotion-import-corpus \
  --output-artifacts /tmp/ail-v03-story-promotion-import-artifacts
```

That writes:

```text
/tmp/ail-v03-story-promotion-import-work/story-promotion-import-demo-report.txt
/tmp/ail-v03-story-promotion-import-work/story-promotion-import-demo-report.fingerprint.txt
```

The report must include `story-artifacts-preserved true` and
`proposed-accepted true`. The output corpus copy stores the reviewed story
artifact bundle under `story-artifacts/<entry-id>/`, appends a promoted
accepted example, and replays it with `--release-evidence`. It still does not
mutate `./examples`.

The harness is intentionally outside the default test suite because it depends
on the hosted llama.cpp server and model behavior. Promote a live run into the
examples corpus only after the generated requirements, spec, Core, bytecode,
agent trace, manifest, story-promotion capture plan, and story-promotion import
demo report have been reviewed.

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
