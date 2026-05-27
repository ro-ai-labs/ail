# AIL Examples

This directory is the authoritative home for AIL examples. An AIL example is
end-to-end by definition: it must be replayable from stored prompt transcript or
checked package source through checked AIL artifact, checked AIL-Core,
bytecode, VM trace, and either binary or target-contract evidence.

Each counted example must be replayable without live model access and must
record the full chain from stored prompt transcript to checked AIL artifact,
checked AIL-Core, bytecode, VM trace, and binary or target-contract evidence.
The v0.2 release gate requires at least 100 distinct semantic examples.

## Capability Ladder

The examples are not just a prompt regression set. Each catalog entry declares
`use-case`, `capability-level`, `capability-under-test`,
`program-scale`, `program-domain`, `module-count`, `spec-count`,
`story-count`, `interacts-with`, `user-story-id`, `user-story`,
`acceptance-criteria`, `story-evidence`, `story-journey`,
`story-roundtrip`, `distinctness-claim`, and `v0.3-signal` metadata so the
verifier can prove that the catalog spans useful work from low-level
development to high-level workflows.

The current ladder is:

1. Low-level development: `c_interop.ail`, `compiler_pass.ail`,
   `network_driver.ail`, and Darwin/Linux target-contract examples prove ABI,
   host-boundary, compiler-pass, system-effect, and backend-portability
   behavior.
2. Mid-level semantics: `ail_std_*.ail`, `support_composed.ail`,
   `runtime_generic.ail`, `secret_access.ail`, and `stateful_counter.ail`
   prove packages, standard library surfaces, generic values, permissions,
   traces, and deterministic state.
3. High-level workflows: `support_ticket.ail`, `refund_tool.ail`,
   `ui_workflow.ail`, `option_map.ail`, and `repeated_task.ail` prove
   application workflows, agent-tool safety, UI semantics, and scheduled
   work.
4. Rejected examples: malformed prompt envelopes, profile mismatch, missing
   traces, hallucinated capabilities, unsupported targets, invalid interop,
   permission/capability failures, and package-resolution failures prove that
   AIL teaches failure modes, not only successful paths.

## Learning Guides

The catalog is the authoritative replay manifest, but package-local READMEs are
the teaching path through repeated example families and support modules.
All 26 package directories include a README.md guide.
`support-packages.md` is the support-only manifest: any top-level
`examples/*.ail` package that is not counted in `examples.md` must be declared
there with `role: support-only`, a `used-by` relationship to counted examples
or executable toolchain evidence, and a concrete reason. This keeps helper
packages visible without making a separate non-end-to-end example category.

- `ail_toolchain_agent.ail/README.md`: AIL-authored agent participation in
  requirements, spec, Core, bytecode, target, manifest, and prompt-portability
  review.
- `support_ticket.ail/README.md`: Application workflow baseline, scheduler
  behavior, ticket state transitions, secret internal notes, native binary
  evidence, and rejected diagnostic gaps.
- `support_composed.ail/README.md`: package composition, explicit imports,
  shared user types, package-aware compile evidence, VM traces, and rejected
  package-graph gaps.
- `support_shared.ail/README.md`: reusable support-domain user and
  permission-denied declarations imported by composed support packages.
- `compiler_pass.ail/README.md`: Compiler profile pass semantics,
  `InferReadPermissions`, AIL-Core graph transforms, native pass evidence, and
  pass-composition/fixed-point bootstrap self-hosting evidence.
- `recursive_factorial.ail/README.md`: compact recursive function and
  arithmetic bytecode fixture for executable semantics.
- `network_driver.ail/README.md`: low-level System profile resources,
  ownership, borrowing, device effects, scheduler and interrupt fixtures,
  native target evidence, runtime traces, and rejected contract diagnostics.
- `c_interop.ail/README.md`: C ABI, pointer ownership, callbacks, layout,
  status-map failures, host imports, and invalid interop repair gaps.
- `darwin_linux_effect.ail/README.md`: target portability, Linux syscall
  effects, Darwin target contract evidence, and unsupported-target
  diagnostics.
