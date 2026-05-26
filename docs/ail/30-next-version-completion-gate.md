# AIL Next Version Completion Gate

## Purpose

This gate defines the next complete AIL version after v0.1.

AIL v0.1 proved the first complete programming loop:

```text
English intent
  -> checked requirements
  -> checked AIL-Spec
  -> checked AIL-Core
  -> AIL-Flow review
  -> bytecode
  -> VM/native/Wasm-contract artifacts
  -> round-trip back to human-reviewable AIL-Spec
```

AIL v0.2 is complete when AIL can prove a larger portability boundary:
packages, capability grants, host imports, UI semantics, prompt-portability
corpus evidence, and one additional backend contract are all checked,
manifested, and reviewable as AIL artifacts.

This gate also requires broad prompt-to-artifact evidence. AIL v0.2 is not
complete when one curated prompt works. It is complete only when the versioned
system prompt pack has produced at least 100 replayable end-to-end examples
that pass through checked AIL artifacts, lowering, compilation, and binary or
target-contract evidence.

The version name for this gate is:

```text
AIL v0.2 package and host-boundary portability
```

## Selection Rationale

The post-v0.1 work naturally falls into three possible next-version shapes:

- IDE-first: build a production graphical AIL editor.
- Backend-first: add another native executable backend.
- Package and host-boundary first: make imports, capability grants, C/host
  bindings, UI declarations, Wasm contracts, and prompt portability verifiable.

AIL v0.2 chooses package and host-boundary first because v0.1 already has a
working authoring-to-native loop, while the strongest remaining language risks
are at trust boundaries: imported packages, imported host capabilities, UI
actions, external calls, and model portability. These risks are also explicitly
represented in the v0.1 non-goals and in the desired-outcome matrix.

## Completion Definition

AIL v0.2 is complete when this chain is reproducible from a clean checkout:

```text
AIL package with imports and capability grants
  -> checked dependency graph and package lock/report
  -> checked standard-library package use
  -> checked C or host binding with ABI/layout/ownership metadata
  -> checked UI route/form/workflow/accessibility semantics
  -> checked AIL-Core and AIL-Flow projections
  -> AIL-Bytecode with host-import metadata
  -> VM trace or Wasm host-contract trace for the host boundary
  -> Linux native artifact for supported local effects
  -> additional OS target contract report
  -> replayable 100-example prompt-to-artifact corpus
  -> release evidence bundle with manifests and fingerprints
```

Every arrow must have at least one of:

- parser or checker gate
- stable diagnostic or rejected fixture
- deterministic fingerprint
- manifest entry
- conformance fixture
- runtime trace or host-contract trace
- AIL-authored agent verification action
- replayed model or Codex-agent transcript checked against deterministic
  artifacts

## Required Gates

### 1. Package Resolution And Capability Grants

Requirement: Imported packages are resolved through explicit package metadata,
versions, hashes, aliases, and capability grants. Imported behavior must not
receive ambient authority.

Evidence:

- Local exact imports still work and version mismatches still fail.
- Compatible local version ranges are resolved deterministically.
- Registry package names are resolved through an explicit registry index that
  records package identity, version, and source path.
- Duplicate aliases, unresolved imports, unbounded major versions, and
  conflicting package names are rejected.
- Capability grants are checked before imported effects can lower.
- Package lock or dependency report records resolved package name, version,
  source path or registry identity, package hash, granted capabilities, and
  imported effect classes.
- Build, lower, conformance, and compile manifests include dependency report
  fingerprints when imports are present.

Minimum proof commands:

```bash
cargo test ail_package_loader_accepts_versioned_imports_and_rejects_mismatches
cargo test ail_package_loader_rejects_duplicate_import_aliases
cargo test cli_ail_package_resolves_compatible_local_import_ranges
cargo test cli_ail_package_rejects_unbounded_major_import_ranges
cargo test cli_ail_package_resolves_registry_import_identity_from_index
cargo test cli_ail_package_rejects_unknown_registry_import
cargo test cli_ail_package_rejects_imported_effect_without_capability_grant
cargo test cli_ail_build_records_dependency_report_for_imported_package_graph
```

