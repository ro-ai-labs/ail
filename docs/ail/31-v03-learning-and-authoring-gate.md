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
  Replay rejects the entry if the named artifact is not actually produced for
  that entry.
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

Catalog path closure is part of the gate. `request-file`, `response-file`, and
`story-file` must stay inside the catalog directory, while `package` must stay
inside repository `./examples`; absolute paths and `..` path escapes are
rejected before replay. Package closure is also part of the gate: every
top-level `examples/*.ail` directory must either be counted as a `package:` in
`examples/examples.md` or be declared in `examples/support-packages.md` with
`role: support-only`, a concrete `used-by` relationship, and an explanation of
why its evidence flows through counted examples, toolchain commands, manual
chapters, docs, or regression tests.

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

The interactive manual is also part of the v0.3 gate. Live rehearsals must be
reproducible against either the hosted llama.cpp server or a local fake
endpoint by using the manual runner transport overrides:

```bash
python3 scripts/run_ail_interactive_manual.py --chapter v03-authoring-gate --dry-run --include-live \
  --live-endpoint http://127.0.0.1:8081/v1/chat/completions \
  --skip-model-check \
  --live-artifact-root /tmp/ail-manual-live-local
```

Those flags must propagate to the live User Story mode, prompt interaction,
AgentTool policy, and direct `ail-story --llm-endpoint` commands so the manual
can validate prompt interactions without depending on one network endpoint.
For `/v1/chat/completions`, the main Rust authoring path must use the same
prompt shape as the hosted prompt harness: prompt-pack text in a `system`
message, story or command payload in a `user` message, `stream: false`,
disabled thinking, and JSON mode via
`response_format: {"type":"json_object"}`. Root `/completion` compatibility is
kept for local servers that only accept a single prompt.

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
artifacts. Support-only package status is machine-checked in
`examples/support-packages.md`; it does not create a separate example category.

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
- `examples/<entry-id>/ui-review-patch.txt` for accepted UI workflow or UI-surface entries
- `examples/<entry-id>/ui-review-patch.fingerprint.txt` for accepted UI workflow or UI-surface entries
- `examples/<entry-id>/agent-policy-review.txt` for accepted AgentTool entries
- `examples/<entry-id>/agent-policy-review.fingerprint.txt` for accepted AgentTool entries
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
- `bootstrap-fixed-point-report.txt`, `bootstrap-native-bytecode-report.txt`,
  `bootstrap-host-boundary-report.txt`, `bootstrap-dependency-report.txt`,
  `bootstrap-handoff-report.txt`, and `manifest.ail-bootstrap.txt` in a
  scratch bootstrap artifact directory when the AIL-authored toolchain agent
  and AIL-Meta compiler pass are run through `ail-bootstrap`.
- `ui-patch-capture-plan.json`, `ui-patch-capture-plan.txt`, and
  `ui-patch-capture-plan.fingerprint.txt` in a scratch capture-plan directory
  when a UI patch is proposed for human-approved import.
- `ui-patch-import-demo-report.txt` and
  `ui-patch-import-demo-report.fingerprint.txt` in a scratch import work
  directory when the approved UI patch is appended to a corpus copy and
  replayed.
- `ui-patch-runtime-state-check-report.txt` and
  `ui-patch-runtime-state-check-report.fingerprint.txt` in the import work
  directory when the imported UI patch has visual-review fingerprint evidence
  and runtime UI-state anchors checked against the promoted target report.
- `agent-policy-capture-plan.json`, `agent-policy-capture-plan.txt`, and
  `agent-policy-capture-plan.fingerprint.txt` in a scratch capture-plan
  directory when an AgentTool policy handoff is proposed for human-approved
  import.
- `agent-policy-import-demo-report.txt` and
  `agent-policy-import-demo-report.fingerprint.txt` in a scratch import work
  directory when the approved policy handoff is appended to a corpus copy and
  replayed.
- `agent-policy-multi-agent-handoff-report.txt` and
  `agent-policy-multi-agent-handoff-report.fingerprint.txt` in the same
  scratch import work directory when the AgentTool policy import is validated
  by a role-separated deterministic handoff witness.
- `agent-policy-live-review-report.txt`,
  `agent-policy-live-review-report.fingerprint.txt`,
  `manifest.v03-agent-policy-live-review.txt`,
  `agent-policy-live-review-review.txt`, and
  `agent-policy-live-review-review.fingerprint.txt` in a live reviewer artifact
  directory when hosted AgentTool policy reviewer roles are executed against a
  complete deterministic evidence bundle and then reviewed offline.