- `ui_workflow.ail/README.md`: UI profile routes, forms, dashboards,
  accessibility, workflow ordering, Wasm target contracts, and rejected UI
  fixture gaps.
- `refund_tool.ail/README.md`: AgentTool safety, approvals, policy review,
  secret handling, and repair-tutorial gaps.
- `stateful_counter.ail/README.md`: deterministic state, package-local
  persistence, idempotency, locking, replay-after-failure diagnostics, and
  native artifact evidence.
- `repeated_task.ail/README.md`: scheduled-workflow behavior, repeated action
  lowering, maintenance-cycle trace evidence, temporal-policy fixtures, and
  scheduler-policy diagnostics.
- `runtime_generic.ail/README.md`: typed runtime priority flow,
  `TicketPrioritized` trace evidence, a rejected missing-trace fixture, target
  reports, and type-inference gaps.
- `secret_access.ail/README.md`: secret internal notes, support-role checks,
  redaction behavior, denied-access traces, package-local rejected fixtures,
  and audit-trail gaps.
- `incident_response.ail/README.md`: high-level multi-module incident response
  with identity, policy, notification, UI, workflow, target-contract, and story
  journey evidence.
- `incident_identity.ail/README.md`: responder, commander, service-owner,
  user, and team support types imported by incident response.
- `incident_policy.ail/README.md`: service-tier and escalation-policy support
  definitions for incident workflows.
- `incident_notifications.ail/README.md`: AgentTool notification support
  contract with pager-token secrecy, provider calls, approvals, audit traces,
  and package-local rejected fixtures for provider-call audit, failure, and
  recovery evidence.
- `missing_registry_import.ail/README.md`: rejected package-resolution fixture
  for unresolved registry import diagnostics.
- `ail_std_core.ail/README.md`: standard-library primitive contracts,
  `Identity.copy`, trace coverage, and accepted fixture evidence.
- `ail_std_collections.ail/README.md`: generic collection and result types,
  `Option.map`, standard-library replay entries, and rejected generic payload
  diagnostics.
- `option_map.ail/README.md`: focused `Option.map` transform, UI-tagged
  prompt surfaces, checked Core evidence, and semantic-anchor gaps for richer
  UI workflows.
- `ail_std_effects.ail/README.md`: declared resource effects, read/write and
  network effect traces, and host-effect repair gaps.
- `ail_std_security.ail/README.md`: `Secret<T>`, permission and capability
  requirements, redaction guarantees, a rejected reveal-without-redaction
  fixture, and secret-leakage repair gaps.
- `ail_std_runtime.ail/README.md`: runtime tasks, failure handling,
  dependency reports, capability grants, and missing-grant diagnostics.

For v0.3, every new example should either add a genuinely new use case or make
an existing use case more useful by testing a different prompt surface, target,
checker assertion, diagnostic, or human-review path. The `distinctness-claim`
field records that reason in the catalog instead of leaving it implicit.
`ail-examples` enforces a minimum usefulness bar: `use-case` and `v0.3-signal`
must be substantive, `v0.3-signal` must identify a needed or recommended next
step, and `distinctness-claim` must name the entry's `semantic-task` and
`capability-under-test` plus a concrete differentiating axis such as prompt
surface, target, checker assertion, diagnostic, story journey, artifact,
executor, or human-review path. Repeated `user-story-id` families are also
checked as families: when a family has at least five entries, it must cover at
least three prompt files and at least two story journeys. This keeps
prompt-surface matrices useful instead of letting one story earn many slots
through label-only variation.

Every required system prompt must also have at least one accepted example, not
only a rejected diagnostic entry or a prompt-file label. The replay report
emits `accepted-prompt-count` lines so reviewers can verify that every prompt
surface has generated an artifact that reached checked Core, bytecode, and
runtime or target evidence.

