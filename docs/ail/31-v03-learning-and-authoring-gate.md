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
- `program-domain`: one of `os-utility`, `c-interop`, `compiler`,
  `runtime`, `package-graph`, `application`, `agent-tool`, `ui-workflow`,
  `system-driver`, or `diagnostic`.
- `module-count`, `spec-count`, and `story-count`: positive integers; a
  `multi-module-system` entry must set each value to at least `2`.
- `interacts-with`: named modules, host contracts, packages, agents, or target
  surfaces crossed by the example, or `none` for a standalone utility.
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
- `semantic-anchors` in the referenced story file: at least three
  reviewer-visible terms, actions, modules, targets, or diagnostics that must
  survive story/spec/Core round-trips. Release evidence rejects a catalog entry
  whose story file omits these anchors.
- `distinctness-claim`: why this entry earns a slot, especially when it shares
  a package with other prompt-surface examples.
- `v0.3-signal`: the language, prompt, checker, runtime, target, or docs gap
  revealed by the example.

Prompt-surface matrices are allowed, but they are not automatically useful.
They count only when the distinctness claim identifies the prompt behavior,
checker assertion, target artifact, diagnostic, user-story journey, or
human-review path being validated.

The v0.2 verifier now enforces the first version of this usefulness bar:
`use-case` and `v0.3-signal` must be substantive, `v0.3-signal` must describe
a needed or recommended next-version improvement, and `distinctness-claim`
must name both the entry's `semantic-task`, its `capability-under-test`, and a
concrete differentiating axis such as prompt behavior, target artifact,
checker assertion, diagnostic, user-story journey, generated artifact,
executor, or human-review path. This does not make the corpus v0.3-complete,
but it prevents new entries from being counted when they are only labels around
a passing replay.

Learning signals are also checked as a corpus. The release verifier requires
at least ten distinct `v0.3-signal` values so one generic future note cannot be
copied across the 100-example suite. Replay reports must emit
`v03-signal-distinct-count` and one `v03-signal-count` line per signal; those
lines are the machine-readable backlog that turns examples into v0.3 language,
prompt, checker, runtime, target, and documentation work. The catalog metadata
field remains `v0.3-signal`; the report uses `v03-*` labels as artifact-safe
line keys.

Replay also writes `v03-roadmap.txt` as the dedicated backlog artifact. It
groups the same signals by count, entry id, capability level, program domain,
prompt file, story journey, and checker result so the interactive manual,
prompt-review agent, and release reviewer do not need to mine the full
`examples-report.txt` to decide which v0.3 improvements are being requested by
the corpus. The direct command is:

```bash
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-v03-roadmap --release-evidence
```

Repeated story families are checked across entries. Any `user-story-id` family
with at least five entries must cover at least three prompt files and at least
two story journeys, and the report must emit `story-family-count` plus
per-family entry, prompt-file, and story-journey counts. This allows useful
prompt matrices while rejecting label-only repetition.

Prompt-pack coverage is counted through accepted examples. Every required
system prompt must have at least one accepted catalog entry so prompt coverage
means the prompt generated an artifact that replayed through checked Core,
bytecode, and runtime or target evidence. The report must emit
`accepted-prompt-count` lines beside raw `prompt-count` lines; rejected-only
prompt appearances are useful diagnostics, but they cannot prove prompt-to-
artifact generation for that prompt.

The v0.3 usefulness gate must preserve domain breadth. At minimum, the release
verifier requires coverage for OS utilities, C interop, compiler passes,
runtime behavior, package graphs, application workflows, agent tools,
UI workflows, and system drivers. Diagnostics are also validated when present:
a diagnostic-domain example must be rejected or carry diagnostic story
evidence.

Domain coverage is not only a count. Each required domain must exercise at
least three prompt files and at least two story journeys, so one repeated
prompt path cannot stand in for a meaningful domain slice. The story files
under `examples/stories/` are also checked against the catalog for story,
journey, evidence, domain, count, and interaction metadata.

The verifier must require at least 10 distinct `user-story-id` values, at least
one high-level `application-workflow` story family with two or more replayed
entries, and coverage across `story-to-spec`, `spec-to-story`, and
`story-amendment` journeys.

## Required Learning Artifacts

Every `examples/*.ail/` package directory must include a package-local README
guide. These guides are part of the authoring surface, not optional prose:
support-only packages explain imported semantics, rejected diagnostic packages
explain useful failure paths, and counted catalog packages explain replay
artifacts.

The current required guide set is:

- `examples/support_ticket.ail/README.md`
- `examples/support_composed.ail/README.md`
- `examples/compiler_pass.ail/README.md`
- `examples/network_driver.ail/README.md`
- `examples/c_interop.ail/README.md`
- `examples/darwin_linux_effect.ail/README.md`
- `examples/ui_workflow.ail/README.md`
- `examples/refund_tool.ail/README.md`
- `examples/stateful_counter.ail/README.md`
- `examples/repeated_task.ail/README.md`
- `examples/runtime_generic.ail/README.md`
- `examples/secret_access.ail/README.md`
- `examples/incident_response.ail/README.md`
- `examples/ail_std_core.ail/README.md`
- `examples/ail_std_collections.ail/README.md`
- `examples/option_map.ail/README.md`
- `examples/ail_std_effects.ail/README.md`
- `examples/ail_std_security.ail/README.md`
- `examples/ail_std_runtime.ail/README.md`
- `examples/ail_toolchain_agent.ail/README.md`
- `examples/incident_identity.ail/README.md`
- `examples/incident_notifications.ail/README.md`
- `examples/incident_policy.ail/README.md`
- `examples/missing_registry_import.ail/README.md`
- `examples/recursive_factorial.ail/README.md`
- `examples/support_shared.ail/README.md`