The story artifact is derived from catalog metadata and fingerprinted in the
same report and manifest as request, response, checked Core, bytecode, VM
trace, native, target-report, UI review, UI review patch, agent policy review,
diagnostics, and repair-tutorial artifacts. Accepted `ui-workflow` entries and
accepted entries tagged with the `ui` surface must also emit `ui-review.txt`,
which records deterministic visual review, accessibility review, workflow
authoring, runtime evidence, semantic-anchor preservation, and upstream
fingerprints. Replay must also emit `ui-review-patch.txt`, a proposed-only
deterministic patch plan that names the `ail-flow-edit` handoff, requires human
approval, and binds itself to the UI review fingerprint. The report must
summarize these files with
`ui-review-fingerprint-*` and `ui-review-patch-fingerprint-*` lines and list
each UI review and patch plan in `manifest.ail-examples.txt`. A reviewed UI
patch can then be captured with `scripts/run_v03_ui_patch_capture_plan.py` and
imported with `scripts/run_v03_ui_patch_import_demo.py`; that demo must
preserve the source entry, apply `ail-flow-edit`, append an accepted
`example-108-ui-patch` candidate to a corpus copy, and replay the copy until
`flow-edit-applied true` and `patched-core-replayed true` are recorded. The
imported patch must then pass
`scripts/run_v03_ui_patch_runtime_state_check.py`, which records
`visual-regression-fingerprint-preserved true`,
`runtime-ui-state-check target-report`, and the replayed
`Ticket.reviewStatus` UI-state anchor. Accepted
`AgentTool` entries must also emit `agent-policy-review.txt`, which records the
multi-agent handoff roles, `ail-agent-contracts examples/agents` check, tool
permission and approval review, external-call review, secret-redaction review,
audit-trace review, human approval requirement, runtime evidence, and upstream
fingerprints. The report must summarize these files with
`agent-policy-review-fingerprint-*` lines and list each review in
`manifest.ail-examples.txt`. A reviewed AgentTool policy handoff can then be
captured with `scripts/run_v03_agent_policy_capture_plan.py` and imported with
`scripts/run_v03_agent_policy_import_demo.py`; that demo must preserve the
source entry, append an accepted `example-40-policy` candidate to a corpus copy,
and replay the copy until `policy-handoff-imported true` and
`policy-handoff-replayed true` are recorded. The repair tutorial is derived from
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
cargo test --test ail_toolchain cli_ail_story_native_target_executes_story_runtime_trace
cargo test --test ail_toolchain script_ail_interactive_manual_v03_authoring_gate_run_checks_succeeds
cargo test --test ail_toolchain script_ail_interactive_manual_systems_profile_run_checks_succeeds
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-v03-application-baseline
cargo run -- ail-conformance examples/secret_access.ail --artifact-dir /tmp/ail-v03-secret-access
cargo run -- ail-conformance examples/stateful_counter.ail --artifact-dir /tmp/ail-v03-stateful-counter
cargo run -- ail-conformance examples/incident_notifications.ail --artifact-dir /tmp/ail-v03-incident-notifications
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
- Application examples now have deterministic User Story mode evidence that
  starts from a support-ticket story, asks the toolchain agent to participate,
  writes requirements/spec/Core/bytecode/story manifests, compiles
  `CloseTicket` to a Linux x86_64 executable, and runs that binary to observe
  `ticket.status=Closed` plus `trace TicketClosed`. Story-amendment inputs now
  also write fingerprinted `story-amendment-comparison.txt` evidence that
  binds source story, normalized story, generated requirements, accepted spec,
  checked Core, and bytecode fingerprints to semantic-anchor preservation
  counts. The deterministic amendment branch now spans both support-ticket and
  incident-response packages, including incident escalation, notification audit
  entry, and public timeline subscriber anchors. The support-ticket package now
  has package-local accepted/rejected conformance fixtures surfaced by the
  `application-baseline` manual chapter, including secret leaks, missing
  traces, invalid fields, and failure-handling diagnostics. That chapter now
  also checks application-specific diagnostics for assignment without
  support-role validation, overdue scheduler mutation without a current-time
  requirement, and ticket status changes that drop customer-visible public
  updates. Incident-response now has package-local accepted/rejected fixtures
  for escalation, responder notification pager requirements, resolution after
  mitigation, postmortem after resolution, private-note leakage,
  commander-review policy, and route or dashboard permissions. Conformance now
  writes fingerprinted package-local repair tutorials for those incident
  failures so the Application baseline teaches diagnostics with source
  provenance, affected graph items, and repair suggestions. The next bar is
  checked repair proof chains and promotion of repaired incident variants back
  through the Application baseline.