The release verifier also requires at least ten distinct `v0.3-signal`
learning signals, and the replay report emits `v03-signal-distinct-count` plus
one `v03-signal-count` line per signal. That makes the corpus usable as a
machine-readable backlog for the next language, prompt, checker, runtime,
target, and documentation improvements. The catalog field keeps the public
version spelling `v0.3-signal`; report labels use `v03-*` as artifact-safe
line keys. Replay also writes `v03-roadmap.txt`, a dedicated backlog artifact
that groups each signal by entry id, capability level, program domain, prompt
file, story journey, and checker result. The roadmap is fingerprinted and
listed in `manifest.ail-examples.txt` so agent reviewers can consume the
learning backlog without scraping the full replay report. Print the roadmap
directly with:

```sh
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-v03-roadmap --release-evidence
```

User stories are also first-class. A story can start the development flow
(`story-to-spec`), be regenerated from a checked spec or Core artifact
(`spec-to-story`), amend an existing specification (`story-amendment`), or
preserve a rejected diagnostic (`diagnostic-story`). Replay writes a
deterministic `user-story.txt` artifact for every entry so the story view can
be fingerprinted beside checked Core, bytecode, VM traces, and target reports.
The catalog's `story-evidence` value must name an artifact produced by replay:
`checked-core`, `bytecode`, `vm-trace`, `target-report`, or `diagnostics`.
Entries that claim evidence which was not generated are rejected before report
artifacts are written.
Rejected entries also get a deterministic `repair-tutorial.txt` artifact that
keeps the expected diagnostic, failure taxonomy, story file, prompt file,
package, diagnostic summary, and replay repair steps together for reviewer and
agent follow-up. Replay also materializes the deterministic repair proof chain:
`repair-candidate.ail-spec.md`, `repair-checked.ail-core.txt`,
`repair-artifact.ailbc.json`, and either `repair-vm-trace.txt` or
`repair-target-report.txt`, plus a `repair-diff.txt` review artifact that
links the rejected artifact to the repaired proof and carries story semantic
anchors forward. Each rejected entry also gets
`repair-promotion-review.txt`, a deterministic promotion decision artifact that
binds the rejected diagnostic, repair proof, repair diff, semantic anchors, and
human-approval requirement before any repaired artifact is proposed as a new
accepted corpus entry. That makes diagnostic examples end-to-end authoring
loops instead of prose-only negative cases.
Repair promotion follows the same corpus-copy rule as other reviewed imports.
Run `scripts/run_v03_repair_promotion_capture_plan.py` against the replay
artifacts to write `repair-promotion-capture-plan.json`, `.txt`, and
`.fingerprint.txt`. The plan requires human-approved request/response JSON for
`scripts/capture_example_batch.py` and records `preserve_rejected_entry: true`
so the rejected learning evidence remains in the corpus. Batch import can then
append `proposed_entry_id` from the plan by supplying a batch entry with
`source_entry_id`, `entry_id`, approved request/response JSON, and
`repair_promotion_capture_plan_json`. The deterministic wrapper
`scripts/run_v03_repair_promotion_import_demo.py` validates the plan
fingerprint, keeps the rejected source entry unchanged, writes new `requests/`,
`responses/`, and `stories/` files for the repaired accepted entry, and replays
the corpus copy before any generated corpus copy is committed.
User Story mode promotion follows the same corpus-copy rule. After
`scripts/run_v03_story_promotion_capture_plan.py` writes
`story-promotion-capture-plan.json`, a batch entry may supply
`story_promotion_capture_plan_json` with human-approved request/response JSON.
The importer validates the plan fingerprint, copies the reviewed story artifact
bundle under `story-artifacts/<entry-id>/`, writes fresh `requests/`,
`responses/`, and `stories/` files, writes
`human-approved-story-promotion-batch.fingerprint.txt`, records
`capture-plan story-promotion-capture-plan.json <fingerprint>`,
`promotion-decision accepted-for-promotion`,
`promotion-source human-approved-story-promotion-batch`, and
`batch-plan-fingerprint` in the
import report, and still requires offline `ail-examples` replay before any
generated corpus copy is committed.
Release story files must include `semantic-anchors` for the terms, actions,
modules, targets, or diagnostics that must survive story/spec/Core
round-trips. In `--release-evidence` mode, `ail-examples` rejects any catalog
entry whose story file lacks at least three anchors. Replay records
`semantic-anchor-story-count`, lists per-entry anchors in the report, and
embeds anchors into the generated story artifact. The report also records
`semantic-anchor-total-count`, `semantic-anchor-preserved-count`,
`semantic-anchor-missing-count`, and per-entry
`entry-semantic-anchor-preservation` lines so the `semantic-similar` story
round-trip claim is auditable from replay output. The report also records
`story-family-count` plus per-family `story-family` lines with entry,
prompt-file, and story-journey counts.
User Story mode runs with `story-journey: story-amendment` also write
`story-amendment-comparison.txt` and
`story-amendment-comparison.fingerprint.txt`, binding source story,
normalized story, generated requirements, accepted spec, checked Core, and
bytecode fingerprints to semantic-anchor preservation counts.