### 2. Standard Library Packages

Requirement: The initial standard library is represented as real AIL packages,
not hidden compiler behavior.

Required standard packages:

- `ail.std.core`
- `ail.std.collections`
- `ail.std.effects`
- `ail.std.security`
- `ail.std.runtime`

Evidence:

- Each required standard package has `ail-package.md`, canonical AIL-Spec,
  checked AIL-Core, public API declarations, target-support metadata, and
  conformance fixtures. Current evidence: `examples/ail_std_core.ail`,
  `examples/ail_std_collections.ail`, `examples/ail_std_effects.ail`,
  `examples/ail_std_security.ail`, and `examples/ail_std_runtime.ail` are real
  package directories with `conformance: v0.2`,
  `schema-version: ail-core.schema.v0`, `target-support:
  ail-core.schema.v0=supported`, and accepted conformance fixtures.
- `Option<T>`, `Result<T,E>`, `List<T>`, `Map<K,V>`, and `Set<T>` have checked
  type/variant surfaces in `ail.std.collections`.
- `Option.map` or an equivalent collection transform has bytecode and VM trace
  evidence.
- `Secret<T>`, permission requirements, capability requirements, trace
  declarations, and failure declarations lower into checked Core through the
  standard security and runtime packages.
- Build/conformance/compile artifact manifests include dependency reports when
  standard packages are imported through local package dependencies.
- No compiler-hidden stdlib declaration is accepted without package source in
  the required package fixture set.
- Rejected fixtures cover unresolved standard imports, version conflicts,
  missing capability grants, and invalid generic use. Current evidence includes
  `invalid-generic-variant-payload.ail-spec.md` in `ail.std.collections` and
  the rejected package fixture `missing-capability-grant.ail` in
  `ail.std.runtime`.

Minimum proof commands:

```bash
cargo test cli_ail_stdlib_packages_have_checked_package_artifacts
cargo test cli_ail_stdlib_import_records_dependency_report
cargo test ail_standard_library_option_type_parses_into_core
cargo test ail_standard_library_option_map_executes_collection_transform_bytecode
cargo test cli_ail_std_rejects_invalid_generic_variant_payload
cargo test cli_ail_std_rejects_missing_capability_grant
```

### 3. Host Boundary And C Interop Contract

Requirement: C and host interop declarations are checked as safe AIL semantics
before any backend can expose them to a host.

Evidence:

- C function imports lower into `ExternalBinding` Core nodes with inputs,
  outputs, status maps, failures, capabilities, traces, ABI, and library
  metadata. Current package evidence lives in `examples/c_interop.ail`.
- Struct layout fixtures record size, alignment, field offsets, and target ABI
  through `struct-layout-minimal.ail-spec.md`.
- Callback fixtures record lifetime, noescape ownership, allowed effects, and
  failure propagation through the `libc.qsort` binding.
- Ownership-transfer fixtures require release semantics through
  `owned-pointer-release-minimal.ail-spec.md`.
- Rejected fixtures cover borrowed pointer escape, nullable-to-non-null
  mismatch, mutable pointer aliasing, missing status map, missing trace, and
  secret leakage across a foreign boundary.
- Wasm contract reports enumerate all host imports and preserve trace
  requirements.
- Contract evidence records foreign-call traces for deterministic host-boundary
  fixtures.

Minimum proof commands:

```bash
cargo test ail_c_interop_import_parses_into_external_binding_core
cargo test cli_ail_compile_wasm_contract_enumerates_external_bindings
cargo test cli_ail_ffi_checks_struct_layout_fixture
cargo test cli_ail_ffi_checks_callback_lifetime_fixture
cargo test cli_ail_ffi_accepts_owned_pointer_release_fixture
cargo test cli_ail_ffi_rejects_borrowed_pointer_escape
cargo test cli_ail_ffi_rejects_owned_pointer_without_release
cargo test cli_ail_ffi_rejects_nullable_to_non_null_mismatch
cargo test cli_ail_ffi_rejects_mutable_pointer_aliasing
cargo test cli_ail_ffi_rejects_secret_leakage
cargo test cli_ail_ffi_rejects_missing_status_map
cargo test cli_ail_ffi_records_foreign_call_trace_contract
```