Each README should state the purpose, concepts taught, files to inspect,
expected replay artifacts, rejected fixtures where applicable, and the next
example to read.

## Learning Guides

Current progress: all 26 package directories now have local guides. The
inventory is checked by `example_learning_readmes_cover_repeated_family_gaps`
for the high-volume teaching paths and by
`example_package_directories_all_have_learning_guides` for complete package
coverage, required section headings, and top-level `examples/README.md` links.

The `ail-examples` replay bundle must also write deterministic story artifacts:

- `examples/<entry-id>/user-story.txt`
- `examples/<entry-id>/user-story.fingerprint.txt`
- `examples/<entry-id>/ui-review.txt` for accepted UI workflow or UI-surface entries
- `examples/<entry-id>/ui-review.fingerprint.txt` for accepted UI workflow or UI-surface entries
- `examples/<entry-id>/repair-tutorial.txt` for rejected entries
- `examples/<entry-id>/repair-tutorial.fingerprint.txt` for rejected entries
- `examples/<entry-id>/repair-candidate.ail-spec.md` for rejected entries
- `examples/<entry-id>/repair-checked.ail-core.txt` for rejected entries
- `examples/<entry-id>/repair-artifact.ailbc.json` for rejected entries
- `examples/<entry-id>/repair-vm-trace.txt` or
  `examples/<entry-id>/repair-target-report.txt` for rejected entries
- `examples/<entry-id>/repair-diff.txt` for rejected entries
- `examples/<entry-id>/repair-promotion-review.txt` for rejected entries
- `v03-roadmap.txt`
- `v03-roadmap.fingerprint.txt`

The story artifact is derived from catalog metadata and fingerprinted in the
same report and manifest as request, response, checked Core, bytecode, VM
trace, native, target-report, UI review, diagnostics, and repair-tutorial
artifacts. Accepted `ui-workflow` entries and accepted entries tagged with the
`ui` surface must also emit `ui-review.txt`, which records deterministic visual
review, accessibility review, workflow authoring, runtime evidence,
semantic-anchor preservation, and upstream fingerprints. The report must
summarize these files with
`ui-review-fingerprint-*` lines and list each UI review in
`manifest.ail-examples.txt`. The repair tutorial is derived from
rejected-entry metadata and diagnostics so the corpus teaches how to move from
a failed prompt/spec response to a corrected spec. The repair proof chain must
then show that corrected spec reaching checked Core, verified bytecode, and
runtime or target evidence. The repair diff must connect rejected and repaired
fingerprints, mark the expected diagnostic as removed, and preserve story
semantic anchors for review. The repair promotion review must then make the
promotion decision explicit, including `accepted-for-promotion`,
`human-approval-required true`, the proposed accepted entry id, and
fingerprints for every upstream repair artifact. This makes promotion
auditable without automatically editing the corpus. The report must also
summarize semantic-anchor preservation with total, preserved, and missing
counts plus per-entry preservation lines. The roadmap artifact is fingerprinted
and listed in `manifest.ail-examples.txt` beside the examples report and
model-executor manifest.

## Minimum Proof Commands

```bash
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --run-checks
cargo test cli_ail_e2e_corpus_requires_replay_metadata
cargo test cli_ail_e2e_corpus_requires_capability_level_thresholds
cargo test cli_ail_e2e_corpus_requires_user_story_metadata
cargo test cli_ail_e2e_corpus_rejects_unknown_story_evidence
cargo test cli_ail_e2e_corpus_requires_story_diversity
cargo test cli_ail_e2e_corpus_requires_accepted_example_for_each_prompt_file
cargo test cli_ail_e2e_corpus_requires_v03_signal_diversity
cargo test cli_ail_e2e_corpus_replays_checked_live_release_corpus
cargo test --test ail_toolchain script_ail_interactive_manual_v03_authoring_gate_run_checks_succeeds
cargo run -- ail-examples examples --artifact-dir /tmp/ail-v03-learning-examples --release-evidence
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-v03-roadmap --release-evidence
test -f /tmp/ail-v03-learning-examples/v03-roadmap.txt
git diff --check -- examples docs/ail src tests scripts
```

## Current v0.3 Signals

Replay now emits `v03-signal-distinct-count` and `v03-signal-count` lines so
these gaps can be read from `examples-report.txt` instead of maintained only
as prose. The current examples reveal these next-version gaps:

- Package examples need package-local teaching guides, not only verifier input
  files.
- Prompt matrices need explicit separation between semantic use-case diversity
  and prompt-surface coverage.
- UI examples now emit deterministic visual review, accessibility review,
  workflow authoring artifacts, and a rejected accessibility diagnostic fixture
  that repairs to checked Core, verified bytecode, and Wasm target-contract
  evidence. The next bar is patchable visual review workflows.
- AgentTool examples need multi-agent handoff and policy-review scenarios.
- Compiler/self-hosting examples need pass composition and fixed-point checks.
- Systems examples need hardware-facing contracts, scheduler or interrupt
  semantics, and clearer unsupported-target migration guidance.
- Rejected examples now emit repair tutorials, corrected repair proof chains,
  semantic repair diffs, deterministic promotion review artifacts, and
  plan-only repair promotion capture plans; the next bar is a human-approved
  batch capture/import flow that can create the proposed accepted corpus entry
  while preserving the rejected evidence.
- Incident-response examples show that complex systems need richer story
  graphs across imported modules, UI surfaces, workflow transitions, target
  contracts, and regenerated story views.
- Stateful examples should move beyond single-action counters into persistence,
  idempotency, retries, migrations, locking, and replay after failure.