The deterministic seed baseline is generated by:

```sh
python3 scripts/generate_e2e_seed_corpus.py
```

The generated catalog identifies itself as `corpus-kind:
legacy-deterministic-seed` with `release-evidence: false`, and generated story
files include semantic anchors so local replay still exercises the current
story metadata shape. It remains intentionally non-release: `--release-evidence`
continues to reject the generated `capture-origin: deterministic-seed` entries.
Use `--output-dir <dir>` to write the seed into a scratch corpus instead of the
active `examples/` tree.

Live entries are then captured into a corpus copy and promoted only after
offline replay passes. One seed entry can be replaced with a stored live LLM
capture by writing a copy of the corpus:

```sh
python3 scripts/capture_example_transcripts.py \
  --base-corpus examples \
  --output-dir /tmp/ail-examples-live-corpus \
  --entry-id example-30 \
  --endpoint http://inteligentia-pro-1:8080/v1/chat/completions \
  --endpoint-label inteligentia-pro-1-qwen3.6-35b-chat \
  --executor-label unsloth-qwen3.6-35b-a3b-gguf-chat \
  --semantic-task support-ticket-live-capture-30 \
  --prompt-file examples/inputs/support-ticket-spec-draft.task.txt \
  --input-json-file examples/inputs/support-ticket-spec-draft.json
```

The capture script stores the raw completion request and raw JSON response in
`requests/` and `responses/`, marks the entry `capture-origin: live-llm`, and
leaves replay to `ail-examples`. For `/v1/chat/completions` endpoints the
request disables thinking with `chat_template_kwargs.enable_thinking=false`,
and replay extracts `choices[0].message.content` from the stored response.
Prompt surfaces with an input schema use `--input-json-file`; longer task
instructions use `--prompt-file`. The committed
`inputs/stdlib-collections-spec-draft.json`,
`inputs/stdlib-collections-spec-draft.task.txt`,
`inputs/support-ticket-spec-draft.json`,
`inputs/support-ticket-spec-draft.task.txt`,
`inputs/refund-tool-spec-draft.json`, and
`inputs/refund-tool-spec-draft.task.txt` fixtures are the first replay-clean
schema-shaped live capture inputs for `spec-draft.system.md` across the
standard-library, Application, and AgentTool surfaces. Replay still does not
call a live model.

Recorded Codex or skill-agent transcripts are promoted through a separate
offline import command:

```sh
python3 scripts/capture_codex_example_transcript.py \
  --base-corpus examples \
  --output-dir /tmp/ail-examples-live-codex-corpus \
  --entry-id example-99 \
  --executor-label codex-ail-spec-writer \
  --semantic-task support-ticket-live-codex-99 \
  --request-json-file /tmp/codex-request.json \
  --response-json-file /tmp/codex-response.json \
  --checker-result accepted
```

That importer stores the provided request and response JSON, marks the entry
`executor-family: codex-skill-agent` and `capture-origin: live-codex`, clears
HTTP endpoint metadata, and then relies on the same offline replay command for
spec -> Core -> bytecode -> VM and target evidence.