### 4. UI Profile Semantics

Requirement: UI declarations are semantic AIL artifacts tied to actions,
permissions, failures, accessibility, and traces.

Evidence:

- Route declarations still lower into checked AIL-Core.
- Form declarations bind fields, validation rules, failure views, and target
  actions.
- Dashboard/view declarations bind reads, permissions, filters, and trace
  events.
- Multi-step workflow declarations enforce step ordering and blocked actions.
- Accessibility declarations include accessible names, focus order, keyboard
  equivalents, and error announcements where applicable.
- AIL-Flow projects route maps, form blocks, workflow steps, permission
  highlights, failure states, and accessibility review blocks.
- Rejected fixtures cover action reachability, dashboard permission parity,
  destructive action confirmation, inaccessible error text, and workflow step
  ordering violations.

Minimum proof commands:

```bash
cargo test ail_ui_route_surface_parses_into_core
cargo test cli_ail_ui_form_calls_checked_action
cargo test cli_ail_ui_dashboard_requires_matching_permission
cargo test cli_ail_ui_workflow_blocks_out_of_order_provider_call
cargo test cli_ail_ui_rejects_unreachable_form_action
cargo test cli_ail_ui_rejects_dashboard_without_permission
cargo test cli_ail_ui_rejects_inaccessible_error_text
cargo test cli_ail_ui_rejects_destructive_action_without_confirmation
cargo test cli_ail_ui_accessibility_trace_records_field_error_announcement
cargo test cli_ail_flow_projects_ui_profile_blocks
```

### 5. Backend Portability Contract

Requirement: AIL has one executable Linux native backend, one Wasm host-import
contract backend, and one additional OS target contract that fails safely when
unsupported effects are requested.

Evidence:

- Linux `linux-x86_64-elf` artifacts still execute supported local effects.
- Wasm contract reports still enumerate host imports and reject stale native
  artifacts in Wasm-only directories.
- `aarch64-apple-darwin-libsystem-macho` contract reports record target
  identity, executable format, host boundary, capability mapping, dependency
  report, and unsupported-effect diagnostics.
- Additional OS target contract artifacts have manifests and fingerprints.
- Backend reports preserve package hash, AIL-Core hash, bytecode hash, target
  identity, trace mapping status, host boundary, and dependency metadata.

Minimum proof commands:

```bash
cargo test cli_ail_compile_emits_runnable_linux_x86_64_elf_executable
cargo test cli_ail_compile_package_writes_wasm_contract_artifacts
cargo test cli_ail_compile_wasm_contract_bundle_rejects_stale_native_bundle_artifacts
cargo test cli_ail_compile_writes_darwin_macho_contract_artifacts
cargo test cli_ail_compile_darwin_contract_rejects_linux_only_syscall_effect
```

### 6. End-To-End Prompt Corpus And Model Portability

Requirement: Prompt-pack behavior is measured with stored accepted and rejected
model outputs rather than trusted by assumption, and the accepted prompt outputs
prove the full toolchain path from prompt to checked AIL artifact to IR to
compiled artifact.

Evidence:

- The release corpus contains at least 100 end-to-end examples. An end-to-end
  example starts from a user intent, a package/profile context, a versioned
  system prompt, and one executor output, then records checked requirements or
  checked AIL-Spec, checked AIL-Core, bytecode, a VM trace, and either a Linux
  native artifact or a target contract artifact.
- The 100-example minimum counts semantic examples, not repeated labels for the
  same stored output. A semantic example is distinct only when it changes the
  user intent, profile, package surface, required feature, target contract, or
  expected diagnostic.
- The corpus spans every required prompt-pack surface: interview,
  requirements, spec draft, core draft, repair, core-to-spec,
  core-to-summary, flow patch, trace-debug, diagnostic repair, and interop.
- The corpus spans Application, AgentTool, Compiler, System, standard-library,
  package-import, UI, C/host interop, Wasm contract, Darwin contract, and
  prompt-failure cases.
