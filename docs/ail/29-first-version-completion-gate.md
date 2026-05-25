# AIL First Version Completion Gate

## Purpose

This gate defines when AIL v0.1 can be called the first fully usable version.
It is stricter than the first vertical slice and more concrete than the
implementation-readiness checklist. AIL v0.1 is complete only when a human or
AI agent can start from intent, reach checked semantics, review and edit those
semantics, compile runnable artifacts, and verify every transformation with
deterministic evidence.

AIL v0.1 does not require the mature language to be frozen. It requires the
first self-consistent programming loop to be executable, auditable, and
repeatable.

## Completion Definition

AIL v0.1 is complete when this chain is reproducible from a clean checkout:

```text
English intent
  -> AI agent interview or requirements capture
  -> checked AIL-Requirements
  -> checked AIL-Spec Canonical
  -> checked AIL-Core
  -> AIL-Flow review and checked visual edit path
  -> AIL-Bytecode
  -> VM execution
  -> Linux x86_64 ELF execution
  -> checked round-trip back to human-reviewable AIL-Spec
```

Every arrow must have at least one of:

- parser or checker gate
- deterministic fingerprint
- manifest entry
- conformance fixture
- runtime trace
- AIL-authored agent verification action
- rejected fixture proving invalid input is blocked

## Required Gates

### 1. English-To-Artifact Authoring

Requirement: A user can begin with English and obtain checked AIL artifacts
without manually writing the final AIL-Spec.

Evidence:

- `ail-interview` can return either focused questions or an `AIL-Interview`
  artifact.
- `ail-requirements` can produce checked `AIL-Requirements`.
- `ail-spec --requirements-file` can produce checked AIL-Spec and repair one
  checker-diagnosed candidate.
- `ail-build --prompt ... --agent ... --artifact-dir ...` writes requirements,
  accepted spec, checked core, flow review, bytecode, manifest, and agent trace.
- Malformed prompt-pack envelopes and wrong-profile handoffs are rejected.

Minimum proof commands:

```bash
cargo test cli_ail_interview_surfaces_prompt_envelope_questions_as_artifact
cargo test cli_ail_requirements_repairs_incomplete_capture_before_printing
cargo test cli_ail_spec_drafts_and_repairs_from_checked_requirements_file
cargo test cli_ail_build_uses_llm_candidate_and_outputs_verified_bytecode
cargo test cli_ail_build_runs_toolchain_agent_bytecode
```

### 2. Trusted Semantic Core

Requirement: AIL-Core is the trusted semantic source after checking.

Evidence:

- AIL-Core schema version, package metadata, safety level, target support,
  imports, capability grants, nodes, edges, and provenance round-trip through
  the text artifact.
- Stable semantic hashes are deterministic across render order where order is
  semantically irrelevant.
- Unknown node kinds, edge kinds, schema versions, type names, and unsafe
  semantic gaps are rejected before lowering.

Minimum proof commands:

```bash
cargo test ail_core_text_preserves_package_entry_features_and_imports
cargo test ail_core_text_preserves_manifest_schema_version_and_safety_level
cargo test ail_core_text_preserves_manifest_capability_grants
cargo test ail_core_rendering_and_hash_are_stable_across_edge_order
cargo test cli_ail_spec_rejects_unknown_core_schema_items
```

### 3. Required Round Trips

Requirement: Source, IR, visual review, and bytecode projections preserve the
same checked semantics or fail with diagnostics.

Evidence:

- AIL-Spec -> AIL-Core -> AIL-Spec reparses to equivalent Core.
- AIL-Core -> AIL-Spec -> AIL-Core writes source, rendered, round-trip, hash,
  and manifest artifacts.
- AIL-Core -> AIL-Flow edit -> AIL-Core patch checks base hash and semantic
  validity.
- AIL-Core -> AIL-Bytecode -> execution trace preserves checked behavior.

Minimum proof commands:

```bash
cargo test ail_spec_render_reparse_preserves_core_equivalence
cargo test cli_ail_spec_core_file_writes_roundtrip_artifacts
cargo test cli_ail_flow_edit_adds_action_requirement_and_native_enforces_it
cargo test ail_bytecode_vm_executes_close_ticket_success_and_failure
cargo test cli_ail_vm_executes_saved_bytecode_artifact
```

### 4. Executable Semantics

Requirement: The first version has real programming semantics, not only
declarative records.

Required behavior:

- state mutation
- requirements and preconditions
- failure propagation
- trace emission
- function calls
- action calls
- branching
- loop or recursion
- integer state update
- external call or tool-call contract

Minimum proof commands:

```bash
cargo test ail_spec_lowers_function_surface_into_runnable_bytecode
cargo test ail_spec_lowers_action_call_bullets_to_call_action_bytecode
cargo test ail_bytecode_vm_executes_branch_and_jump_control_flow
cargo test ail_bytecode_vm_executes_integer_loop_state_mutation
cargo test ail_bytecode_vm_executes_action_call_control_flow
cargo test ail_agent_tool_profile_lowers_to_verified_bytecode
```

Required fixtures before completion:

- recursive factorial
- map/filter/reduce or equivalent collection transform
- stateful counter
- event-loop or repeated task fixture
- compiler graph pass

If a fixture is represented by a different example name, this file must be
updated to name the replacement explicitly before v0.1 can be claimed.

### 5. VM, Native ELF, And Wasm Artifact Boundaries

Requirement: Checked semantics lower to bytecode, execute in the VM, compile to
native Linux x86_64 ELF, and produce a Wasm target contract.

Evidence:

- `ail-lower` writes bytecode and a manifest from source or saved Core.
- `ail-vm` executes saved bytecode without source package access.
- `ail-compile --target linux-x86_64-elf` emits runnable ELF bytes.
- `ail-compile --target wasm32-unknown-sandbox-wasm` emits a contract report
  and rejects stale native artifacts in Wasm-only artifact directories.
- Backend manifests include target artifact fingerprints and dependency
  reports when native output is emitted.

Minimum proof commands:

```bash
cargo test cli_ail_lower_accepts_saved_core_file_artifact
cargo test cli_ail_vm_executes_saved_bytecode_artifact
cargo test cli_ail_compile_emits_runnable_linux_x86_64_elf_executable
cargo test cli_ail_compile_package_writes_wasm_contract_artifacts
cargo test cli_ail_compile_wasm_contract_bundle_rejects_stale_native_bundle_artifacts
```

### 6. AI Agent As IDE, But Untrusted

Requirement: The AI agent can drive authoring and review, but cannot bypass
the checker or silently rewrite accepted behavior.

Evidence:

- The AIL-authored toolchain agent has bytecode actions for requirements
  capture, spec preparation, spec acceptance, compiler-pass acceptance, Core
  acceptance, Flow review acceptance, application compilation, bytecode
  verification, target verification, and manifest verification.
- Build traces show the agent action order before artifacts are trusted.
- Bad saved specs fail before agent acceptance.
- Prompt portability reports can be generated for a target model label.

Minimum proof commands:

```bash
cargo test ail_toolchain_agent_package_lowers_to_verified_bytecode
cargo test cli_ail_build_agent_accepts_flow_review_before_compile
cargo test cli_ail_build_saved_spec_checks_before_agent_acceptance
cargo test cli_ail_build_agent_verifies_bytecode_artifact_after_compile
cargo test cli_ail_build_agent_compares_prompt_portability_before_compile
```

### 7. No-Code Review And Patch Path

Requirement: Reviewers can inspect checked Core through AIL-Flow and apply
checked visual edits without editing host-language code.

Evidence:

- `ail-flow` renders checked Core with core hash, core labels, and edge
  references.
- `ail-flow-edit` translates supported visual edits into checked Core patches.
- Supported flow edits include action rename, action requirement addition, and
  data table field addition.
- Patched Core can render back to AIL-Spec and lower to bytecode/native output.

Minimum proof commands:

```bash
cargo test ail_flow_projection_renders_no_code_view_from_core
cargo test cli_ail_flow_edit_renames_action_card_and_round_trips_to_spec
cargo test cli_ail_flow_edit_adds_data_table_field_and_round_trips_to_spec
cargo test cli_ail_flow_edit_adds_action_requirement_and_native_enforces_it
```

### 8. Package And Profile Coverage

Requirement: The first version supports the minimum profile set needed to show
AIL is a language family, not one application DSL.

Required profiles:

- Application
- AgentTool
- Compiler
- System, at least as checked Core and conformance fixtures

Evidence:

- Support Ticket package exercises Application semantics.
- Refund Tool package exercises AgentTool semantics.
- Compiler Pass package exercises AIL-Meta compiler-pass semantics.
- Network Driver package exercises System resources, ownership, borrowing,
  effects, scheduling, interrupts, locks, regions, and layouts.
- Local package imports and exact version mismatches are checked.

