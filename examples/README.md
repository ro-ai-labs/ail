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
the teaching path through repeated example families:

- `support_ticket.ail/README.md`: Application workflow baseline, scheduler
  behavior, ticket state transitions, secret internal notes, native binary
  evidence, and rejected diagnostic gaps.
- `support_composed.ail/README.md`: package composition, explicit imports,
  shared user types, package-aware compile evidence, VM traces, and rejected
  package-graph gaps.
- `compiler_pass.ail/README.md`: Compiler profile pass semantics,
  `InferReadPermissions`, AIL-Core graph transforms, native pass evidence, and
  fixed-point self-hosting gaps.
- `network_driver.ail/README.md`: low-level System profile resources,
  ownership, borrowing, device effects, and missing-capability diagnostics.
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
- `stateful_counter.ail/README.md`: deterministic state, persistence,
  idempotency, locking, replay after failure, and native artifact evidence.
- `incident_response.ail/README.md`: high-level multi-module incident response
  with identity, policy, notification, UI, workflow, target-contract, and story
  journey evidence.
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
  requirements, redaction guarantees, and secret-leakage repair gaps.
- `ail_std_runtime.ail/README.md`: runtime tasks, failure handling,
  dependency reports, capability grants, and missing-grant diagnostics.

For v0.3, every new example should either add a genuinely new use case or make
an existing use case more useful by testing a different prompt surface, target,
checker assertion, diagnostic, or human-review path. The `distinctness-claim`
field records that reason in the catalog instead of leaving it implicit.
`ail-examples` enforces a minimum usefulness bar: `use-case` and `v0.3-signal`
must be substantive, `v0.3-signal` must identify a needed or recommended next
step, and `distinctness-claim` must name the entry's `semantic-task` and
`capability-under-test`.

User stories are also first-class. A story can start the development flow
(`story-to-spec`), be regenerated from a checked spec or Core artifact
(`spec-to-story`), amend an existing specification (`story-amendment`), or
preserve a rejected diagnostic (`diagnostic-story`). Replay writes a
deterministic `user-story.txt` artifact for every entry so the story view can
be fingerprinted beside checked Core, bytecode, VM traces, and target reports.
Story files can include `semantic-anchors` for the terms, actions, modules,
targets, or diagnostics that must survive story/spec/Core round-trips; replay
records `semantic-anchor-story-count`, lists per-entry anchors in the report,
and embeds anchors into the generated story artifact.

The deterministic seed baseline is generated by:

```sh
python3 scripts/generate_e2e_seed_corpus.py
```

Live entries are then captured into a corpus copy and promoted only after
offline replay passes. One seed entry can be replaced with a stored live LLM
capture by writing a copy of the corpus:

```sh
python3 scripts/capture_e2e_transcripts.py \
  --base-corpus examples \
  --output-dir /tmp/ail-e2e-live-corpus \
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
python3 scripts/capture_codex_e2e_transcript.py \
  --base-corpus examples \
  --output-dir /tmp/ail-e2e-live-codex-corpus \
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
python3 scripts/capture_e2e_batch.py \
  --base-corpus examples \
  --output-dir /tmp/ail-e2e-live-batch-corpus \
  --plan-json /tmp/ail-e2e-capture-plan.json
```

Each plan entry uses `executor_family: llm-http` with endpoint, prompt, and
model labels, or `executor_family: codex-skill-agent` with recorded request and
response JSON files. The batch output still must be replayed with
`ail-examples` before promotion.

The generated files are committed so release verification does not depend on
live LLM access. The current corpus stores:

- `examples.md`: 116 manifest entries with prompt, executor, profile, surface,
  use-case, capability-level, capability-under-test, program scale, program
  domain, module/spec/story counts, interaction metadata, user-story metadata,
  story journey, distinctness, capture-origin, checker-result, target, and
  v0.3 learning metadata. One hundred eight entries are accepted
  prompt-to-artifact examples that replay through checked Core, bytecode, VM
  trace, and binary or target-contract evidence; eight entries are rejected
  diagnostic examples.
- `stories/`: one deterministic user-story view per catalog entry. The
  verifier rejects story files whose story, journey, evidence, domain,
  interaction, or count metadata drifts from the catalog.
- `requests/`: stored prompt request transcripts.
- `responses/`: stored model/agent response artifacts.
- `inputs/`: schema-shaped prompt inputs used for live capture attempts.
- `agents/`: Codex-style skill-agent executor contracts used by
  `live-codex` transcript imports.

This is checked release evidence with four replay-clean live LLM
captures and one hundred twelve replay-clean live Codex skill-agent captures. The
current corpus marks zero entries `capture-origin: deterministic-seed`, four
entries `capture-origin: live-llm`, and one hundred twelve `codex-ail-spec-writer`
entries `capture-origin: live-codex`. The replay report exposes
capability-level counts, program-scale counts, story-journey counts,
program-domain counts, story-evidence counts, capture-origin counts, response,
extracted-artifact, checked Core, bytecode, VM trace, native, target-report,
and diagnostics fingerprint reuse. Response, extracted-artifact, and
target-report duplicate counts must remain zero before claiming the v0.2
prompt-to-artifact release gate. Rejected example replay includes stored
prompt-envelope diagnostics for malformed model outputs and profile mismatch
checker-handoff diagnostics, plus checked AIL-Spec diagnostics for missing
trace coverage and hallucinated capability or permission references; broader
backend rejected-output replay records unsupported target diagnostics from the
Darwin contract path; invalid interop replay records nullable-to-non-null FFI
diagnostics from the C interop checker; permission/capability replay records
missing system capability diagnostics from the System profile checker; package
resolution replay records unresolved registry-import diagnostics from the
package loader. Broader rejected taxonomy coverage is still tracked by the v0.2
completion gate. The artifact bundle also writes
`model-executor-manifest.txt` and
`model-executor-manifest.fingerprint.txt`, which enumerate executor families,
executor labels, endpoint labels, capture origins, executor/origin pairs,
executor/endpoint pairs, and per-entry semantic task provenance.

The seed includes three real `UI` profile replays through `ui_workflow.ail`,
which lower UI route, form, dashboard, and workflow semantics into checked Core,
bytecode, VM trace, and Wasm target-contract artifacts across the core-to-spec,
spec-draft, and requirements prompt surfaces. Other UI-tagged seed entries still
use surface metadata to keep threshold checks active. Package-import seed
entries replay through package-aware import resolution and compile the composed
support package through checked Core, bytecode, and VM trace artifacts.

The corpus also includes `incident_response.ail`, a multi-module application
that imports identity, policy, and notification support packages and exercises
incident declaration, escalation, responder notification, dashboards, command
routes, lifecycle workflow, VM traces, Wasm contracts, and Darwin contract
evidence across five user-story families.

Replay with:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-e2e-seed-artifacts
```

Final release replay adds the strict evidence switch:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-e2e-release-artifacts --release-evidence
```

That mode requires all counted entries to come from stored live captures and
requires both `live-llm` and `live-codex` origins. The current corpus is
expected to pass that stricter command with zero deterministic seed entries.