- Each accepted end-to-end example records the prompt file and fingerprint,
  prompt-pack version, executor family, executor label, capture origin
  (`deterministic-seed`, `live-llm`, or `live-codex`), raw request fingerprint,
  raw response fingerprint, extracted artifact fingerprint, checked Core
  fingerprint, bytecode fingerprint, manifest fingerprint, and binary or
  target-contract fingerprint.
- Each accepted executable example runs through VM verification. Each accepted
  native Linux example writes an executable artifact and target report. Each
  accepted host-boundary example writes Wasm or Darwin contract reports instead
  of pretending unsupported effects are executable.
- Rejected outputs demonstrate prompt-envelope, profile mismatch,
  hallucinated capability, missing trace, semantic drift, unsupported target,
  invalid interop, permission/capability, and package resolution diagnostics.
- At least two executor families are represented in release evidence:
  `llm-http` and `codex-skill-agent`. At least two LLM endpoint or model labels
  are represented for the same semantic task family.
- Live model calls are not release proof by themselves. Release proof is the
  replayable stored transcript and artifact bundle, and the replay verifier must
  not call a live model endpoint.
- Portability reports include model or executor label, endpoint label where
  applicable, prompt fingerprint, request fingerprint, response fingerprint,
  artifact fingerprint, checker result, compile result, target result, capture
  origin, and failure taxonomy.
- The example replay report includes duplicate-fingerprint counts for request,
  response, extracted artifact, checked Core, bytecode, VM trace, native,
  target-report, diagnostics artifacts, and capture-origin buckets. Final
  release evidence requires zero duplicate response, extracted-artifact, and
  target-report entries, plus broad `live-llm` and `live-codex` capture-origin
  coverage unless this file names a specific shared artifact as intentional
  non-release scaffolding.
- The example artifact bundle includes `model-executor-manifest.txt` and
  `model-executor-manifest.fingerprint.txt`, covering executor families,
  executor labels, endpoint labels, capture origins, executor/origin pairs,
  executor/endpoint pairs, and per-entry semantic task provenance.

Minimum proof commands:

```bash
cargo test cli_ail_build_agent_compares_prompt_portability_before_compile
cargo test cli_ail_prompt_corpus_accepts_checked_outputs
cargo test cli_ail_prompt_corpus_rejects_semantic_drift_outputs
cargo test cli_ail_prompt_corpus_writes_portability_report
cargo test cli_ail_e2e_corpus_replays_checked_live_release_corpus
cargo test cli_ail_e2e_corpus_requires_100_distinct_semantic_examples
cargo test cli_ail_e2e_corpus_requires_full_prompt_pack_coverage
cargo test cli_ail_e2e_corpus_requires_llm_and_codex_executor_families
cargo test cli_ail_e2e_corpus_requires_llm_endpoint_diversity
cargo test cli_ail_e2e_corpus_requires_target_thresholds
cargo test cli_ail_e2e_corpus_replays_imported_package_specs
cargo test cli_ail_e2e_corpus_replays_ui_profile_specs
cargo test cli_ail_e2e_corpus_replays_rejected_prompt_failures
cargo test cli_ail_e2e_corpus_release_evidence_rejects_deterministic_seed_corpus
cargo test cli_ail_e2e_corpus_release_evidence_accepts_live_corpus
cargo test cli_ail_e2e_corpus_release_evidence_requires_live_codex_for_codex_executor
cargo run -- ail-examples examples --artifact-dir /tmp/ail-v02-examples --release-evidence
```

### 7. Diagnostics And Governance Coverage

Requirement: Every v0.2 checker rule has a stable diagnostic or documented
verifier string with at least one accepted fixture and one rejected fixture.

Evidence:

- Diagnostic catalog includes v0.2 package, standard library, FFI, UI, backend,
  and prompt-corpus diagnostics.
- Desired outcome traceability rows affected by v0.2 name concrete schema,
  examples, diagnostics, and conformance boundaries.
- Evolution notes identify migration behavior for v0.1 packages.
- Placeholder scans over active docs return no unresolved placeholders.

Minimum proof commands:

```bash
cargo test ail_core_reports_stable_invalid_fixture_diagnostics
cargo test cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test cli_ail_conformance_checks_v02_package_host_boundary_fixtures
rg -n "TB[D]|TO[D]O|FIXM[E]|implement late[r]|fill in detail[s]" docs/ail README.md docs/README.md
```

For the placeholder scan, no matches is the expected result.

## Release Audit Command Set

Before claiming AIL v0.2 complete, run this command set from the repository
root and preserve the output as release evidence:

```bash
python3 scripts/run_v02_release_audit.py --bundle-root /tmp/ail-v02-release-evidence
```

The runner writes `release-audit-manifest.txt`,
`release-audit-manifest.fingerprint.txt`, per-command logs, and all
artifact directories under the bundle root. It fails if any command fails or if
an artifact-producing command does not write its expected manifest and
`manifest.fingerprint.txt`.

The runner expands to this command set:

```bash
cargo fmt --check
git diff --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-support
cargo run -- ail-conformance examples/refund_tool.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-refund
cargo run -- ail-conformance examples/compiler_pass.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-compiler
cargo run -- ail-conformance examples/network_driver.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-system
cargo run -- ail-conformance examples/ail_std_collections.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-std-collections
cargo run -- ail-conformance examples/c_interop.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-c-interop
cargo run -- ail-conformance examples/ui_workflow.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-conformance-ui
cargo run -- ail-build examples/support_ticket.ail --spec-file examples/support_ticket.ail/spec.ail-spec.md --agent examples/ail_toolchain_agent.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-build-support --target linux-x86_64-elf --action CloseTicket --out /tmp/ail-v02-release-evidence/artifacts/v02-close-ticket
cargo run -- ail-compile examples/c_interop.ail --target wasm32-unknown-sandbox-wasm --all-actions --agent examples/ail_toolchain_agent.ail --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-wasm-host-contract
cargo run -- ail-compile examples/support_ticket.ail --target aarch64-apple-darwin-libsystem-macho --action CloseTicket --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-darwin-contract
cargo run -- ail-spec --core-file /tmp/ail-v02-release-evidence/artifacts/v02-build-support/checked.ail-core.txt --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-spec-roundtrip
cargo run -- ail-bootstrap examples/ail_toolchain_agent.ail --pass examples/compiler_pass.ail --agent examples/ail_toolchain_agent.ail --target linux-x86_64-elf --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-bootstrap
cargo run -- ail-examples examples --artifact-dir /tmp/ail-v02-release-evidence/artifacts/v02-examples --release-evidence
```

The release audit fails if any command fails, if a named command does not
exist, or if any expected artifact directory is missing its manifest and
fingerprint files.

## Required Release Artifacts

The v0.2 release evidence bundle must contain:

- top-level `release-audit-manifest.txt`,
  `release-audit-manifest.fingerprint.txt`, and per-command stdout/stderr logs
- v0.1 release evidence regenerated on the v0.2 tree
- conformance artifact directory for each v0.2 profile or package fixture
- package dependency report and fingerprint for imported-package builds
- standard library conformance reports and fingerprints
- C/host interop conformance report and fingerprint
- UI profile conformance report and fingerprint
- Wasm host-contract report, dependency report, manifest, and fingerprints
- Darwin Mach-O contract report, dependency report, manifest, and fingerprints
- full checked-spec-to-native build artifact directory for Support Ticket,
  with prompt-to-artifact coverage proven by the prompt corpus and example
  artifacts
- `ail-spec --core-file` round-trip artifact directory
- VM bytecode artifact and fingerprint
- native ELF artifact and fingerprint
- AIL-Flow review artifact and fingerprint
- build manifest and manifest fingerprint
- agent bytecode, agent fingerprint, and agent trace
- prompt portability report and fingerprint
- 100-example end-to-end prompt corpus replay report and fingerprint
- per-example transcript, checked-artifact, Core, bytecode, target, manifest,
  and failure-taxonomy fingerprints for the end-to-end corpus
- model/executor manifest covering `llm-http` and `codex-skill-agent`
  executor families, endpoint labels, executor labels, and capture origins