Batch promotion uses a JSON plan so multiple entries can be captured into the
same corpus copy without overwriting earlier replacements:

```sh
python3 scripts/capture_example_batch.py \
  --base-corpus examples \
  --output-dir /tmp/ail-examples-live-batch-corpus \
  --plan-json /tmp/ail-examples-capture-plan.json
```

Each plan entry uses `executor_family: llm-http` with endpoint, prompt, and
model labels, or `executor_family: codex-skill-agent` with recorded request and
response JSON files. Codex entries may also provide
`repair_promotion_capture_plan_json`, `story_promotion_capture_plan_json`, or
`ui_patch_capture_plan_json`, or `agent_policy_capture_plan_json` when the
batch appends a human-approved promotion candidate. The batch output still must
be replayed with `ail-examples` before promotion.

The generated files are committed so release verification does not depend on
live LLM access. The current corpus stores:

- `examples.md`: 123 manifest entries with prompt, executor, profile, surface,
  use-case, capability-level, capability-under-test, program scale, program
  domain, module/spec/story counts, interaction metadata, user-story metadata,
  story journey, distinctness, capture-origin, checker-result, target, and
  v0.3 learning metadata. One hundred thirteen entries are accepted
  prompt-to-artifact examples that replay through checked Core, bytecode, VM
  trace, and binary or target-contract evidence; nine entries are rejected
  diagnostic examples.
- `stories/`: one deterministic user-story view per catalog entry. The
  verifier rejects story files whose story, journey, evidence, domain,
  interaction, or count metadata drifts from the catalog.
- `story-artifacts/`: optional reviewed User Story mode artifact bundles copied
  into corpus promotion working trees by story-promotion import demos. Story
  amendment bundles include `story-amendment-comparison.txt` when the
  artifact was generated from a `story-journey: story-amendment` input.
- `requests/`: stored prompt request transcripts.
- `responses/`: stored model/agent response artifacts.
- `inputs/`: schema-shaped prompt inputs used for live capture attempts.
- `agents/`: Codex-style skill-agent executor contracts used by
  `live-codex` transcript imports.
- `v03-roadmap.txt`: generated by replay in the artifact bundle, not committed
  in place, and used as the machine-readable next-version backlog.
- `examples/<entry-id>/repair-tutorial.txt`: generated by replay for each
  rejected entry and fingerprinted beside `diagnostics.txt` so diagnostic
  failures are teachable repair paths, not only failure records.
- `examples/<entry-id>/repair-candidate.ail-spec.md`,
  `repair-checked.ail-core.txt`, `repair-artifact.ailbc.json`, and repair
  runtime or target evidence: generated by replay for each rejected entry to
  prove the diagnostic-to-fix loop reaches checked Core and verified bytecode.
- `examples/<entry-id>/repair-diff.txt`: generated by replay for each rejected
  entry to compare rejected and repaired fingerprints, mark the expected
  diagnostic as removed, and preserve story semantic anchors.
- `examples/<entry-id>/repair-promotion-review.txt`: generated by replay for
  each rejected entry to record `accepted-for-promotion`,
  `human-approval-required true`, the proposed accepted entry id, and
  fingerprints for the rejected diagnostic, repair proof, repair evidence, and
  repair diff.
- `examples/<entry-id>/ui-review.txt`: generated by replay for each accepted
  UI workflow or UI-surface entry to record deterministic visual review,
  accessibility review, workflow authoring evidence, runtime evidence,
  semantic-anchor preservation, and upstream fingerprints.
- `examples/<entry-id>/ui-review-patch.txt`: generated beside each UI review
  to record a proposed-only `ail-flow-edit` patch plan, human approval
  requirement, patch scope, and upstream UI review fingerprint.
- `ui-patch-capture-plan.json`, `ui-patch-import-demo-report.txt`, and
  `ui-patch-runtime-state-check-report.txt`: generated in scratch artifact
  directories by `scripts/run_v03_ui_patch_capture_plan.py`,
  `scripts/run_v03_ui_patch_import_demo.py`, and
  `scripts/run_v03_ui_patch_runtime_state_check.py` to validate a
  human-approved `ail-flow-edit`, append `example-108-ui-patch` to a corpus
  copy, replay that copy through checked Core, bytecode, VM trace, and
  target-contract evidence, then bind visual review fingerprints to runtime
  UI-state anchors for the imported patch.
