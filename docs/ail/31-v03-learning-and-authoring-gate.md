# AIL v0.3 Learning And Authoring Gate

## Purpose

AIL v0.2 proves that stored prompts and checked packages can replay through the
full toolchain. AIL v0.3 should raise the bar from "the examples pass" to "the
examples teach AIL and expose the next missing capabilities."

The v0.3 gate is complete when the examples form a coherent authoring ladder
from low-level development to high-level workflows, and when each example
records what it teaches, why it is distinct, and what it tells us to improve in
the language or toolchain.

## Completion Definition

AIL v0.3 learning and authoring is complete when this chain is reproducible:

```text
useful scenario
  -> prompt or package source
  -> checked AIL-Spec or AIL-Core
  -> compile and replay evidence
  -> human-readable walkthrough
  -> v0.3 learning signal
  -> promoted language, toolchain, prompt, or documentation improvement
```

Every counted example must still satisfy the v0.2 end-to-end replay path. The
new v0.3 requirement is that replay evidence is paired with learning evidence:
what the example is for, what capability it exercises, and how it informs the
next version.

## Capability Levels

The examples must cover all three levels:

- `low-level`: compiler passes, ABI/FFI contracts, system effects, backend
  portability, bytecode/native/Wasm boundaries, and unsafe host interactions.
- `mid-level`: packages, standard library APIs, generic values, runtime state,
  permissions, secrets, failures, traces, and dependency graphs.
- `high-level`: application workflows, UI workflows, AgentTool behavior,
  scheduled work, repair flows, and human-reviewable authoring.

The `ail-examples` verifier requires at least 20 entries at each level. This is
a floor, not the desired final shape. A v0.3-ready corpus should also make the
levels easy to read in `examples/README.md` and package-local READMEs.

## Required Example Metadata

Each entry in `examples/examples.md` must include:

- `use-case`: practical scenario the example teaches or proves.
- `capability-level`: one of `low-level`, `mid-level`, or `high-level`.
- `capability-under-test`: concrete AIL surface under pressure.
- `program-scale`: one of `utility`, `module`, or `multi-module-system`.
- `user-story-id`: stable story family used to group prompt, target, and
  repair variants.
- `user-story`: one-line story in reviewer-facing form.
- `acceptance-criteria`: observable criteria tied to checked artifacts.
- `story-evidence`: strongest artifact that proves the story path, one of
  `checked-core`, `bytecode`, `vm-trace`, `target-report`, or `diagnostics`.
- `story-journey`: one of `story-to-spec`, `spec-to-story`,
  `story-amendment`, or `diagnostic-story`.
- `story-roundtrip`: `semantic-similar` for accepted stories or
  `diagnostic-preserving` for rejected diagnostic stories.
- `distinctness-claim`: why this entry earns a slot, especially when it shares
  a package with other prompt-surface examples.
- `v0.3-signal`: the language, prompt, checker, runtime, target, or docs gap
  revealed by the example.

Prompt-surface matrices are allowed, but they are not automatically useful.
They count only when the distinctness claim identifies the prompt behavior,
checker assertion, target artifact, diagnostic, user-story journey, or
human-review path being validated.

The verifier must require at least 10 distinct `user-story-id` values, at least
one high-level `application-workflow` story family with two or more replayed
entries, and coverage across `story-to-spec`, `spec-to-story`, and
`story-amendment` journeys.

## Required Learning Artifacts

Before claiming v0.3 complete, the repository should add package-local README
files for the main teaching packages:

- `examples/support_ticket.ail`
- `examples/refund_tool.ail`
- `examples/compiler_pass.ail`
- `examples/network_driver.ail`
- `examples/c_interop.ail`
- `examples/ui_workflow.ail`
- `examples/ail_std_core.ail`
- `examples/ail_std_collections.ail`
- `examples/ail_std_effects.ail`
- `examples/ail_std_security.ail`
- `examples/ail_std_runtime.ail`

Each README should state the purpose, concepts taught, files to inspect,
expected replay artifacts, rejected fixtures where applicable, and the next
example to read.

The `ail-examples` replay bundle must also write deterministic story artifacts:

- `examples/<entry-id>/user-story.txt`
- `examples/<entry-id>/user-story.fingerprint.txt`

The story artifact is derived from catalog metadata and fingerprinted in the
same report and manifest as request, response, checked Core, bytecode, VM
trace, native, target-report, and diagnostics artifacts.

## Minimum Proof Commands

```bash
cargo test cli_ail_e2e_corpus_requires_replay_metadata
cargo test cli_ail_e2e_corpus_requires_capability_level_thresholds
cargo test cli_ail_e2e_corpus_requires_user_story_metadata
cargo test cli_ail_e2e_corpus_rejects_unknown_story_evidence
cargo test cli_ail_e2e_corpus_requires_story_diversity
cargo test cli_ail_e2e_corpus_replays_checked_live_release_corpus
cargo run -- ail-examples examples --artifact-dir /tmp/ail-v03-learning-examples --release-evidence
git diff --check -- examples docs/ail src tests scripts
```

## Current v0.3 Signals

The current examples reveal these next-version gaps:

- Package examples need package-local teaching guides, not only verifier input
  files.
- Prompt matrices need explicit separation between semantic use-case diversity
  and prompt-surface coverage.
- UI examples need stronger visual review, accessibility review, and workflow
  authoring artifacts.
- AgentTool examples need multi-agent handoff and policy-review scenarios.
- Compiler/self-hosting examples need pass composition and fixed-point checks.
- Systems examples need hardware-facing contracts, scheduler or interrupt
  semantics, and clearer unsupported-target migration guidance.
- Rejected examples need repair tutorials that turn diagnostics into corrected
  specs.