- bootstrap artifact directory with fixed-point, host-boundary, dependency, and
  handoff reports

## Non-Goals For AIL v0.2

These remain post-v0.2 unless another completion gate revises them:

- production graphical IDE
- public package registry
- complete standard library
- direct native C linking for every platform
- executable Mach-O or PE/COFF backend
- fully self-hosted compiler replacing the bootstrap implementation
- optimizer pipeline
- every OS target
- complete natural-language grammar coverage
- formal proof system

## Current Status

As of the v0.1 completion baseline, AIL v0.2 is not complete. Current evidence
already covers parts of this gate:

- exact local imports and version mismatch rejection
- compatible local import range resolution and unbounded major range rejection
- registry index import resolution with recorded registry identity and missing
  registry entry rejection
- package dependency report records resolved import identities, source hashes,
  capability grants, approvals, and imported effect classes
- `ail-build`, `ail-lower`, `ail-conformance`, and `ail-compile` artifact
  manifests record package dependency reports and fingerprints for
  imported-package graphs
- imported action effects are rejected before bytecode lowering unless the root
  package grants the import alias, import path, or resolved package name for
  that effect class
- manifest preservation of capability grants
- C binding parsing into `ExternalBinding` Core nodes
- C interop package fixtures for external bindings, struct layout, callbacks,
  missing status maps, missing traces, and borrowed pointer escape
- route, form, dashboard, workflow, and accessibility parsing into checked UI
  Core nodes
- AIL-Flow projection for UI route, form, dashboard, workflow, and
  accessibility blocks
- UI rejected fixtures for action reachability, dashboard permission parity,
  destructive action confirmation, inaccessible error text, and workflow step
  ordering
- Wasm contract reports with host import enumeration
- Darwin Mach-O contract reports with libSystem external-symbol metadata,
  dependency reports, manifests, fingerprints, and Linux-only syscall
  rejection
- Linux native executable artifacts
- stored prompt portability corpus across base and target model labels,
  including accepted checked outputs and rejected prompt-envelope,
  profile-mismatch, hallucinated-capability, missing-trace, and semantic-drift
  taxonomy
- prompt-envelope checks and prompt-to-native build evidence
- `ail-examples` replay verifier, threshold tests, stored transcript replay,
  accepted prompt-output compilation to bytecode, VM traces, Linux native
  artifacts, Wasm and Darwin target-contract reports, rejected-output replay,
  top-level manifest/report fingerprints, and a fingerprinted
  model/executor manifest covering executor families, endpoint labels,
  executor labels, capture origins, executor/origin pairs, executor/endpoint
  pairs, and per-entry semantic task provenance
- checked 116-entry live release examples under `examples`,
  including 108 accepted prompt-to-artifact examples plus one rejected
  semantic-drift diagnostic example and one rejected profile-mismatch
  diagnostic example, one rejected missing-trace diagnostic example, and one
  rejected hallucinated-capability diagnostic example, plus one rejected
  unsupported-target backend diagnostic example and one rejected invalid
  interop diagnostic example and one rejected permission/capability diagnostic
  example, plus one rejected package-resolution diagnostic example
- example catalog entries now carry `use-case`, `capability-level`,
  `capability-under-test`, `program-scale`, `program-domain`,
  `module-count`, `spec-count`, `story-count`, `interacts-with`,
  `user-story-id`, `user-story`, `acceptance-criteria`, `story-evidence`,
  `story-journey`, `story-roundtrip`, `distinctness-claim`, and
  `v0.3-signal` metadata; replay reports count low-level, mid-level,
  high-level, utility, module, multi-module system, program domain, story
  journey, and story evidence coverage so prompt matrices cannot silently
  stand in for a useful learning corpus; each required domain must also span
  at least three prompt files and at least two story journeys
- every release example now writes a deterministic `user-story.txt` artifact
  and fingerprint into the replay bundle, tying user-story views to the same
  manifest/report path as checked Core, bytecode, VM traces, target reports,
  native artifacts, and diagnostics; checked story files must match catalog
  story, journey, evidence, domain, count, and interaction metadata