Minimum proof commands:

```bash
cargo test cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test cli_ail_conformance_checks_agent_tool_fixtures
cargo test cli_ail_conformance_checks_compiler_profile_fixtures
cargo test cli_ail_conformance_checks_system_profile_fixtures
cargo test ail_package_loader_accepts_versioned_imports_and_rejects_mismatches
```

### 9. Diagnostics And Rejected Fixtures

Requirement: Invalid programs fail with stable diagnostics that are useful to
humans and AI agents.

Evidence:

- Every v0.1 checker rule has a stable diagnostic code or documented verifier
  error string.
- Rejected fixtures cover missing references, secret leaks, missing failure
  handling, unknown fields, unknown types, missing traces, unsafe tool output,
  permission/approval omissions, and System profile ownership/effect errors.
- Machine-readable diagnostics exist for draft/repair loops.

Minimum proof commands:

```bash
cargo test cli_ail_draft_can_emit_machine_readable_diagnostics
cargo test cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test cli_ail_conformance_checks_agent_tool_fixtures
cargo test cli_ail_conformance_checks_system_profile_fixtures
```

### 10. Self-Hosting Direction Has Executable Evidence

Requirement: v0.1 does not need a fully self-hosted compiler, but it must prove
the self-hosting path with AIL-authored toolchain components.

Evidence:

- An AIL-Meta compiler pass executes over checked AIL-Core.
- Bootstrap artifacts include toolchain-agent source snapshots, compiler-pass
  source snapshots, checked Core, bytecode, fixed-point report, conformance
  reports, native bytecode report, host-boundary report, dependency report,
  handoff report, and manifest.
- Native handoff runs every emitted toolchain-agent, verifier-agent, and
  compiler-pass ELF action and records trace markers.

Minimum proof commands:

```bash
cargo test ail_compiler_pass_bytecode_transforms_checked_core_ir
cargo test cli_ail_pass_runs_compiler_pass_over_checked_package_core
cargo test cli_ail_bootstrap_writes_native_toolchain_bundle
```

## Release Audit Command Set

Before claiming AIL v0.1 complete, run this command set from the repository
root and preserve the output as release evidence:

```bash
cargo fmt --check
git diff --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-v0-conformance-support
cargo run -- ail-conformance examples/refund_tool.ail --artifact-dir /tmp/ail-v0-conformance-refund
cargo run -- ail-conformance examples/compiler_pass.ail --artifact-dir /tmp/ail-v0-conformance-compiler
cargo run -- ail-conformance examples/network_driver.ail --artifact-dir /tmp/ail-v0-conformance-system
cargo run -- ail-build examples/support_ticket.ail --prompt "Build an AIL support ticket bytecode artifact" --agent examples/ail_toolchain_agent.ail --artifact-dir /tmp/ail-v0-build-support --target linux-x86_64-elf --action CloseTicket --out /tmp/ail-v0-close-ticket
cargo run -- ail-spec --core-file /tmp/ail-v0-build-support/checked.ail-core.txt --artifact-dir /tmp/ail-v0-spec-roundtrip
```

The release audit fails if any command fails or if any expected artifact
directory is missing its manifest and fingerprint files.

## Required Release Artifacts

The v0.1 release evidence bundle must contain:

- conformance artifact directory for each required profile
- full prompt-to-native build artifact directory for Support Ticket
- `ail-spec --core-file` round-trip artifact directory
- VM bytecode artifact and fingerprint
- native ELF artifact and fingerprint
- AIL-Flow review artifact and fingerprint
- build manifest and manifest fingerprint
- agent bytecode, agent fingerprint, and agent trace
- bootstrap artifact directory with fixed-point, host-boundary, dependency, and
  handoff reports

## Non-Goals For AIL v0.1

These are explicitly post-v0.1 unless another completion gate revises them:

- production graphical IDE
- full package registry
- complete standard library
- fully self-hosted compiler replacing the bootstrap implementation
- optimizer pipeline
- every OS target
- complete natural-language grammar coverage
- formal proof system

## Completion Decision Rule

AIL v0.1 may be called complete only when:

- every required gate in this file has direct current evidence
- the release audit command set passes from a clean checkout
- release artifacts exist and are fingerprinted
- docs and tests name any intentionally deferred behavior as post-v0.1
- no desired-outcome row in `27-desired-outcome-traceability.md` is supported
  only by prose for the v0.1 scope

If any item lacks current evidence, the language remains in pre-v0.1
implementation progress.