- `examples/<entry-id>/agent-policy-review.txt`: generated for each accepted
  AgentTool entry to record deterministic multi-agent handoff review, the
  `ail-agent-contracts examples/agents` check, permission and approval review,
  external-call review, secret-redaction review, audit-trace review, human
  approval requirement, runtime evidence, and upstream fingerprints. It is
  fingerprinted beside the review as `agent-policy-review.fingerprint.txt`.
- `agent-policy-capture-plan.json` and
  `agent-policy-import-demo-report.txt`: generated in scratch artifact
  directories by `scripts/run_v03_agent_policy_capture_plan.py` and
  `scripts/run_v03_agent_policy_import_demo.py` to validate a human-approved
  multi-agent policy handoff, append `example-40-policy` to a corpus copy, and
  replay that copy through checked Core, bytecode, VM trace, and native target
  evidence.
- `agent-policy-multi-agent-handoff-report.txt`: generated by
  `scripts/run_v03_agent_policy_multi_agent_handoff.py` after the import demo
  to validate `ail-agent-contracts examples/agents`, the capture plan, the
  import report, and the promoted checked Core trace as a role-separated
  deterministic handoff witness.
- `agent-policy-live-review-report.txt` and
  `agent-policy-live-review-review.txt`: generated by
  `scripts/run_v03_agent_policy_live_reviewer_harness.py` when hosted
  AgentTool reviewer roles execute against the policy handoff evidence and
  the recorded request/response/content bundle is reviewed offline. Offline
  review requires every hosted reviewer request to carry a complete
  deterministic evidence bundle with artifact fingerprints and bounded content
  excerpts; filename-only reviewer prompts are rejected.
- `examples/agents/codex-ail-agent-policy-reviewer.md` and
  `examples/agents/skills/ail-agent-policy-reviewer/SKILL.md`: reusable Codex
  reviewer contract and skill for checking AgentTool policy handoff evidence
  before any human-approved policy trace amendment is proposed for corpus-copy
  promotion.

Catalog paths are closed over this tree. The `request-file`, `response-file`,
and `story-file` values must stay inside the catalog directory, and `package`
values must stay inside repository `./examples`. Absolute paths and `..` path
escapes are rejected before replay so example evidence remains portable and
human-reviewable.