- four replay-clean live LLM captures for the Standard Collections, Support
  Ticket, and Refund Tool packages, using schema-shaped prompt input or
  constrained prose prompting with an OpenAI-compatible chat-completions
  endpoint with thinking disabled
- one hundred twelve replay-clean live Codex `codex-ail-spec-writer` captures for the
  Standard Collections, Composed Support, Refund Tool, Support Ticket,
  Stateful Counter, UI Workflow, C Interop, Network Driver, Compiler Pass,
  Secret Access, Repeated Task, Runtime Generic, and Incident Response
  packages, imported from recorded Codex sub-agent transcripts and replayed
  through the Darwin target-contract, VM, Wasm host-boundary target-contract,
  package-import, AgentTool, compiler, C interop system, secret access,
  repeated-task, runtime-generic, repair, multi-module incident workflow, and
  Linux native target paths
- a recorded Codex/skill-agent transcript importer that promotes stored request
  and response JSON into `capture-origin: live-codex` corpus entries for
  offline replay
- a batch capture runner that applies multiple live LLM captures and recorded
  Codex transcript imports to one corpus copy before replay
- a v0.2 release-audit runner that expands the audit command set, writes a
  top-level fingerprinted manifest, stores per-command logs, and verifies each
  artifact-producing command's manifest and `manifest.fingerprint.txt`
- named Codex skill-agent contracts for requirements writing, spec writing, and
  diagnostic repair under `examples/agents/`
- checked release example responses and extracted artifacts have zero
  duplicate fingerprints after deterministic per-scenario trace specialization;
  target reports also have zero duplicate fingerprints after contract reports
  started recording the compiled bytecode fingerprint
- package-import release entries replay through package-aware import
  resolution and compile the composed support package through checked Core,
  bytecode, and VM trace artifacts
- three real UI-profile release entries replay `ui_workflow.ail` through
  checked Core, semantic-contract bytecode, VM trace, and Wasm target-contract
  artifacts across the core-to-spec, spec-draft, and requirements prompt
  surfaces
- five Incident Response release entries replay a multi-module application
  with identity, policy, and notification imports through checked Core,
  bytecode, VM trace, Wasm target-contract, Darwin target-contract, workflow,
  dashboard, form, route, and regenerated-story evidence
- rejected-output example replay supports prompt-envelope diagnostics through
  stored transcript artifacts, including `AIL-PROMPT-001` diagnostics for
  malformed prompt envelopes and profile-mismatch checker handoffs, and
  checked AIL-Spec diagnostics for missing trace coverage and hallucinated
  capability or permission references, plus Darwin backend diagnostics for
  unsupported target effects and C interop diagnostics for invalid nullable
  pointer contracts, plus System profile diagnostics for missing capabilities
  and package-loader diagnostics for unresolved registry imports
- clean-worktree v0.2 release audit run at commit `ea37eeb`, generated with
  `python3 scripts/run_v02_release_audit.py --bundle-root
  /tmp/ail-semantic-anchor-report-clean-ea37eeb`; its
  `release-audit-manifest.fingerprint.txt` is `fnv64:45832a2198c7ad64`, and
  the audit manifest records `ok` for cargo format, diff whitespace, check,
  tests, clippy, conformance fixtures including Incident Response, build, Wasm
  host contract, Darwin contract, spec round-trip, bootstrap, and example
  release evidence, including the release semantic-anchor story coverage and
  replay-report anchor evidence gates

Post-v0.2 learning work is tracked in
`31-v03-learning-and-authoring-gate.md`.

Missing v0.2 evidence includes:

- none currently documented

## Completion Decision Rule

AIL v0.2 may be called complete only when:

- every required gate in this file has direct current evidence
- every minimum proof command exists and passes or is replaced by an explicitly
  documented stronger command in this file
- the release audit command set passes from a clean checkout
- release artifacts exist and are fingerprinted
- docs and tests name any intentionally deferred behavior as post-v0.2
- no desired-outcome row affected by v0.2 is supported only by prose

If any item lacks current evidence, the language remains at v0.1 plus
v0.2-in-progress implementation work.
