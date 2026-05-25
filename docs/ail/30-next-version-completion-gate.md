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

## Required Gates

### 1. Package Resolution And Capability Grants

Requirement: Imported packages are resolved through explicit package metadata,
versions, hashes, aliases, and capability grants. Imported behavior must not
receive ambient authority.

Evidence:

- Local exact imports still work and version mismatches still fail.
- Compatible local version ranges are resolved deterministically.
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
  conformance fixtures.
- `Option<T>`, `Result<T,E>`, `List<T>`, `Map<K,V>`, and `Set<T>` have checked
  type/variant surfaces.
- `Option.map` or an equivalent collection transform has bytecode and VM trace
  evidence.
- Rejected fixtures cover unresolved standard imports, version conflicts,
  missing capability grants, and invalid generic use.

Minimum proof commands:

```bash
cargo test ail_standard_library_option_type_parses_into_core
cargo test ail_standard_library_option_map_executes_collection_transform_bytecode
cargo test cli_ail_conformance_checks_standard_library_fixtures
cargo test cli_ail_std_rejects_invalid_generic_variant_payload
cargo test cli_ail_std_rejects_missing_capability_grant
```

### 3. Host Boundary And C Interop Contract

Requirement: C and host interop declarations are checked as safe AIL semantics
before any backend can expose them to a host.

Evidence:

- C function imports lower into `ExternalBinding` Core nodes with inputs,
  outputs, status maps, failures, capabilities, traces, ABI, and library
  metadata.
- Struct layout fixtures record size, alignment, field offsets, and target ABI.
- Callback fixtures record lifetime, reentrancy, allowed effects, and failure
  propagation.
- Ownership-transfer fixtures require release semantics.
- Rejected fixtures cover borrowed pointer escape, nullable-to-non-null
  mismatch, mutable pointer aliasing, missing status map, missing trace, and
  secret leakage across a foreign boundary.
- Wasm contract reports enumerate all host imports and preserve trace
  requirements.
- VM or contract execution evidence records a foreign-call trace for at least
  one deterministic host-boundary fixture.

Minimum proof commands:

```bash
cargo test ail_c_interop_import_parses_into_external_binding_core
cargo test cli_ail_compile_wasm_contract_enumerates_external_bindings
cargo test cli_ail_ffi_checks_struct_layout_fixture
cargo test cli_ail_ffi_checks_callback_lifetime_fixture
cargo test cli_ail_ffi_rejects_borrowed_pointer_escape
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
- Rejected fixtures cover action not reachable from form, missing permission
  parity, destructive action without confirmation, inaccessible error text, and
  workflow step ordering violations.

Minimum proof commands:

```bash
cargo test ail_ui_route_surface_parses_into_core
cargo test cli_ail_ui_form_calls_checked_action
cargo test cli_ail_ui_dashboard_requires_matching_permission
cargo test cli_ail_ui_workflow_blocks_out_of_order_provider_call
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

### 6. Prompt Portability Corpus

Requirement: Prompt-pack behavior is measured with stored accepted and rejected
model outputs rather than trusted by assumption.

Evidence:

- Prompt corpus includes interview, requirements, spec draft, repair,
  core-to-spec, flow patch, diagnostic repair, and trace-debug tasks.
- Corpus entries include at least two model labels or endpoint labels for the
  same semantic task.
- Accepted outputs produce checked artifacts.
- Rejected outputs demonstrate prompt-envelope, profile mismatch,
  hallucinated capability, missing trace, and semantic drift diagnostics.
- Portability reports include base model, target model, prompt fingerprint,
  artifact fingerprint, checker result, and failure taxonomy.

Minimum proof commands:

```bash
cargo test cli_ail_build_agent_compares_prompt_portability_before_compile
cargo test cli_ail_prompt_corpus_accepts_checked_outputs
cargo test cli_ail_prompt_corpus_rejects_semantic_drift_outputs
cargo test cli_ail_prompt_corpus_writes_portability_report
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
cargo fmt --check
git diff --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-v02-conformance-support
cargo run -- ail-conformance examples/refund_tool.ail --artifact-dir /tmp/ail-v02-conformance-refund
cargo run -- ail-conformance examples/compiler_pass.ail --artifact-dir /tmp/ail-v02-conformance-compiler
cargo run -- ail-conformance examples/network_driver.ail --artifact-dir /tmp/ail-v02-conformance-system
cargo run -- ail-conformance examples/std_collections.ail --artifact-dir /tmp/ail-v02-conformance-std-collections
cargo run -- ail-conformance examples/c_interop.ail --artifact-dir /tmp/ail-v02-conformance-c-interop
cargo run -- ail-conformance examples/ui_workflow.ail --artifact-dir /tmp/ail-v02-conformance-ui
cargo run -- ail-build examples/support_ticket.ail --prompt "Build an AIL support ticket bytecode artifact with imported package and UI host-boundary evidence" --agent examples/ail_toolchain_agent.ail --artifact-dir /tmp/ail-v02-build-support --target linux-x86_64-elf --action CloseTicket --out /tmp/ail-v02-close-ticket
cargo run -- ail-compile examples/c_interop.ail --target wasm32-unknown-sandbox-wasm --all-actions --agent examples/ail_toolchain_agent.ail --artifact-dir /tmp/ail-v02-wasm-host-contract
cargo run -- ail-compile examples/support_ticket.ail --target aarch64-apple-darwin-libsystem-macho --action CloseTicket --artifact-dir /tmp/ail-v02-darwin-contract
cargo run -- ail-spec --core-file /tmp/ail-v02-build-support/checked.ail-core.txt --artifact-dir /tmp/ail-v02-spec-roundtrip
cargo run -- ail-bootstrap examples/ail_toolchain_agent.ail --pass examples/compiler_pass.ail --agent examples/ail_toolchain_agent.ail --target linux-x86_64-elf --artifact-dir /tmp/ail-v02-bootstrap
```

The release audit fails if any command fails, if a named command does not
exist, or if any expected artifact directory is missing its manifest and
fingerprint files.

## Required Release Artifacts

The v0.2 release evidence bundle must contain:

- v0.1 release evidence regenerated on the v0.2 tree
- conformance artifact directory for each v0.2 profile or package fixture
- package dependency report and fingerprint for imported-package builds
- standard library conformance reports and fingerprints
- C/host interop conformance report and fingerprint
- UI profile conformance report and fingerprint
- Wasm host-contract report, dependency report, manifest, and fingerprints
- Darwin Mach-O contract report, dependency report, manifest, and fingerprints
- full prompt-to-native build artifact directory for Support Ticket
- `ail-spec --core-file` round-trip artifact directory
- VM bytecode artifact and fingerprint
- native ELF artifact and fingerprint
- AIL-Flow review artifact and fingerprint
- build manifest and manifest fingerprint
- agent bytecode, agent fingerprint, and agent trace
- prompt portability report and fingerprint
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
- package dependency report records resolved import identities, source hashes,
  capability grants, approvals, and imported effect classes
- manifest preservation of capability grants
- C binding parsing into `ExternalBinding` Core nodes
- route parsing into UI Core nodes
- Wasm contract reports with host import enumeration
- Linux native executable artifacts
- prompt-envelope checks and prompt-to-native build evidence

Missing v0.2 evidence includes:

- CLI manifest integration for package dependency reports on imported-package
  builds
- checker enforcement of imported capability grants
- standard library packages as first-class package fixtures
- struct layout, callback, ownership-transfer, and unsafe-pointer FFI fixtures
- UI forms, dashboards, workflows, accessibility diagnostics, and flow blocks
- Darwin Mach-O contract artifacts
- stored prompt portability corpus across model labels
- v0.2-specific release evidence bundle

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