- UI examples now emit deterministic visual review, accessibility review,
  workflow authoring artifacts, deterministic UI patch plans, a human-approved
  UI patch import demo, and a rejected accessibility diagnostic fixture that
  repairs to checked Core, verified bytecode, and Wasm target-contract
  evidence. Imported UI patches now also emit deterministic visual-regression
  fingerprint evidence and runtime UI-state checks that bind the patch to a
  promoted target report. The next bar is browser-backed visual regression
  evidence for UI surfaces and additional imported patch variants.
- AgentTool examples now emit deterministic policy review artifacts with
  multi-agent handoff roles, contract checks, permission and approval review,
  external-call review, secret-redaction review, audit-trace review, runtime
  evidence, a reusable Codex AgentTool policy reviewer contract and skill, a
  human-approved AgentTool policy import demo, and a deterministic
  role-separated multi-agent handoff witness. An opt-in live reviewer harness
  now records hosted request/response/content bundles for five reviewer roles
  and writes an offline review with `reviewer-envelope-valid-count`,
  `reviewer-envelope-invalid-count`, `evidence-bundle-present-count`,
  `reviewer-decision-accept-count`,
  `reviewer-decision-needs-repair-count`, and
  `reviewer-decision-reject-count`. The review is accepted only when every
  recorded request contains the deterministic evidence bundle and all reviewer
  roles return `decision: accept`; valid non-accept decisions become
  `review-result needs-repair` evidence. Incident notification support now has
  package-local accepted/rejected AgentTool conformance fixtures for approval
  rules, permission rules, secret output redaction, and
  `AIL-AGENT-AUDIT-001` provider-call audit evidence,
  `AIL-AGENT-FAILURE-001` provider failure declarations, and
  `AIL-AGENT-RECOVERY-001` provider recovery policy. The next bar is broader
  live reviewer coverage where accepted and rejected AgentTool policy handoffs
  are both produced by separate reviewer roles and promoted only after human
  approval, plus runtime evidence for bounded notification retry attempts.
- Compiler/self-hosting examples now include a deterministic `ail-bootstrap`
  manual check that composes the AIL-authored toolchain agent with the
  AIL-Meta `InferReadPermissions` compiler pass, verifies fixed-point pass
  output, records host-boundary and dependency reports, and runs native handoff
  checks. The next bar is pass-order diagnostics and multiple composed
  compiler-pass variants.
- Systems examples now include a deterministic manual chapter for
  `network_driver.ail` that runs package-local conformance, accepts scheduler
  and interrupt fixtures, rejects invalid interrupt/task contracts with stable
  diagnostics, compiles `NetworkPacketReceiver` to a Linux x86_64 ELF target,
  and runs the executable to observe resource, capability, effect, and trace
  output. The next bar is clearer unsupported-target migration guidance and a
  broader driver family with transmit and interrupt-handler runtime variants.
- Rejected examples now emit repair tutorials, corrected repair proof chains,
  semantic repair diffs, deterministic promotion review artifacts, plan-only
  repair promotion capture plans, and a human-approved batch import demo that
  can create the proposed accepted corpus entry while preserving the rejected
  evidence. The next bar is broader multi-diagnostic promotion coverage and
  reviewer-produced promotion decisions instead of one deterministic import
  script.
- Secret Access examples now include package-local accepted/rejected
  conformance fixtures for support-role guarded secret reads, redaction,
  denied-access traces, and declared `PermissionDenied` failures. The next bar
  is threat-model annotations and audit-trail artifacts that connect secret
  diagnostics to reviewer-facing security stories.
- Incident-response examples show that complex systems need richer story
  graphs across imported modules, UI surfaces, workflow transitions, target
  contracts, and regenerated story views.
- Recursive Turing Core examples now include checked recursive factorial
  replay and a package-local rejected fixture for `AIL-CONTROL-003` when a
  self-recursive function has no checker-visible base-case branch. The next
  bar is explicit stack-depth policy, non-decreasing recursive argument
  diagnostics, and richer termination proofs beyond simple base-case patterns.
- Stateful examples now include package-local accepted/rejected conformance
  fixtures for persistence guarantees, retry idempotency keys, shared-state
  locking or serialization, and replay recovery after failure. The next bar is
  migration fixtures, stale-state conflict detection, multi-action
  transactions, and durable runtime evidence beyond text-level policy checks.