This is checked release evidence with four replay-clean live LLM
captures and one hundred eighteen replay-clean live Codex skill-agent captures. The
current corpus marks zero entries `capture-origin: deterministic-seed`, four
entries `capture-origin: live-llm`, and one hundred eighteen `codex-ail-spec-writer`
entries `capture-origin: live-codex`. The replay report exposes
capability-level counts, program-scale counts, story-journey counts,
program-domain counts, story-evidence counts, capture-origin counts, response,
extracted-artifact, checked Core, bytecode, VM trace, native, target-report,
UI review, UI review patch, agent policy review, diagnostics, repair-tutorial,
repair-candidate, repair-checked-core, repair-bytecode, repair-vm-trace,
repair-target-report, repair-diff, and repair-promotion-review fingerprint
reuse. Response, extracted-artifact, and target-report duplicate counts must
remain zero before claiming the v0.2 prompt-to-artifact release gate. Accepted UI workflow and
UI-surface replay must emit `ui-review-fingerprint-*` and
`ui-review-patch-fingerprint-*` report lines plus `ui-review` and
`ui-review-patch` manifest entries before claiming the visual/accessibility
patch-planning path. The human-approved import path is checked by
`scripts/run_v03_ui_patch_capture_plan.py`,
`scripts/run_v03_ui_patch_import_demo.py`, and
`scripts/run_v03_ui_patch_runtime_state_check.py`; the reports must include
`source-preserved true`, `proposed-accepted true`, `flow-edit-applied true`,
`patched-core-replayed true`, `visual-regression-fingerprint-preserved true`,
`runtime-ui-state-check target-report`, and the `Ticket.reviewStatus` runtime
UI-state anchor. Accepted AgentTool replay must emit
`agent-policy-review-fingerprint-*` report lines plus `agent-policy-review`
manifest entries before claiming the multi-agent policy handoff review path.
The human-approved AgentTool import path is checked by
`scripts/run_v03_agent_policy_capture_plan.py` followed by
`scripts/run_v03_agent_policy_import_demo.py`; the import report must include
`source-preserved true`, `proposed-accepted true`,
`policy-handoff-imported true`, and `policy-handoff-replayed true`.
Rejected example replay
includes stored prompt-envelope diagnostics for malformed model outputs and
profile mismatch checker-handoff diagnostics, plus checked AIL-Spec
diagnostics for missing trace coverage and hallucinated capability or
permission references; broader backend rejected-output replay records
unsupported target diagnostics from the Darwin contract path; invalid interop
replay records nullable-to-non-null FFI diagnostics from the C interop checker;
permission/capability replay records missing system capability diagnostics from
the System profile checker; package resolution replay records unresolved
registry-import diagnostics from the package loader; UI accessibility replay
records inaccessible form-validation diagnostics from the UI checker. Each
rejected replay now
also writes a repair tutorial and a checked repair proof chain that turns those
diagnostics into a corrected spec, checked Core, verified bytecode, and runtime
or target evidence. The repair diff ties those artifacts together and records
semantic-anchor preservation. The promotion review records whether the repair
is accepted for human-approved promotion and lists every upstream fingerprint
needed to audit that decision. All repair artifacts are listed in
`examples-report.txt` and `manifest.ail-examples.txt`. Broader rejected
taxonomy coverage is still tracked by the v0.2 completion gate. The artifact bundle also writes
`model-executor-manifest.txt` and
`model-executor-manifest.fingerprint.txt`, which enumerate executor families,
executor labels, endpoint labels, capture origins, executor/origin pairs,
executor/endpoint pairs, and per-entry semantic task provenance.

The seed includes four real `UI` profile replays through `ui_workflow.ail`,
which lower UI route, form, dashboard, and workflow semantics into checked Core,
bytecode, VM trace, and Wasm target-contract artifacts across the core-to-spec,
spec-draft, and requirements prompt surfaces, plus a rejected accessibility
diagnostic that repairs to the same checked target path. All accepted UI workflow entries
and complex application entries tagged with the `ui` surface also produce
`ui-review.txt` and `ui-review-patch.txt` so visual review, accessibility
review, workflow authoring, proposed patch planning, and runtime handoff are
fingerprinted in the replay bundle. Other UI-tagged seed
entries still use surface metadata to keep threshold checks active. Package-import seed
entries replay through package-aware import resolution and compile the composed
support package through checked Core, bytecode, and VM trace artifacts.

The AgentTool seed entries replay the refund tool across the prompt pack and
now produce `agent-policy-review.txt` artifacts. These reviews bind the
Codex/LLM executor label, prompt file, named payment provider, policy engine,
audit log interactions, human approval requirement, and runtime evidence into a
fingerprinted artifact. The policy import demo then turns example-40 review
evidence into a human-approved `example-40-policy` corpus-copy entry with
`PolicyHandoffApprovedScenario40` replayed through checked Core, bytecode,
native target evidence, and VM trace.

The corpus also includes `incident_response.ail`, a multi-module application
that imports identity, policy, and notification support packages and exercises
incident declaration, escalation, responder notification, dashboards, command
routes, lifecycle workflow, VM traces, Wasm contracts, and Darwin contract
evidence across five user-story families.

Replay with:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-examples-seed-artifacts
```

Final release replay adds the strict evidence switch:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-examples-release-artifacts --release-evidence
```

That mode requires all counted entries to come from stored live captures and
requires both `live-llm` and `live-codex` origins. The current corpus is
expected to pass that stricter command with zero deterministic seed entries.
