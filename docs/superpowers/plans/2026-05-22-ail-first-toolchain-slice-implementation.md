# AIL First Toolchain Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the first AIL toolchain slice: package loading, structured-English parsing, AIL-Core elaboration, deterministic serialization, and base LLM endpoint wiring.

**Architecture:** Add a focused `src/ail.rs` module that owns AIL package metadata, parsed AIL-Spec structures, AIL-Core graph construction, deterministic rendering, and simple core diagnostics. Reuse the existing `core_model::Graph` primitives for graph storage so the AIL path aligns with the current prototype instead of creating an unrelated IR stack.

**Tech Stack:** Rust 2024, existing standard-library-only codebase, `cargo test`, current docs under `docs/ail`, llama.cpp-compatible endpoint at `http://inteligentia-pro-1:8080/v1/chat/completions`.

---

### Task 1: AIL Package Loader And Spec Parser

**Files:**
- Create: `src/ail.rs`
- Modify: `src/lib.rs`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing tests**

Add tests that require:

- `load_ail_package_dir("examples/support_ticket.ail")` reads `ail-package.md`
- the default base LLM endpoint is `http://inteligentia-pro-1:8080/v1/chat/completions`
- `parse_ail_spec_text` extracts the Support Ticket application, `Ticket` fields, `Secret<List<Text>>`, `CreateTicket`, `AssignTicket`, `CloseTicket`, `NotFound`, and `PermissionDenied`

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: compile failure because the `ail` module and functions do not exist.

- [x] **Step 3: Implement package metadata and parser**

Create `src/ail.rs` with package structs, metadata parsing for `key: value`
lines, default LLM endpoint wiring, structured-English parsing for the current
AIL-Spec example shape, and public functions exported through `src/lib.rs`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: package and parser tests pass.

### Task 2: AIL-Core Elaboration And Deterministic Serialization

**Files:**
- Modify: `src/ail.rs`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing tests**

Add tests that require:

- Support Ticket AIL-Spec elaborates into AIL-Core nodes for `Application`,
  `Thing`, `Field`, `Action`, `Failure`, `Guarantee`, `Secret`, `Trace`, and
  `Provenance`
- deterministic serialization contains stable lines for `CloseTicket`,
  `Ticket.internal notes`, `PermissionDenied`, and source provenance
- `check_ail_core` returns no diagnostics for the Support Ticket example

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: failure because AIL-Core elaboration and serialization are missing.

- [x] **Step 3: Implement elaboration and checker**

Build `core_model::Program` from parsed AIL docs, render sorted deterministic
text, and add simple diagnostics for missing provenance, actions without traces,
secret fields without protection, and failures without handling text.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: AIL-Core tests pass.

### Task 3: CLI Surface For The First Slice

**Files:**
- Modify: `src/main.rs`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing CLI tests**

Add tests for:

- `eigl ail-check examples/support_ticket.ail` prints `ail diagnostics: none`
- `eigl ail-core examples/support_ticket.ail` prints deterministic AIL-Core
- package metadata output includes the base LLM endpoint

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: CLI tests fail because the commands are missing.

- [x] **Step 3: Implement CLI commands**

Add `ail-check` and `ail-core` commands that load the package, parse the entry
spec, elaborate AIL-Core, run diagnostics, and print deterministic output.

- [x] **Step 4: Verify GREEN And Baseline**

Run:

```bash
cargo test --test ail_toolchain
cargo test
```

Expected: targeted tests and full suite pass.

### Task 4: AIL-Spec Round Trip And Trace Runtime

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing render/reparse and runtime tests**

Add tests requiring deterministic AIL-Spec rendering, reparsing to equivalent
AIL-Core, `CloseTicket` success execution with state changes and trace events,
`NotFound` failure execution with failure trace events, and `ail-run` CLI
coverage for both paths.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: compile failure because `render_ail_spec` and `run_ail_action` do not
exist yet.

- [x] **Step 3: Implement renderer, equivalence path, and runtime**

Add deterministic AIL-Spec rendering, reparsing through the existing parser,
runtime action execution for the Support Ticket slice, and `ail-run --action`
CLI support.

- [x] **Step 4: Verify GREEN And Baseline**

Run:

```bash
cargo test --test ail_toolchain
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: targeted tests, full suite, and clippy pass.

### Task 5: Conformance Fixtures And CLI Gate

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `examples/support_ticket.ail/examples/rejected/*.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing conformance test**

Add a CLI test requiring `eigl ail-conformance examples/support_ticket.ail`
to accept `spec.ail-spec.md`, reject each fixture under `examples/rejected`,
print the stable diagnostic code for each rejected fixture, and finish with
`ail conformance: ok`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because `ail-conformance` is not routed through the AIL
package loader yet.

- [x] **Step 3: Implement conformance runner and CLI command**

Add `run_ail_conformance`, structured conformance result types, sorted rejected
fixture discovery, parse-error reporting for invalid rejected specs, CLI output
for accepted/rejected fixtures, and command routing for `ail-conformance`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the conformance CLI test passes.

### Task 6: LLM Draft Command For AIL-Spec Candidates

**Files:**
- Modify: `src/llm_bridge.rs`
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing LLM draft CLI test**

Add a mock llama.cpp chat-completions server test requiring
`eigl ail-draft examples/support_ticket.ail --prompt ... --llm-endpoint ...`
to send a chat request with thinking disabled, include package and AIL-Spec
drafting instructions, sanitize fenced/thinking model text, parse/check the
candidate, and print `ail-draft diagnostics: none` for a valid candidate.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_draft_uses_llm_endpoint_and_checks_candidate_spec
```

Expected: failure because `ail-draft` is not routed through the AIL package
loader and sends no model request.

- [x] **Step 3: Implement draft bridge and CLI command**

Expose sanitized LLM text invocation from the existing llama.cpp bridge, add
`draft_ail_spec`, parse/check the model candidate, add `--prompt` parsing, and
print the candidate plus diagnostics through `ail-draft`. Normalize common
model type aliases such as `String` and `Secret List<String>` to canonical AIL
types, and include canonical type spellings in the draft prompt.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_draft_uses_llm_endpoint_and_checks_candidate_spec
```

Expected: the draft CLI test passes.

### Task 7: Checker Diagnostics For Unknown Field References

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/unknown-field.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing invalid-fixture test**

Add a rejected AIL-Spec fixture where an action reads `ticket owner email` and
writes `ticket archive code to Archived` even though those fields are not
declared. Extend the invalid-fixture and conformance tests to require stable
`AIL004` diagnostics for both read and write references.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because unresolved read/write field references are still
lowered to fallback `Effect` nodes without a diagnostic.

- [x] **Step 3: Implement AIL004 checking**

Add a checker pass that inspects `reads` and `writes` edges pointing at fallback
effects. When the effect text looks like it is referencing a declared Thing
field but no declared field resolved, emit `AIL004`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: the invalid-fixture diagnostic test passes.

### Task 8: AIL-Flow No-Code Projection

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing projection and CLI tests**

Add tests requiring checked AIL-Core to render a deterministic AIL-Flow JSON
projection with package/application identity, things, fields, views, action
cards, requirements, reads, writes, guarantees, and trace events. Add CLI
coverage for `eigl ail-flow examples/support_ticket.ail`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_flow_projection_renders_no_code_view_from_core
```

Expected: compile failure because `render_ail_flow_view` does not exist.

- [x] **Step 3: Implement AIL-Flow renderer and command**

Render the no-code projection from accepted AIL-Core graph nodes and edges, and
route `ail-flow` through the same package parse, elaboration, and checker gate
as `ail-core`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_flow_projection_renders_no_code_view_from_core
cargo test --test ail_toolchain cli_ail_check_and_core_use_package_loader
```

Expected: the projection and CLI tests pass.

### Task 9: Checked AIL Patch Application

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/patches/escalate-ticket.ail-patch.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing patch tests**

Add a sample AIL patch that targets the Support Tickets application, adds a
`Ticket.escalation reason` field, adds an escalation queue view, and adds an
`Escalate ticket` action. Add tests requiring the patch to apply to the parsed
AIL document, check cleanly, render deterministic AIL-Spec, and reparse to
equivalent AIL-Core. Add CLI coverage for `ail-patch`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_patch_adds_field_view_and_action_then_round_trips
```

Expected: compile failure because AIL patch parsing and application APIs do not
exist.

- [x] **Step 3: Implement AIL patch parse/apply and CLI command**

Add a small typed AIL patch model for application-level field, view, and action
additions. Apply patches to `AilDocument`, re-elaborate and check before
printing canonical AIL-Spec through `ail-patch`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_patch_adds_field_view_and_action_then_round_trips
cargo test --test ail_toolchain cli_ail_check_and_core_use_package_loader
```

Expected: the patch API and CLI tests pass.

### Task 10: AIL Package Imports And Alias Namespacing

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_shared.ail/`
- Create: `examples/support_composed.ail/`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing package import tests**

Add `support_shared.ail` and `support_composed.ail` fixtures where the composed
package declares `imports: ../support_shared.ail as Shared`. Add tests requiring
the package loader to read the import, imported declarations to be namespaced
as `Shared.*`, `ail-core` to include the namespaced nodes, and canonical
AIL-Spec render/reparse to preserve AIL-Core equivalence.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_package_imports_namespace_declarations_and_round_trip
```

Expected: compile failure because package imports and
`parse_ail_package_document` do not exist.

- [x] **Step 3: Implement package import loading and namespaced merge**

Add import metadata parsing, recursive package loading with cycle detection,
package-aware document parsing, and whole-fragment alias qualification for
imported things, actions, failures, fields, references, and types. Route AIL CLI
commands through package-aware parsing.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_package_imports_namespace_declarations_and_round_trip
cargo test --test ail_toolchain cli_ail_check_and_core_use_package_loader
```

Expected: the package import and CLI tests pass.

### Task 11: Generic Runtime Field Writes And Requirements

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/runtime_generic.ail/`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing runtime tests**

Add a `runtime_generic.ail` fixture with a `Prioritize ticket` action that
requires the ticket to exist, requires `ticket.priority` not to be `High`,
changes `ticket.priority` to `High`, guarantees handling order, and records a
trace. Include a same-field-name decoy thing so qualified text such as
`ticket priority` must resolve to `ticket.priority`, not an arbitrary longer
thing name. Add library and CLI tests requiring `run_ail_action`/`ail-run` to
update `ticket.priority=High` and fail when the negative requirement is
violated.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_executes_generic_field_writes_and_requirements
```

Expected: failure because the runtime only updates the hardcoded
`ticket.status` field.

- [x] **Step 3: Implement generic runtime field resolution**

Resolve runtime requirement and write phrases against declared AIL fields. Use
declared thing/field names to derive runtime keys such as `ticket.priority`,
handle simple existence requirements through `<subject>.id`, handle negative
`not to be` requirements, and handle `changes <field> to <value>` assignments.
Prefer explicit qualified field references first; resolve bare field names only
when they are unambiguous.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_executes_generic_field_writes_and_requirements
cargo test --test ail_toolchain cli_ail_check_and_core_use_package_loader
```

Expected: the generic runtime and CLI tests pass.

Additional RED/GREEN checkpoint after local review:

```bash
cargo test --test ail_toolchain ail_runtime_executes_generic_field_writes_and_requirements
```

Expected: the same-field-name decoy initially exposes incorrect field
resolution, then passes after qualified references are prioritized.

### Task 12: AIL Runtime Secret Redaction

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing CLI redaction test**

Add an `ail-run` regression using the Support Ticket package where runtime
state includes `ticket.internal notes=sensitive note`. Require CLI output to
print `ticket.internal notes=<secret>` and never disclose the raw secret value.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_run_redacts_secret_runtime_state
```

Expected: failure because `ail-run` prints raw final-state values directly.

- [x] **Step 3: Implement redacted AIL runtime-state rendering**

Add an AIL runtime-state rendering helper that checks runtime keys against
declared secret fields and substitutes `<secret>` for display. Keep
`AilRunResult.final_state` raw for internal execution semantics, and route the
`ail-run` CLI output through the redacting helper.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_run_redacts_secret_runtime_state
```

Expected: the CLI redaction test passes.

### Task 13: Positive Field Requirements And Read Traces

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/secret_access.ail/`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing runtime and CLI tests**

Add a `secret_access.ail` fixture with a `Requester.role` state field, a secret
`Ticket.internal notes` field, and a `View internal notes` action. Require the
runtime to pass when `requester.role=SupportAgent`, fail with
`PermissionDenied` when `requester.role=Customer`, trace the secret field read
without exposing the secret value, and keep CLI output redacted.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_positive_field_requirements_and_read_traces
```

Expected: failure because positive `to be A or B` field requirements are only
observed and reads are not traced.

- [x] **Step 3: Implement positive field requirement enforcement**

Resolve `the <thing> <field> to be <value> or <value>` against declared fields,
compare runtime state to the allowed values, fail missing or disallowed values,
and map role/permission requirement failures to a declared `PermissionDenied`
failure when available. Emit semantic read trace entries for action reads.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_positive_field_requirements_and_read_traces
cargo test --test ail_toolchain cli_ail_check_and_core_use_package_loader
```

Expected: the library and CLI coverage pass.

### Task 14: Secret Read Protection Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/secret-read-without-protection.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing rejected-fixture tests**

Add a rejected Support Ticket fixture where an action reads
`ticket internal notes` but does not declare any `does not reveal ...`
protection rule. Extend the stable diagnostic and conformance tests to require
`AIL005`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because the checker only reports unprotected secret writes.

- [x] **Step 3: Implement secret-read protection checking**

Add a checker pass for `reads` edges targeting secret fields. Reuse the same
action-to-secret-field protection edge used for write checks, but emit stable
`AIL005` diagnostics for read violations so conformance can distinguish read
and write leaks.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the focused diagnostic and conformance tests pass.

### Task 28: Structured Conformance Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing conformance metadata test**

Require `ail-conformance` rejected output for `missing-reference.ail-spec.md`
to include the existing stable `AIL001` message plus
`source=action:CloseTicket.requirement:the account to exist` and a concrete
repair suggestion.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because conformance output only prints plain diagnostic
strings.

- [x] **Step 3: Implement structured diagnostic carrier**

Add `AilDiagnostic` with stable code, message, severity, optional source
provenance, affected graph item, and repair suggestion. Keep `check_ail_core`
string-compatible by rendering `check_ail_core_diagnostics` back to plain
messages, and make conformance output use `detailed_message()`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo run -- ail-conformance examples/support_ticket.ail
```

Expected: the focused conformance test passes, and rejected fixture lines retain
their stable codes while surfacing metadata for structured diagnostics.

### Task 29: Structured Secret Access Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing secret metadata test**

Require `ail-conformance` rejected output for `secret-leak.ail-spec.md` and
`secret-read-without-protection.ail-spec.md` to include the existing stable
`AIL002`/`AIL005` messages plus source behavior bullet provenance and concrete
repair suggestions.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because secret access diagnostics still print plain strings.

- [x] **Step 3: Preserve read/write edge provenance**

Store `provenance` attributes on action `reads` and `writes` edges when
elaborating AIL-Core. Convert the secret access checker to emit
`AilDiagnostic` values with source provenance from that edge, affected graph
item set to the edge ID, and a repair suggestion that adds a matching
`the system does not reveal ...` protection bullet.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: focused conformance output includes metadata for both secret write
and secret read violations while the legacy plain checker messages remain
unchanged.

### Task 30: Structured Failure Lifecycle Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing failure metadata test**

Require `ail-conformance` rejected output for `missing-failure-handler.ail-spec.md`,
`failure-without-handling.ail-spec.md`, and `failure-without-trace.ail-spec.md`
to include the existing stable `AIL003`/`AIL008`/`AIL009` messages plus source
provenance and concrete repair suggestions.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because failure lifecycle diagnostics still print plain
strings.

- [x] **Step 3: Preserve failure edge and declaration provenance**

Store `provenance` attributes on action `may_fail_with` edges when elaborating
AIL-Core. Prefer declared failure-section provenance on `Failure` nodes so
`AIL003` points at the action failure edge while `AIL008` and `AIL009` point at
the declared failure node. Convert all three checks to emit `AilDiagnostic`
values with source provenance, affected graph item, and repair guidance.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: focused conformance output includes metadata for failure declaration,
handling, and trace coverage violations while legacy plain checker messages
remain unchanged.

### Task 31: Structured Unknown Field Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing unknown-field metadata test**

Require `ail-conformance` rejected output for `unknown-field.ail-spec.md` to
include the existing stable `AIL004` read/write messages plus source behavior
bullet provenance and repair suggestions that distinguish read bullets from
write bullets.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because unknown-field diagnostics still print plain strings.

- [x] **Step 3: Emit structured unknown-field diagnostics**

Convert `check_unknown_field_references` to return `AilDiagnostic` values. Use
the unresolved `reads` or `writes` edge provenance as the source, the edge ID as
the affected graph item, and a repair suggestion that tells the author to
declare the missing field or update the read/write bullet to an existing field.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: focused conformance output includes metadata for both unknown field
read and write violations while legacy plain checker messages remain unchanged.

### Task 32: Structured Field Validation Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing field validation metadata test**

Require `ail-conformance` rejected output for `unknown-field-type.ail-spec.md`
and `unknown-requirement-field.ail-spec.md` to include the existing stable
`AIL006`/`AIL007` messages plus source provenance and repair suggestions.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because field validation diagnostics still print plain
strings.

- [x] **Step 3: Emit structured field validation diagnostics**

Convert `check_field_types` and `check_requirement_field_references` to return
`AilDiagnostic` values. Use field declaration provenance for `AIL006`,
requirement rule provenance for `AIL007`, and point affected graph items at the
field or rule node.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: focused conformance output includes metadata for unknown field types
and unknown requirement fields while legacy plain checker messages remain
unchanged.

### Task 33: Structured Semantic Integrity Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing semantic integrity metadata tests**

Extend the focused `AIL011` through `AIL016` checker tests to inspect
`check_ail_core_diagnostics` and require affected graph items, source
provenance when available, and repair suggestions.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_
```

Expected: the semantic integrity tests fail because these diagnostics are still
plain messages converted through `AilDiagnostic::from_message`.

- [x] **Step 3: Emit structured semantic integrity diagnostics**

Convert the provenance and attachment checker passes for `AIL011` through
`AIL016` to return `AilDiagnostic` values directly. Point each diagnostic at
the semantic node whose graph invariant is incomplete, preserve source
provenance when present, and keep `check_ail_core` string-compatible.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: focused semantic integrity diagnostics include graph metadata and
repairs while legacy plain checker messages remain unchanged.

### Task 34: Structured AIL Draft Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing draft diagnostic CLI test**

Mock the LLM chat endpoint with a rejected Support Ticket candidate and require
`eigl ail-draft ...` to return non-zero while printing the same structured
diagnostic metadata used by conformance output.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_draft_prints_structured_candidate_diagnostics
```

Expected: failure because `ail-draft` only prints the plain checker diagnostic
string for invalid candidates.

- [x] **Step 3: Route draft checks through structured diagnostics**

Change `AilDraftResult` to carry `AilDiagnostic` values, build candidate
diagnostics with `check_ail_core_diagnostics`, preserve structured parse
failures as `AIL000`, and print `detailed_message()` in the CLI.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_draft_prints_structured_candidate_diagnostics
cargo test --test ail_toolchain cli_ail_draft_uses_llm_endpoint_and_checks_candidate_spec
```

Expected: invalid draft candidates print code, source, affected graph item, and
repair guidance while valid draft candidates still report no diagnostics.

### Task 35: Minimal AgentTool Profile Support

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/refund_tool.ail/ail-package.md`
- Create: `examples/refund_tool.ail/spec.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing Refund Tool profile test**

Add a package fixture for the documented Refund Tool example and require the
AIL package parser, core elaborator, checker, AIL-Flow projection, and
canonical render/reparse path to understand `Tool:` contracts with typed
inputs, outputs, requirements, reads, writes, calls, secrets, failures, and
guarantees.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_agent_tool_profile_parses_renders_and_checks_refund_tool
```

Expected: compile failure because `AilDocument` has no tool model.

- [x] **Step 3: Implement minimal Tool model and graph lowering**

Add `AilTool` and `AilToolSlot`, parse AgentTool surface sections, lower tools
to `Tool`, `Input`, `Output`, `Rule`, `Effect`, `Secret`, `Failure`,
`Guarantee`, and `Trace` graph items, and let checker attachment rules accept
tool-owned rules, effects, traces, and secrets.

- [x] **Step 4: Add AgentTool draft prompt coverage**

Require `ail-draft` on an AgentTool package to teach the LLM the tool contract
surface instead of the application/action surface, while preserving the existing
Application prompt behavior.

- [x] **Step 5: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_agent_tool_profile_parses_renders_and_checks_refund_tool
cargo test --test ail_toolchain cli_ail_draft_for_agent_tool_profile_prompts_tool_surface
cargo test --test ail_toolchain cli_ail_draft_uses_llm_endpoint_and_checks_candidate_spec
```

Expected: Refund Tool packages check, render, and round-trip through AIL-Core,
and the LLM draft prompt is profile-appropriate for both AgentTool and
Application packages.

### Task 36: Minimal AIL-Meta Compiler Pass Support

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/compiler_pass.ail/ail-package.md`
- Create: `examples/compiler_pass.ail/spec.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing Compiler profile test**

Add a package fixture for the documented `Infer read permissions` compiler
pass and require the AIL parser, core elaborator, checker, AIL-Flow projection,
and canonical render/reparse path to understand `Compiler pass:` declarations
with typed values, reads, writes, steps, failures, guarantees, and traces.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_profile_parses_renders_and_checks_compiler_pass
```

Expected: compile failure because `AilDocument` has no compiler-pass model.

- [x] **Step 3: Implement minimal compiler-pass model and graph lowering**

Add `AilCompilerPass` and `AilPassValue`, parse Compiler profile sections,
lower compiler passes to `Action` nodes marked `kind=CompilerPass` plus typed
`Value`, `Step`, `Effect`, `Failure`, `Guarantee`, and `Trace` graph items, and
preserve canonical render/reparse equality.

- [x] **Step 4: Add Compiler draft prompt coverage**

Require `ail-draft` on a Compiler package to teach the LLM the compiler-pass
surface instead of the Application or AgentTool surface, while preserving the
existing profile prompt behavior.

- [x] **Step 5: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_profile_parses_renders_and_checks_compiler_pass
cargo test --test ail_toolchain cli_ail_draft_for_compiler_profile_prompts_compiler_pass_surface
cargo test --test ail_toolchain cli_ail_draft_for_agent_tool_profile_prompts_tool_surface
cargo test --test ail_toolchain cli_ail_draft_uses_llm_endpoint_and_checks_candidate_spec
```

Expected: Compiler packages check, render, and round-trip through AIL-Core, and
the LLM draft prompt is profile-appropriate for Compiler, AgentTool, and
Application packages.

### Task 37: Cross-Profile Type Validation

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing cross-profile type diagnostic test**

Mutate the Refund Tool and Compiler Pass fixtures to use unknown tool input,
tool output, compiler input, and compiler output types. Require stable `AIL006`
diagnostics with source provenance, affected graph items, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unknown_profile_value_types
```

Expected: failure because type validation only inspects `Field` nodes.

- [x] **Step 3: Validate every typed AIL-Core node**

Extend `AIL006` validation to `Field`, `Input`, `Output`, and `Value` nodes.
Keep existing field diagnostic strings stable, add profile built-ins needed by
the Compiler example, and unwrap generic wrappers when suggesting a declaration
for an unknown inner type.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unknown_profile_value_types
cargo test --test ail_toolchain ail_agent_tool_profile_parses_renders_and_checks_refund_tool
cargo test --test ail_toolchain ail_compiler_profile_parses_renders_and_checks_compiler_pass
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: unknown profile value types produce structured `AIL006` diagnostics,
and valid Application, AgentTool, and Compiler fixtures remain accepted.

### Task 38: Multi-Profile Conformance Fixtures

**Files:**
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/refund_tool.ail/examples/accepted/refund-minimal.ail-spec.md`
- Create: `examples/refund_tool.ail/examples/rejected/unknown-input-type.ail-spec.md`
- Create: `examples/compiler_pass.ail/examples/accepted/infer-read-permissions-minimal.ail-spec.md`
- Create: `examples/compiler_pass.ail/examples/rejected/unknown-value-type.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing profile conformance CLI tests**

Require `ail-conformance` on the Refund Tool and Compiler Pass packages to
report accepted and rejected fixtures with structured diagnostics, matching the
Application package conformance shape.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_agent_tool_fixtures
cargo test --test ail_toolchain cli_ail_conformance_checks_compiler_profile_fixtures
```

Expected: failure because the AgentTool and Compiler packages have no
accepted/rejected fixture directories.

- [x] **Step 3: Add profile conformance fixtures**

Add one accepted and one rejected fixture for each profile package. Use `AIL006`
unknown profile type diagnostics for the rejected examples so the fixtures
exercise the same structured diagnostic path as `ail-draft` and Application
conformance.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_agent_tool_fixtures
cargo test --test ail_toolchain cli_ail_conformance_checks_compiler_profile_fixtures
cargo run -- ail-conformance examples/refund_tool.ail
cargo run -- ail-conformance examples/compiler_pass.ail
```

Expected: Application, AgentTool, and Compiler packages all have accepted and
rejected conformance fixtures.

### Task 39: AgentTool Audit Trace Coverage

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/06-agent-tools.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `examples/refund_tool.ail/spec.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/accepted/refund-minimal.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/rejected/unknown-input-type.ail-spec.md`
- Create: `examples/refund_tool.ail/examples/rejected/tool-without-trace.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing AgentTool trace tests**

Require the Refund Tool parser, renderer, AIL-Core graph, AIL-Flow projection,
LLM draft prompt, and conformance output to recognize an explicit `The tool
records:` audit trace section. Add a rejected fixture for a tool without any
tool trace.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: failures because the AgentTool parser ignores `The tool records:`,
the draft prompt omits it, and the no-trace rejected fixture is unexpectedly
accepted.

- [x] **Step 3: Implement tool audit trace coverage**

Parse and render `The tool records:` bullets into `Tool.records_trace` edges,
include the section in the AgentTool draft prompt, and emit stable `AIL017`
diagnostics for tools without an audit trace.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain
cargo run -- ail-conformance examples/refund_tool.ail
```

Expected: the Refund Tool package has explicit trace coverage, and
`tool-without-trace.ail-spec.md` is rejected with `AIL017`.

### Task 40: AgentTool Approval Rules

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/02-structured-spec.md`
- Modify: `docs/ail/06-agent-tools.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/examples/refund-tool.ail-spec.md`
- Modify: `docs/ail/examples/refund-tool.ail-core.md`
- Modify: `examples/refund_tool.ail/spec.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/accepted/refund-minimal.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/rejected/unknown-input-type.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/rejected/tool-without-trace.ail-spec.md`
- Create: `examples/refund_tool.ail/examples/rejected/approval-without-rule.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing AgentTool approval tests**

Require the Refund Tool parser, AIL-Core graph, AIL-Flow projection, canonical
renderer, LLM draft prompt, and conformance output to recognize explicit `The
tool requires approval:` rules. Add a rejected fixture where behavior mentions
approval but the tool has no explicit approval section.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: compile or test failure because `AilTool` has no approval model and
the parser/checker do not know `The tool requires approval:`.

- [x] **Step 3: Implement approval nodes and diagnostics**

Parse and render `The tool requires approval:` bullets, lower them to
`Approval` nodes through `requires_approval` edges, project them in AIL-Flow,
include the section in the AgentTool draft prompt, and emit stable `AIL018`
when a tool mentions approval but has no explicit approval rule.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain
cargo run -- ail-conformance examples/refund_tool.ail
```

Expected: the Refund Tool package has explicit approval rules, and
`approval-without-rule.ail-spec.md` is rejected with `AIL018`.

### Task 41: AgentTool Permission Rules

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/02-structured-spec.md`
- Modify: `docs/ail/06-agent-tools.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/examples/refund-tool.ail-spec.md`
- Modify: `docs/ail/examples/refund-tool.ail-core.md`
- Modify: `examples/refund_tool.ail/spec.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/accepted/refund-minimal.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/rejected/approval-without-rule.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/rejected/tool-without-trace.ail-spec.md`
- Modify: `examples/refund_tool.ail/examples/rejected/unknown-input-type.ail-spec.md`
- Create: `examples/refund_tool.ail/examples/rejected/permission-without-rule.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing AgentTool permission tests**

Require the Refund Tool parser, AIL-Core graph, AIL-Flow projection, canonical
renderer, LLM draft prompt, and conformance output to recognize explicit `The
tool requires permission:` rules. Add a rejected fixture where behavior mentions
permission but the tool has no explicit permission section.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: compile or test failure because `AilTool` has no permission model and
the parser/checker do not know `The tool requires permission:`.

- [x] **Step 3: Implement permission nodes and diagnostics**

Parse and render `The tool requires permission:` bullets, lower them to
`Permission` nodes through `requires` edges, project them in AIL-Flow, include
the section in the AgentTool draft prompt, and emit stable `AIL019` when a tool
mentions permission but has no explicit permission rule.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain
cargo run -- ail-conformance examples/refund_tool.ail
```

Expected: the Refund Tool package has explicit permission rules, and
`permission-without-rule.ail-spec.md` is rejected with `AIL019`.

### Task 42: AgentTool Secret Output Disclosure

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/06-agent-tools.md`
- Modify: `docs/ail/07-types-values-effects.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/refund_tool.ail/examples/rejected/secret-output.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing secret-output tests**

Add a rejected Refund Tool fixture where a tool output has type `Secret<Text>`
and the tool has no explicit reveal or disclose permission. Require stable
`AIL020` diagnostics with output provenance, affected graph node, and repair
guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain
```

Expected: the secret-output fixture is unexpectedly accepted because only field
and input secrets have checker coverage.

- [x] **Step 3: Implement secret output disclosure validation**

Attach secret outputs to `Secret` nodes in AIL-Core, allow output-attached
secret nodes in the secret attachment invariant, and emit `AIL020` when a tool
output type contains `Secret` without a matching reveal or disclose permission.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain
cargo run -- ail-conformance examples/refund_tool.ail
```

Expected: `secret-output.ail-spec.md` is rejected with `AIL020`, while valid
Refund Tool fixtures remain accepted.

### Task 43: AIL-System Resource Capability Slice

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/network_driver.ail/`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing System profile tests**

Add a Network Driver `System` package with a component that declares typed
resources, a device capability, effects, a trace, and a guarantee. Add a
rejected fixture where the component performs an effect without a capability,
and require stable `AIL021` diagnostics with effect provenance, affected graph
edge, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: the parser rejects `System component:` specs because the toolchain
only accepts Application, AgentTool, and Compiler profile declarations.

- [x] **Step 3: Implement System profile parsing and checking**

Add `SystemComponent` and `Resource` document/core nodes, parse component
resource, capability, effect, trace, and guarantee sections, render AIL-Spec
and AIL-Flow projections, teach `ail-draft` the System surface, and emit
`AIL021` when a component performs effects without a declared capability.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: the Network Driver package accepts valid fixtures and rejects
`effect-without-capability.ail-spec.md` with `AIL021`.

### Task 44: AIL-System Effect Resource Tracking

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/network_driver.ail/examples/rejected/unknown-effect-resource.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing System effect-resource tests**

Require valid Network Driver effects such as `read network device` and
`write rx buffer` to create `targets_resource` edges from the `Effect` node to
the declared `Resource` node. Add a rejected fixture where the component has a
capability but performs `read dma ring` without declaring `dma ring`, and
require stable `AIL022` diagnostics with effect provenance, affected graph
node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: valid System effects have no `targets_resource` edge and the
unknown-resource rejected fixture is unexpectedly accepted.

- [x] **Step 3: Implement System effect-resource tracking**

Resolve regular resource-effect verbs to component resources during AIL-Core
elaboration, emit `targets_resource` edges for resolved effects, and emit
`AIL022` when a System effect names a resource that is not declared by the
component.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver effects target declared resources, and
`unknown-effect-resource.ail-spec.md` is rejected with `AIL022`.

### Task 45: AIL-System Device Capability Resource Coverage

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/network_driver.ail/examples/rejected/device-effect-without-matching-capability.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing Device capability coverage tests**

Require valid Network Driver capability `access network device` to create an
`authorizes_resource` edge to `NetworkPacketReceiver.network device`. Add a
rejected fixture where a component has a capability for `rx buffer` but reads
`network device`, and require stable `AIL023` diagnostics with effect
provenance, affected `targets_resource` edge, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: valid capabilities have no `authorizes_resource` edge and the
device-effect rejected fixture is unexpectedly accepted.

- [x] **Step 3: Implement Device capability coverage**

Resolve regular capability verbs to component resources during AIL-Core
elaboration, emit `authorizes_resource` edges for resolved capabilities, and
emit `AIL023` when a System effect targets a `Device` resource that is not
authorized by a matching component capability.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver capabilities authorize their resources, and
`device-effect-without-matching-capability.ail-spec.md` is rejected with
`AIL023`.

### Task 46: AIL-System Mutable Resource Ownership

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `examples/network_driver.ail/spec.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/mutable-effect-without-ownership.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing mutable-resource ownership tests**

Require valid Network Driver ownership of `rx buffer` to render as an
`owns_resource` edge, appear in AIL-Flow, render/reparse through AIL-Spec, and
appear in the System draft prompt. Add a rejected fixture where the component
performs `write rx buffer` without owning `rx buffer`, and require stable
`AIL024` diagnostics with effect provenance, affected `targets_resource` edge,
and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `The component owns:` is not parsed, valid package parsing fails, and
the mutable-resource rejected fixture is accepted.

- [x] **Step 3: Implement mutable-resource ownership**

Parse `The component owns:` sections, emit `owns_resource` edges for ownership
declarations that resolve to component resources, project ownership in
AIL-Flow and AIL-Spec rendering, include it in the System draft prompt, and
emit `AIL024` when a mutable resource effect targets a resource that the
component does not own.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver ownership is preserved across core/flow/spec
projections, and `mutable-effect-without-ownership.ail-spec.md` is rejected
with `AIL024`.

### Task 47: AIL-System Read Borrowing

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Modify: `examples/network_driver.ail/spec.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/read-effect-without-borrow.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing read-borrowing tests**

Require valid Network Driver borrowing of `packet metadata` to render as a
`borrows_resource` edge, appear in AIL-Flow, render/reparse through AIL-Spec,
and appear in the System draft prompt. Add a rejected fixture where the
component performs `read packet metadata` without owning or borrowing
`packet metadata`, and require stable `AIL025` diagnostics with effect
provenance, affected `targets_resource` edge, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: valid borrowing has no `borrows_resource` edge, the System draft
prompt omits `The component borrows:`, and the read-without-borrow rejected
fixture is unexpectedly accepted.

- [x] **Step 3: Implement read borrowing**

Parse `The component borrows:` sections, emit `borrows_resource` edges for
borrowing declarations that resolve to component resources, project borrowing
in AIL-Flow and AIL-Spec rendering, include it in the System draft prompt, and
emit `AIL025` when a non-device read effect targets a resource that the
component neither owns nor borrows.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver borrowing is preserved across core/flow/spec
projections, and `read-effect-without-borrow.ail-spec.md` is rejected with
`AIL025`.

### Task 48: AIL-System Resource Region Placement

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Modify: `examples/network_driver.ail/spec.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/resource-without-region.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing region-placement tests**

Require valid Network Driver placement of `rx buffer` and `packet metadata` in
`packet processing region` to render as a `Region` node, `uses_region` and
`in_region` edges, appear in AIL-Flow, render/reparse through AIL-Spec, and
appear in the System draft prompt. Add a rejected fixture where
`read packet metadata` targets a non-device resource with no region placement,
and require stable `AIL026` diagnostics with effect provenance, affected
`targets_resource` edge, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: valid placement has no `Region` node or region edges, the System
draft prompt omits `The component places:`, and the region rejected fixture is
unexpectedly accepted.

- [x] **Step 3: Implement resource region placement**

Parse `The component places:` sections, emit `Region` nodes plus `uses_region`
and `in_region` edges for resource placements that resolve to component
resources, project regions in AIL-Flow and AIL-Spec rendering, include
placement in the System draft prompt, and emit `AIL026` when a non-device
resource effect targets a resource with no region placement.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver region placement is preserved across
core/flow/spec projections, and `resource-without-region.ail-spec.md` is
rejected with `AIL026`.

### Task 49: AIL-System Borrowed Resource Mutation

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/rejected/mutate-borrowed-resource.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing borrow-checker tests**

Add a rejected Network Driver fixture where `rx buffer` is both owned and
borrowed, then mutated with `write rx buffer`. Require stable `AIL027`
diagnostics with effect provenance, affected `targets_resource` edge, and
repair guidance to remove the borrow or stop mutating the resource.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `mutate-borrowed-resource.ail-spec.md` is unexpectedly accepted and
the focused diagnostic test reports missing `AIL027`.

- [x] **Step 3: Implement borrowed-resource mutation checking**

Emit `AIL027` when a mutable System effect targets a resource currently
declared in `The component borrows:` for that component, even if the component
also owns that resource. This is the first coarse borrow-checking rule before
shorter lifetime scopes exist.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver still passes, and
`mutate-borrowed-resource.ail-spec.md` is rejected with `AIL027`.

### Task 50: AIL-System Use After Release

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Modify: `examples/network_driver.ail/spec.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/use-after-release.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing lifetime tests**

Add `release rx buffer` as the final valid Network Driver effect and require it
to target the declared `rx buffer` resource. Add a rejected fixture that
performs `release rx buffer` and then `read rx buffer`, and require stable
`AIL028` diagnostics with the post-release use provenance, affected
`targets_resource` edge, and repair guidance to move the use before release or
remove it.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `use-after-release.ail-spec.md` is unexpectedly accepted and the
focused diagnostic test reports missing `AIL028`.

- [x] **Step 3: Implement use-after-release checking**

Walk each System component's performed effects in author order. Once a
`release` or `free` effect targets a resource, emit `AIL028` for any later
effect that targets the same resource.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: valid Network Driver release-at-end still passes, and
`use-after-release.ail-spec.md` is rejected with `AIL028`.

### Task 51: AIL-System Mutable Borrow Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/mutable-borrow-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/shared-and-mutable-borrow.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing mutable-borrow tests**

Add an accepted Network Driver fixture where a component mutably borrows
`dma ring` and writes it without owning it. Require a
`mutably_borrows_resource` edge, `mutablyBorrows` in AIL-Flow,
`The component mutably borrows:` in AIL-Spec rendering, and canonical
render/reparse equality. Add a rejected fixture where the same resource is
declared in both `The component borrows:` and `The component mutably borrows:`,
and require stable `AIL029` diagnostics with component provenance, affected
mutable-borrow edge, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: the mutable-borrow accepted fixture reports `AIL024`, the
shared-and-mutable rejected fixture is unexpectedly accepted, and the System
draft prompt omits `The component mutably borrows:`.

- [x] **Step 3: Implement mutable borrow declarations**

Parse `The component mutably borrows:` sections, emit
`mutably_borrows_resource` edges for declarations that resolve to component
resources, project mutable borrowing in AIL-Flow and AIL-Spec rendering,
namespace imported mutable-borrow references, include the surface in the
System draft prompt, allow mutable/read effects against mutably borrowed
resources, and emit `AIL029` when a resource is declared as both shared and
mutable borrowed by the same component.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: mutable borrowing is preserved across core/flow/spec projections,
the accepted fixture passes without ownership, and
`shared-and-mutable-borrow.ail-spec.md` is rejected with `AIL029`.

### Task 52: AIL-System Move Semantics Verification

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/move-resource-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/move-without-ownership.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/use-after-move.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing move-semantics tests**

Add an accepted Network Driver fixture where `move rx buffer` targets an owned
resource and is preserved across AIL-Core, AIL-Flow, and canonical
render/reparse. Add rejected fixtures for moving `rx buffer` without ownership
and for reading `rx buffer` after it has been moved. Require stable diagnostics
for the ownership violation and for post-move use.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `move rx buffer` has no `targets_resource` edge,
`move-without-ownership.ail-spec.md` and `use-after-move.ail-spec.md` are
unexpectedly accepted, and the focused diagnostic tests report missing move
ownership and `AIL030` diagnostics.

- [x] **Step 3: Implement move semantics**

Teach the System effect resolver that `move <resource>` targets the named
resource. Require component ownership before moving a resource, without
allowing mutable borrowing to stand in for ownership. Track moved resources in
component effect order and emit `AIL030` for any later effect targeting the
same resource.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: move effects target resources, moving without ownership is rejected,
and `use-after-move.ail-spec.md` is rejected with `AIL030`.

### Task 53: AIL-System ABI Layout Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/layout-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/layout-unknown-resource.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing ABI layout tests**

Add an accepted Network Driver fixture with `The component lays out:` and a
layout bullet shaped as `packet header: repr(C), align 8`. Require a `Layout`
node, `uses_layout` and `layouts_resource` edges, AIL-Flow layout projection,
AIL-Spec render/reparse equality, and conformance acceptance. Add a rejected
fixture where a layout bullet names undeclared `dma ring`, and require stable
`AIL031` diagnostics with layout provenance, affected layout node, and repair
guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `The component lays out:` is not parsed, layout fixtures report parse
errors, and the focused accepted/rejected tests fail before implementation.

- [x] **Step 3: Implement ABI layout declarations**

Parse `The component lays out:` sections, store resource layout bullets,
elaborate them into `Layout` nodes with `uses_layout` and `layouts_resource`
edges when the resource resolves, project layouts in AIL-Flow and AIL-Spec
rendering, namespace imported layout resource references, include the surface
in the System draft prompt, and emit `AIL031` when a layout declaration names
an undeclared component resource.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: layout declarations are preserved across core/flow/spec projections,
`layout-minimal.ail-spec.md` is accepted, and
`layout-unknown-resource.ail-spec.md` is rejected with `AIL031`.

### Task 54: AIL-System Allocation Placement Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/allocation-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/allocation-unknown-resource.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing allocation placement tests**

Add an accepted Network Driver fixture with `The component allocates:` and an
allocation bullet shaped as `packet buffer: stack`. Require an `Allocation`
node, `uses_allocation` and `allocates_resource` edges, AIL-Flow allocation
projection, AIL-Spec render/reparse equality, and conformance acceptance. Add a
rejected fixture where an allocation bullet names undeclared `dma ring`, and
require stable `AIL032` diagnostics with allocation provenance, affected
allocation node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `The component allocates:` is not parsed, allocation fixtures report
parse errors, and the focused accepted/rejected tests fail before
implementation.

- [x] **Step 3: Implement allocation placement declarations**

Parse `The component allocates:` sections, store resource allocation bullets,
elaborate them into `Allocation` nodes with `uses_allocation` and
`allocates_resource` edges when the resource resolves, project allocations in
AIL-Flow and AIL-Spec rendering, namespace imported allocation resource
references, include the surface in the System draft prompt, and emit `AIL032`
when an allocation declaration names an undeclared component resource.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: allocation declarations are preserved across core/flow/spec
projections, `allocation-minimal.ail-spec.md` is accepted, and
`allocation-unknown-resource.ail-spec.md` is rejected with `AIL032`.

### Task 55: AIL-System Interrupt Context Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/interrupt-context-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/interrupt-context-blocking-effect.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing interrupt context tests**

Add an accepted Network Driver fixture with `The component runs in context:`
and a context bullet shaped as `interrupt`. Require an `ExecutionContext` node,
a `runs_in_context` edge, AIL-Flow context projection, AIL-Spec render/reparse
equality, and conformance acceptance. Add a rejected fixture where an interrupt
context component performs `wait for scheduler`, and require stable `AIL033`
diagnostics with effect provenance, the affected `performs` edge, and repair
guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `The component runs in context:` is not parsed, interrupt-context
fixtures report parse errors, and the System draft prompt lacks the new
surface before implementation.

- [x] **Step 3: Implement interrupt context declarations**

Parse `The component runs in context:` sections, store execution context
bullets, elaborate them into `ExecutionContext` nodes with `runs_in_context`
edges, project contexts in AIL-Flow and AIL-Spec rendering, preserve contexts
across imported namespaces, include the surface in the System draft prompt, and
emit `AIL033` when a component in `interrupt` context performs a blocking
effect such as `wait`, `sleep`, `block`, or `park`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: interrupt context declarations are preserved across core/flow/spec
projections, `interrupt-context-minimal.ail-spec.md` is accepted, and
`interrupt-context-blocking-effect.ail-spec.md` is rejected with `AIL033`.

### Task 56: AIL-System Interrupt Priority Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/interrupt-priority-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/interrupt-priority-unknown-context.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing interrupt priority tests**

Add an accepted Network Driver fixture with
`The component sets interrupt priority:` and a priority bullet shaped as
`interrupt: high`. Require an `InterruptPriority` node,
`uses_interrupt_priority` and `prioritizes_context` edges, AIL-Flow priority
projection, AIL-Spec render/reparse equality, and conformance acceptance. Add a
rejected fixture where an interrupt priority bullet names undeclared
`interrupt`, and require stable `AIL034` diagnostics with priority provenance,
the affected interrupt-priority node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `The component sets interrupt priority:` is not parsed as a priority
surface, the accepted fixture has no `InterruptPriority` node, the rejected
fixture has no `AIL034`, and the System draft prompt lacks the new surface.

- [x] **Step 3: Implement interrupt priority declarations**

Parse `The component sets interrupt priority:` sections, store priority bullets,
elaborate them into `InterruptPriority` nodes with `uses_interrupt_priority`
edges and `prioritizes_context` edges when the context resolves, project
priorities in AIL-Flow and AIL-Spec rendering, preserve priorities across
imported namespaces, include the surface in the System draft prompt, and emit
`AIL034` when a priority declaration names an undeclared component context.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: interrupt priority declarations are preserved across core/flow/spec
projections, `interrupt-priority-minimal.ail-spec.md` is accepted, and
`interrupt-priority-unknown-context.ail-spec.md` is rejected with `AIL034`.

### Task 57: AIL-System Scheduler Task Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/scheduler-task-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/scheduler-task-unknown-context.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing scheduler task tests**

Add an accepted Network Driver fixture with `The component schedules task:` and
a task bullet shaped as `packet poller: process`. Require a `SchedulerTask`
node, `schedules_task` and `task_runs_in_context` edges, AIL-Flow task
projection, AIL-Spec render/reparse equality, and conformance acceptance. Add a
rejected fixture where a scheduler task bullet names undeclared `process`, and
require stable `AIL035` diagnostics with task provenance, the affected
scheduler-task node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: `The component schedules task:` is not parsed as a task surface, the
accepted fixture has no `SchedulerTask` node, the rejected fixture has no
`AIL035`, and the System draft prompt lacks the new surface.

- [x] **Step 3: Implement scheduler task declarations**

Parse `The component schedules task:` sections, store task bullets, elaborate
them into `SchedulerTask` nodes with `schedules_task` edges and
`task_runs_in_context` edges when the context resolves, project tasks in
AIL-Flow and AIL-Spec rendering, preserve tasks across imported namespaces,
include the surface in the System draft prompt, and emit `AIL035` when a task
declaration names an undeclared component context.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: scheduler task declarations are preserved across core/flow/spec
projections, `scheduler-task-minimal.ail-spec.md` is accepted, and
`scheduler-task-unknown-context.ail-spec.md` is rejected with `AIL035`.

### Task 58: AIL-System Scheduler Task Priority Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/scheduler-task-priority-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/scheduler-task-priority-unknown-task.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing scheduler task priority tests**

Add an accepted Network Driver fixture with
`The component sets task priority:` and a priority bullet shaped as
`packet poller: realtime`. Require a `SchedulerTaskPriority` node,
`uses_task_priority` and `prioritizes_task` edges, AIL-Flow task-priority
projection, AIL-Spec render/reparse equality, and conformance acceptance. Add a
rejected fixture where a task-priority bullet names undeclared `packet poller`,
and require stable `AIL036` diagnostics with task-priority provenance, the
affected scheduler-task-priority node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: the accepted fixture has no `SchedulerTaskPriority` node, the
rejected fixture is unexpectedly accepted instead of producing `AIL036`, and
the System draft prompt lacks the new surface.

- [x] **Step 3: Implement scheduler task priority declarations**

Parse `The component sets task priority:` sections, store task-priority
bullets, elaborate them into `SchedulerTaskPriority` nodes with
`uses_task_priority` edges and `prioritizes_task` edges when the scheduler task
resolves, project task priorities in AIL-Flow and AIL-Spec rendering, preserve
task priorities across imported namespaces, include the surface in the System
draft prompt, and emit `AIL036` when a task-priority declaration names an
undeclared scheduler task.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: scheduler task priority declarations are preserved across
core/flow/spec projections, `scheduler-task-priority-minimal.ail-spec.md` is
accepted, and `scheduler-task-priority-unknown-task.ail-spec.md` is rejected
with `AIL036`.

### Task 59: AIL-System Scheduler Task Timing Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/scheduler-task-timing-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/scheduler-task-timing-unknown-task.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing scheduler task timing tests**

Add an accepted Network Driver fixture with
`The component sets task timing:` and a timing bullet shaped as
`packet poller: deadline 10 ms, budget 2 ms`. Require a `SchedulerTaskTiming`
node, `uses_task_timing` and `times_task` edges, AIL-Flow task-timing
projection, AIL-Spec render/reparse equality, and conformance acceptance. Add a
rejected fixture where a task-timing bullet names undeclared `packet poller`,
and require stable `AIL037` diagnostics with task-timing provenance, the
affected scheduler-task-timing node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: the accepted fixture has no `SchedulerTaskTiming` node, the rejected
fixture is unexpectedly accepted instead of producing `AIL037`, and the System
draft prompt lacks the new surface.

- [x] **Step 3: Implement scheduler task timing declarations**

Parse `The component sets task timing:` sections, store deadline and budget
bullets, elaborate them into `SchedulerTaskTiming` nodes with
`uses_task_timing` edges and `times_task` edges when the scheduler task
resolves, project task timings in AIL-Flow and AIL-Spec rendering, preserve
task timings across imported namespaces, include the surface in the System
draft prompt, and emit `AIL037` when a task-timing declaration names an
undeclared scheduler task.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: scheduler task timing declarations are preserved across
core/flow/spec projections, `scheduler-task-timing-minimal.ail-spec.md` is
accepted, and `scheduler-task-timing-unknown-task.ail-spec.md` is rejected with
`AIL037`.

### Task 60: AIL-System Lock Guard Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/lock-guard-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/lock-guard-unknown-lock.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/lock-guard-unknown-resource.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing lock guard tests**

Add an accepted Network Driver fixture with `The component guards:` and a guard
bullet shaped as `scheduler state with scheduler lock`. Require a `LockGuard`
node, `uses_lock_guard`, `guards_resource`, and `uses_lock_resource` edges,
AIL-Flow lock-guard projection, AIL-Spec render/reparse equality, conformance
acceptance, and System draft prompt coverage. Add rejected fixtures where a
lock-guard bullet names an undeclared protected resource or undeclared lock
resource, and require stable `AIL038` and `AIL039` diagnostics with
lock-guard provenance, the affected lock-guard node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: the accepted fixture has no `LockGuard` node, the rejected
unknown-lock fixture is unexpectedly accepted instead of producing `AIL039`,
and the System draft prompt lacks the new surface. Keep the guard section at
the start of the fixture while the parser does not recognize it, so the RED is
about missing lock-guard behavior rather than a stale active subsection.

- [x] **Step 3: Implement lock guard declarations**

Parse `The component guards:` sections, store protected-resource and lock
resource bullets, elaborate them into `LockGuard` nodes with `uses_lock_guard`,
`guards_resource`, and `uses_lock_resource` edges when resources resolve,
project lock guards in AIL-Flow and AIL-Spec rendering, preserve lock guards
across imported namespaces, include the surface in the System draft prompt,
and emit `AIL038`/`AIL039` when a lock guard names undeclared resources.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: lock guard declarations are preserved across core/flow/spec
projections, `lock-guard-minimal.ail-spec.md` is accepted,
`lock-guard-unknown-resource.ail-spec.md` is rejected with `AIL038`, and
`lock-guard-unknown-lock.ail-spec.md` is rejected with `AIL039`.

### Task 61: AIL-System Interrupt Mask Declarations

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/03-semantic-ir.md`
- Modify: `docs/ail/09-system-profile.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Modify: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `examples/network_driver.ail/ail-package.md`
- Create: `examples/network_driver.ail/examples/accepted/interrupt-mask-minimal.ail-spec.md`
- Create: `examples/network_driver.ail/examples/rejected/interrupt-mask-unknown-context.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing interrupt mask tests**

Add an accepted Network Driver fixture with `The component masks interrupt:`
and a mask bullet shaped as `interrupt: mask lower priority interrupts`.
Require an `InterruptMask` node, `uses_interrupt_mask` and `masks_context`
edges, AIL-Flow interrupt-mask projection, AIL-Spec render/reparse equality,
conformance acceptance, and System draft prompt coverage. Add a rejected
fixture where a mask bullet names an undeclared execution context, and require
stable `AIL040` diagnostics with interrupt-mask provenance, the affected
interrupt-mask node, and repair guidance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain system
```

Expected: the accepted fixture has no `InterruptMask` node, the System draft
prompt lacks the new surface, and the rejected unknown-context fixture is
accepted instead of producing `AIL040`.

- [x] **Step 3: Implement interrupt mask declarations**

Parse `The component masks interrupt:` sections, store context-to-mask bullets,
elaborate them into `InterruptMask` nodes with `uses_interrupt_mask` and
`masks_context` edges when contexts resolve, project interrupt masks in
AIL-Flow and AIL-Spec rendering, preserve interrupt masks across imported
namespaces, include the surface in the System draft prompt, and emit `AIL040`
when a mask declaration names an undeclared execution context.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain system
cargo run -- ail-conformance examples/network_driver.ail
```

Expected: interrupt mask declarations are preserved across core/flow/spec
projections, `interrupt-mask-minimal.ail-spec.md` is accepted, and
`interrupt-mask-unknown-context.ail-spec.md` is rejected with `AIL040`.

### Task 62: AIL-Core To AIL-Bytecode Compiler

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing bytecode compiler tests**

Add library tests requiring a checked Application package to lower into an
AIL-Bytecode artifact with package metadata, one action block per AIL action,
requirement opcodes, field-set opcodes, trace opcodes, and declared failure
trace tables. Add a bytecode VM test requiring the compiled `CloseTicket`
action to execute both success and NotFound failure paths. Extend CLI coverage
so `ail-lower` prints bytecode through the checked package path.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode
```

Expected: compile failure because `compile_ail_bytecode`,
`render_ail_bytecode`, and `run_ail_bytecode_action` do not exist.

- [x] **Step 3: Implement AIL bytecode compiler and VM**

Add AIL-native bytecode program, action, instruction, and failure-table
structures. Lower checked Application actions into opcodes for action begin,
requirements, reads, writes, field sets, effects, guarantees, trace emission,
and return. Render deterministic bytecode JSON, add a bootstrap bytecode VM,
route `ail-lower` through the checked package gate, and run `ail-run` through
the bytecode VM instead of direct document execution.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode
cargo test --test ail_toolchain ail_bytecode_vm_executes_close_ticket_success_and_failure
cargo test --test ail_toolchain cli_ail_check_and_core_use_package_loader
```

Expected: checked AIL lowers to deterministic AIL-Bytecode, the bytecode VM
executes `CloseTicket` success and NotFound failure paths, and the CLI exposes
`ail-lower`.

### Task 63: Saved AIL-Bytecode Artifact Execution

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing bytecode artifact tests**

Add a library test requiring `render_ail_bytecode` output to parse back into an
equivalent `AilBytecodeProgram` and execute through the bytecode VM without the
source package. Add a CLI test that saves `ail-lower` output to an `.ailbc.json`
file and runs success and NotFound failure paths through `ail-vm --action`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_bytecode_artifact_parses_and_executes_without_source_package
```

Expected: compile failure because `parse_ail_bytecode` does not exist.

- [x] **Step 3: Implement bytecode artifact parsing and VM CLI**

Add a small dependency-free JSON reader for the deterministic AIL-Bytecode
format, reconstruct actions, instructions, operands, and failure trace tables,
and route `ail-vm` to load a saved bytecode artifact and execute it directly
through the bytecode VM.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_bytecode_artifact_parses_and_executes_without_source_package
cargo test --test ail_toolchain cli_ail_vm_executes_saved_bytecode_artifact
```

Expected: saved AIL-Bytecode reparses to equivalent bytecode, and `ail-vm`
executes the saved artifact's `CloseTicket` success and NotFound failure paths.

### Task 64: Saved AIL-Bytecode Verification

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing bytecode verifier tests**

Add a library test requiring valid bytecode to pass verification and malformed
bytecode to report diagnostics for an unknown opcode and a missing required
operand. Add a CLI test that saves malformed `.ailbc.json` and requires
`ail-vm --action` to print verifier diagnostics without executing the action.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_bytecode_verifier_rejects_invalid_opcodes_and_operands -- --nocapture
```

Expected: compile failure because `verify_ail_bytecode` does not exist.

- [x] **Step 3: Implement bytecode verification**

Add a dependency-free verifier table for known opcodes and required operands.
Gate both `ail-vm` and the library bytecode VM with this verifier so malformed
saved artifacts are rejected before execution.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_bytecode_verifier_rejects_invalid_opcodes_and_operands -- --nocapture
cargo test --test ail_toolchain cli_ail_vm_rejects_invalid_bytecode_before_execution -- --nocapture
```

Expected: valid bytecode has no verifier diagnostics, invalid bytecode reports
`AILBC001` or `AILBC002`, and `ail-vm` returns exit code 1 without printing a
successful execution state.

### Task 65: Prompt-To-Bytecode Application Build

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing prompt-to-bytecode CLI test**

Add a mock llama.cpp chat-completions server test requiring
`eigl ail-build examples/support_ticket.ail --prompt ... --llm-endpoint ...`
to send the user prompt through the AIL draft prompt, accept the returned
AIL-Spec candidate, lower it to AIL-Bytecode, verify it, and print a parseable
bytecode artifact on stdout.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_uses_llm_candidate_and_outputs_verified_bytecode -- --nocapture
```

Expected: failure because `ail-build` is not routed through the AIL package
command path and does not call the LLM endpoint.

- [x] **Step 3: Implement `ail-build`**

Route `ail-build`, accept `--prompt` and `--llm-endpoint`, reuse the existing
LLM draft/check path, reparse the accepted candidate with package imports,
compile it through the AIL bytecode compiler, verify the bytecode, and print
only the bytecode artifact on success.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_uses_llm_candidate_and_outputs_verified_bytecode -- --nocapture
```

Expected: the mock endpoint receives the prompt, stdout parses as
AIL-Bytecode, the verifier reports no diagnostics, and the emitted bytecode
executes the `CloseTicket` action successfully through the bytecode VM.

### Task 66: Requirements-Grounded Prompt-To-Bytecode Build

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing two-stage build test**

Extend the `ail-build` test so the mock llama.cpp server expects two requests:
one requirements-drafting request from the user prompt, and one AIL-Spec
drafting request that contains the generated requirements. Keep the assertion
that stdout parses as verified AIL-Bytecode and executes through the bytecode
VM.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_uses_llm_candidate_and_outputs_verified_bytecode -- --nocapture
```

Expected: failure because `ail-build` only asks the LLM for an AIL-Spec
candidate and never performs the requirements-drafting request.

- [x] **Step 3: Implement requirements grounding**

Add a dependency-free AIL requirements prompt, call it from `ail-build`, feed
the returned requirements into the AIL-Spec draft prompt, and keep the checked
AIL-Core to verified AIL-Bytecode lowering unchanged.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_uses_llm_candidate_and_outputs_verified_bytecode -- --nocapture
```

Expected: `ail-build` sends a requirements prompt, sends a requirements-grounded
AIL-Spec prompt, emits verified AIL-Bytecode, and the emitted bytecode executes
the `CloseTicket` action successfully.

### Task 67: AIL-Authored Toolchain Agent Package

**Files:**
- Create: `examples/ail_toolchain_agent.ail/ail-package.md`
- Create: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `README.md`
- Modify: `docs/ail/README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing toolchain-agent bytecode test**

Add a test requiring `examples/ail_toolchain_agent.ail` to load, check cleanly,
lower to verified AIL-Bytecode, include `CompileApplication`, and execute the
compile action through the bytecode VM.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
```

Expected: failure because the `ail_toolchain_agent.ail` package does not exist.

- [x] **Step 3: Add the AIL-authored toolchain agent**

Add an Application-profile package that models the toolchain agent's developer
interview, requirements capture, spec/IR compilation, bytecode artifact
emission, and prompt-portability comparison responsibilities.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
```

Expected: the package checks cleanly, lowers to verified bytecode, includes
`CompileApplication` and `CompareAgentPromptPortability`, and executes
`CompileApplication` successfully through the bytecode VM.

### Task 68: Compiler-Profile Bytecode Lowering

**Files:**
- Modify: `src/ail.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing Compiler bytecode test**

Add a test requiring `examples/compiler_pass.ail` to load, check cleanly, lower
to verified AIL-Bytecode, include compiler-pass opcodes, and execute
`InferReadPermissions` through the bytecode VM trace path.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: failure because `compile_ail_bytecode` rejects `Compiler` packages
with `ail-lower currently supports Application packages, not Compiler`.

- [x] **Step 3: Implement Compiler bytecode lowering**

Teach the bytecode compiler to lower Compiler-profile compiler passes into
bytecode actions with pass metadata, input/output declarations, reads, steps,
writes, guarantees, traces, and return. Extend bytecode verification and VM
trace handling for the new pass opcodes.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: the Compiler package lowers to verified bytecode and the VM executes
`InferReadPermissions` successfully with pass and trace events.

### Task 69: AgentTool-Profile Bytecode Lowering

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing AgentTool bytecode test**

Add an AgentTool lowering test for `examples/refund_tool.ail` requiring
`RefundCustomerPayment` to lower to verified AIL-Bytecode, include explicit
tool opcodes for requirements, inputs, outputs, external calls, permissions,
approvals, secret protections, guarantees, and traces, and execute through the
bytecode VM.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_agent_tool_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: failure because `compile_ail_bytecode` rejects `AgentTool` packages
with `ail-lower currently supports Application and Compiler packages, not
AgentTool`.

- [x] **Step 3: Implement AgentTool bytecode lowering**

Teach the bytecode compiler to lower AgentTool-profile tool declarations into
bytecode actions with tool metadata, requirements, typed inputs and outputs,
reads, calls, writes, permissions, approvals, secret protections, guarantees,
traces, and return. Extend bytecode verification and VM trace handling for the
new tool opcodes.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_agent_tool_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: the AgentTool package lowers to verified bytecode and the VM executes
`RefundCustomerPayment` successfully with tool and trace events.

### Task 70: System-Profile Bytecode Lowering

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing System bytecode test**

Add a System lowering test for `examples/network_driver.ail` requiring
`NetworkPacketReceiver` to lower to verified AIL-Bytecode, include explicit
system opcodes for component metadata, resources, ownership, borrowing,
regions, capabilities, effects, guarantees, and traces, and execute through the
bytecode VM.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_system_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: failure because `compile_ail_bytecode` rejects `System` packages
with `ail-lower currently supports Application, AgentTool, and Compiler
packages, not System`.

- [x] **Step 3: Implement System bytecode lowering**

Teach the bytecode compiler to lower System-profile component declarations into
bytecode actions with component metadata, resources, ownership and borrow
relations, regions, layouts, allocations, lock guards, execution contexts,
interrupt configuration, scheduler tasks, capabilities, effects, guarantees,
traces, and return. Extend bytecode verification and VM trace handling for the
new system opcodes.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_system_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: the System package lowers to verified bytecode and the VM executes
`NetworkPacketReceiver` successfully with system and trace events.

### Task 71: Profile-Aware `ail-build` Requirements Prompt

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing AgentTool `ail-build` prompt test**

Add an `ail-build` test for `examples/refund_tool.ail` with a two-response mock
LLM. Require the first AIL-Requirements prompt to name actions, tools, compiler
passes, and system components as possible profile surfaces; require the second
prompt to use the AgentTool AIL-Spec surface and the final output to parse as
verified AgentTool AIL-Bytecode.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_for_agent_tool_profile_prompts_tool_requirements_and_outputs_bytecode -- --nocapture
```

Expected: failure because the requirements prompt still says `actions or
compiler surfaces`, which under-specifies AgentTool and System packages.

- [x] **Step 3: Implement profile-aware requirements wording**

Update the AIL-Requirements prompt to name actions, tools, compiler passes, and
system components directly while preserving the two-stage
requirements-to-AIL-Spec-to-AIL-Core-to-AIL-Bytecode build flow.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_for_agent_tool_profile_prompts_tool_requirements_and_outputs_bytecode -- --nocapture
```

Expected: `ail-build` sends a profile-aware requirements prompt, sends an
AgentTool-shaped AIL-Spec prompt grounded in those requirements, and prints
verified AgentTool AIL-Bytecode.

### Task 72: `ail-build` Diagnostics-Guided Repair Pass

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing repair-loop test**

Add an `ail-build` test where the mock base LLM first returns requirements,
then returns a rejected AIL-Spec candidate, then returns a repaired accepted
candidate. Require the third prompt to include the previous candidate, the
draft requirements, detailed checker diagnostics, source provenance, affected
graph item, and repair suggestion; require the command to emit verified
AIL-Bytecode from the repaired candidate.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_repairs_rejected_candidate_before_lowering -- --nocapture
```

Expected: failure because `ail-build` stops after the rejected candidate and
does not send a third diagnostics-guided repair request.

- [x] **Step 3: Implement one repair pass**

Add a repair prompt that preserves the original human request and
AIL-Requirements, includes the rejected AIL-Spec candidate and detailed checker
diagnostics, calls the base LLM once more, and rechecks the repaired candidate
before lowering. Keep the output target as verified AIL-Bytecode only.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_repairs_rejected_candidate_before_lowering -- --nocapture
```

Expected: `ail-build` sends requirements, initial AIL-Spec, and repair prompts,
then emits verified AIL-Bytecode from the repaired candidate.

### Task 73: `ail-build` Intermediate Artifact Output

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing artifact-output test**

Add an `ail-build --artifact-dir <dir>` test that keeps stdout as parseable
AIL-Bytecode while writing the captured AIL-Requirements, accepted AIL-Spec,
checked AIL-Core IR, and final AIL-Bytecode artifact to deterministic files.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
```

Expected: failure because `--artifact-dir` is not accepted and the command
never reaches the mock base LLM.

- [x] **Step 3: Implement artifact output**

Parse `--artifact-dir` for `ail-build`, create the directory, write
`requirements.ail-requirements.md`, `accepted.ail-spec.md`,
`checked.ail-core.txt`, and `artifact.ailbc.json` after the spec checks and
bytecode verifies, and keep stdout as the same bytecode text for pipeline
compatibility.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
```

Expected: the command emits verified bytecode on stdout and writes the four
reviewable intermediate artifacts.

### Task 74: AIL-Core IR To Bytecode Entry Point

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing AIL-Core bytecode test**

Add a library test that parses the Support Ticket package, elaborates and
checks AIL-Core, then calls `compile_ail_core_bytecode` directly. Require the
rendered bytecode to include the package, action, requirement opcodes, and to
execute `CloseTicket` through the bytecode VM.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_core_ir_to_bytecode -- --nocapture
```

Expected: compile failure because `compile_ail_core_bytecode` does not exist.

- [x] **Step 3: Implement checked IR bytecode lowering**

Add `compile_ail_core_bytecode` as the bytecode compiler entrypoint, reconstruct
the bytecode lowering view from checked AIL-Core graph nodes and edges, keep
`compile_ail_bytecode` as a source-document compatibility wrapper through
AIL-Core elaboration, and route `ail-lower`, `ail-run`, and `ail-build` through
the checked AIL-Core entrypoint.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_core_ir_to_bytecode -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain ail_bytecode_vm_executes_close_ticket_success_and_failure -- --nocapture
cargo test --test ail_toolchain ail_agent_tool_profile_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain ail_compiler_profile_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain ail_system_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: checked AIL-Core lowers to verified AIL-Bytecode for Application,
AgentTool, Compiler, and System profiles, and the bytecode VM execution path
stays unchanged.

### Task 75: `ail-build` Requirements Coverage Repair Gate

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing requirements-repair test**

Add an `ail-build` test where the first base LLM response is an
AIL-Requirements artifact with too little coverage. Require the command to send
a requirements-repair prompt with stable diagnostics before asking for
AIL-Spec, then continue to verified AIL-Bytecode after repaired requirements.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_repairs_incomplete_requirements_before_spec_drafting -- --nocapture
```

Expected: failure because request 2 is still AIL-Spec drafting instead of
requirements repair.

- [x] **Step 3: Implement requirements coverage diagnostics**

Add `check_ail_requirements` with profile-specific coverage diagnostics for
application, agent-tool, compiler, and system requirements. Add
`repair_ail_requirements_from_diagnostics` and route `ail-build` through one
requirements repair pass before spec drafting. If requirements still fail,
print `ail-build requirements diagnostics:` and exit nonzero.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build -- --nocapture
```

Expected: incomplete requirements are repaired before spec drafting, existing
build flows still emit verified bytecode, and requirements fixtures include
explicit data, behavior, failure, guarantee, trace, and profile-specific
coverage.

### Task 76: Compiler-Pass Bytecode Transforms Checked AIL-Core

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing compiler-pass transform test**

Add a library test that compiles `examples/compiler_pass.ail` to
AIL-Bytecode, parses Support Ticket into checked AIL-Core, runs
`InferReadPermissions` bytecode over that AIL-Core, and requires the result to
contain a candidate read `Permission` node attached to the action that reads
`Ticket.status`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_pass_bytecode_transforms_checked_core_ir -- --nocapture
```

Expected: compile failure because `run_ail_compiler_pass_on_core` does not
exist.

- [x] **Step 3: Implement compiler-pass core runner**

Add `run_ail_compiler_pass_on_core` for Compiler-profile bytecode. Execute the
pass through the existing bytecode VM, detect AIL-authored passes that add
candidate read permissions, and transform checked AIL-Core by adding
`Permission` nodes, `requires` edges, and compiler-pass provenance for
non-secret read edges.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_pass_bytecode_transforms_checked_core_ir -- --nocapture
cargo test --test ail_toolchain ail_compiler_profile_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain ail_compiler_profile_parses_renders_and_checks_compiler_pass -- --nocapture
```

Expected: the AIL-authored `InferReadPermissions` pass now transforms checked
AIL-Core while preserving existing Compiler-profile parse, render, bytecode,
and VM trace behavior.

### Task 77: Compiler-Pass IR Transform Bytecode Primitive

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing transform-opcode tests**

Extend Compiler-profile bytecode tests to require
`CORE_INFER_READ_PERMISSIONS` in the rendered bytecode and require
`run_ail_compiler_pass_on_core` to leave AIL-Core unchanged when that opcode is
removed from a pass that still has prose `PASS_WRITE` text.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_pass -- --nocapture
```

Expected: failure because compiler-pass bytecode has no explicit IR-transform
opcode and the runner still triggers read-permission inference from prose.

- [x] **Step 3: Lower and execute explicit transform opcodes**

Emit `CORE_INFER_READ_PERMISSIONS` for the AIL-Meta read-permission inference
pass, add verifier operand coverage for the opcode, trace it in the bytecode
VM, and route `run_ail_compiler_pass_on_core` through that opcode instead of
matching prose in `PASS_WRITE`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_pass -- --nocapture
cargo test --test ail_toolchain ail_compiler_profile_lowers_to_verified_bytecode -- --nocapture
```

Expected: compiler-pass bytecode exposes an explicit IR-transform primitive,
the runner transforms only when that primitive is present, and existing
Compiler-profile bytecode verification still passes.

### Task 78: Compiler-Pass CLI Toolchain Stage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-pass` CLI test**

Add a CLI test that runs:

```bash
eigl ail-pass examples/compiler_pass.ail examples/support_ticket.ail --action InferReadPermissions
```

Require stdout to be transformed Support Ticket AIL-Core containing
`Permission read Ticket.status`, the `requires` edge from
`MarksOverdueTickets`, and compiler-pass provenance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_runs_compiler_pass_over_checked_package_core -- --nocapture
```

Expected: failure because `ail-pass` is not a recognized command.

- [x] **Step 3: Implement `ail-pass`**

Parse `ail-pass <compiler-pass-package> <target-package> --action <PassName>`.
Load and check the compiler-pass package, compile it to verified AIL-Bytecode,
load and check the target package as AIL-Core, run the selected pass bytecode
over the checked target core, re-check the transformed core, and print the
deterministic transformed AIL-Core artifact.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_runs_compiler_pass_over_checked_package_core -- --nocapture
```

Expected: `ail-pass` succeeds and prints only the transformed checked AIL-Core,
making AIL-authored compiler passes available as a CLI toolchain stage.

### Task 79: `ail-pass` Intermediate Artifact Output

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-pass --artifact-dir` test**

Add a CLI test that runs `ail-pass` with `--artifact-dir <dir>` and requires
stdout to remain the transformed AIL-Core while the artifact directory contains
`pass.ailbc.json`, `input.ail-core.txt`, `output.ail-core.txt`, and
`trace.txt`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: failure because `--artifact-dir` is still restricted to `ail-build`.

- [x] **Step 3: Implement pass artifact writing**

Allow `--artifact-dir` for `ail-pass`, create the directory, and write the
compiled pass bytecode, checked target input core, transformed output core, and
compiler-pass VM trace. Keep stdout as the transformed output core so the
command remains pipeline-friendly.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: the artifact files are written, output core matches stdout, pass
bytecode verifies, and trace records pass start, transform opcode, and
permission insertion.

### Task 80: `ail-pass` Runs Saved Compiler-Pass Bytecode

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-bytecode `ail-pass` test**

Add a CLI test that first runs:

```bash
eigl ail-lower examples/compiler_pass.ail
```

Save stdout as a `.ailbc.json` artifact, then run:

```bash
eigl ail-pass <saved-pass.ailbc.json> examples/support_ticket.ail --action InferReadPermissions
```

Require stdout to contain transformed Support Ticket AIL-Core with
`Permission read Ticket.status` and the `requires` edge from
`MarksOverdueTickets`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_accepts_saved_compiler_pass_bytecode_artifact -- --nocapture
```

Expected: failure because `ail-pass` treats the saved bytecode file path as an
AIL package directory.

- [x] **Step 3: Implement saved pass bytecode loading**

Teach `ail-pass` to accept either an AIL-Meta package directory or a saved
Compiler-profile AIL-Bytecode artifact as its first argument. For a file input,
read and parse bytecode directly, verify it, and run it over the checked target
AIL-Core without loading the compiler-pass source package.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_accepts_saved_compiler_pass_bytecode_artifact -- --nocapture
```

Expected: saved compiler-pass bytecode applies to the target package and emits
the same transformed AIL-Core as source-package pass execution.

### Task 81: `ail-build` Runs Compiler Pass Before Bytecode Lowering

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-build --pass` pipeline test**

Add an `ail-build` test that uses the mock llama.cpp chat endpoint to produce
requirements and an accepted Support Ticket AIL-Spec, runs:

```bash
eigl ail-build examples/support_ticket.ail --prompt "Build an AIL support ticket bytecode artifact" --pass examples/compiler_pass.ail --artifact-dir <dir> --llm-endpoint <mock>
```

Require stdout to remain verified AIL-Bytecode while
`checked.ail-core.txt` contains `Permission read Ticket.status`, the
`requires` edge from `MarksOverdueTickets`, and compiler-pass provenance.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_compiler_pass_before_bytecode_lowering -- --nocapture
```

Expected: failure before any mock LLM request because `--pass` is not accepted
by `ail-build`.

- [x] **Step 3: Implement build-pass integration**

Parse `--pass` for `ail-build`, accept either a compiler-pass package directory
or saved Compiler-profile AIL-Bytecode artifact, verify the pass bytecode,
require exactly one compiler-pass action for automatic build integration, run
that action over the checked candidate AIL-Core, re-check the transformed IR,
and lower the post-pass IR into verified AIL-Bytecode.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_compiler_pass_before_bytecode_lowering -- --nocapture
```

Expected: `ail-build --pass` writes the post-pass checked AIL-Core artifact,
stdout remains parseable verified AIL-Bytecode, and final bytecode lowering
uses the transformed IR instead of the pre-pass candidate.

### Task 82: `ail-build --pass` Writes Auditable Pass Artifacts

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing build-pass artifact test**

Extend the `ail-build --pass --artifact-dir` test to require `pass.ailbc.json`
and `pass-trace.txt` alongside the existing requirements, spec, checked core,
and final bytecode artifacts.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_compiler_pass_before_bytecode_lowering -- --nocapture
```

Expected: failure because `ail-build --pass --artifact-dir` only writes the
final post-pass core and bytecode, not the pass bytecode or pass trace.

- [x] **Step 3: Implement build-pass artifact writing**

Carry the compiler-pass bytecode text and pass VM trace from `ail-build --pass`
through artifact writing. When a pass ran, write `pass.ailbc.json` and
`pass-trace.txt`; when no pass ran, keep the existing four-file artifact set.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_compiler_pass_before_bytecode_lowering -- --nocapture
```

Expected: `ail-build --pass --artifact-dir` writes the pass bytecode and trace,
the pass bytecode verifies, and the trace records pass start, transform opcode,
and permission insertion.

### Task 83: Requirements Capture CLI Stage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-requirements` CLI test**

Add a CLI test that runs:

```bash
eigl ail-requirements examples/support_ticket.ail --prompt "Capture requirements for a support ticket app" --llm-endpoint <mock>
```

The mock base LLM first returns incomplete requirements, then repaired
requirements. Require the command to print only the checked AIL-Requirements
artifact and never ask for an AIL-Spec candidate.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_requirements_repairs_incomplete_capture_before_printing -- --nocapture
```

Expected: failure before any mock LLM request because `ail-requirements` is not
a recognized AIL command.

- [x] **Step 3: Implement checked requirements capture**

Route `ail-requirements` through package loading, `--prompt`, optional
`--llm-endpoint`, AIL-Requirements drafting, profile-specific requirements
coverage checking, and one diagnostics-guided repair pass. Refactor `ail-build`
to reuse the same checked requirements helper before spec drafting.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_requirements_repairs_incomplete_capture_before_printing -- --nocapture
```

Expected: the command sends a draft requirements prompt, sends one repair prompt
when coverage is incomplete, prints the repaired AIL-Requirements artifact, and
does not proceed to AIL-Spec drafting.

### Task 84: Requirements-To-Spec CLI Stage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-spec` CLI test**

Add a CLI test that writes a checked AIL-Requirements artifact to a temp file
and runs:

```bash
eigl ail-spec examples/support_ticket.ail --prompt "Draft a support ticket app from captured requirements" --requirements-file <requirements-file> --llm-endpoint <mock>
```

The mock base LLM first returns a rejected AIL-Spec and then a repaired
candidate. Require stdout to be only accepted AIL-Spec, not bytecode or command
diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_spec_drafts_and_repairs_from_checked_requirements_file -- --nocapture
```

Expected: failure before any mock LLM request because `ail-spec` is not a
recognized AIL command.

- [x] **Step 3: Implement requirements-to-spec command**

Route `ail-spec` through package loading, `--prompt`,
`--requirements-file`, optional `--llm-endpoint`, requirements-file validation,
requirements-grounded AIL-Spec drafting, one diagnostics-guided repair pass,
and checked AIL-Spec stdout. Refactor `ail-build` to reuse the same
requirements-to-spec helper before AIL-Core and bytecode lowering.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_spec_drafts_and_repairs_from_checked_requirements_file -- --nocapture
```

Expected: the command validates the checked requirements file, sends a
requirements-grounded spec prompt, sends one repair prompt when the first
candidate is rejected, and prints accepted AIL-Spec that reparses and checks
cleanly.

### Task 85: Saved AIL-Spec Artifact Input For IR And Bytecode

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `--spec-file` CLI test**

Add a CLI test that writes a valid generated AIL-Spec artifact to a temp file
and requires:

```bash
eigl ail-core examples/support_ticket.ail --spec-file <spec-file>
eigl ail-lower examples/support_ticket.ail --spec-file <spec-file>
```

to render checked AIL-Core and verified AIL-Bytecode from that saved spec file
without modifying the package entry spec.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_core_and_lower_accept_saved_spec_file_artifact -- --nocapture
```

Expected: usage failure because `--spec-file` is not accepted by AIL package
commands.

- [x] **Step 3: Implement saved spec-file input**

Parse `--spec-file` for `ail-check`, `ail-core`, `ail-flow`, `ail-lower`, and
`ail-run`. When supplied, read and parse that AIL-Spec text against the package
metadata; otherwise keep using the package entry spec. Leave `ail-patch`,
`ail-build`, conformance, and compiler-pass package loading on their existing
entry-spec paths.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_core_and_lower_accept_saved_spec_file_artifact -- --nocapture
```

Expected: saved AIL-Spec artifacts render AIL-Core and lower to verified
AIL-Bytecode while preserving package metadata.

### Task 86: Saved AIL-Core Artifact Input For Bytecode

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-core lowering test**

Add a CLI test that renders `ail-core` for `examples/support_ticket.ail`, saves
that checked AIL-Core text to a temp file, reparses it as AIL-Core, and requires:

```bash
eigl ail-lower examples/support_ticket.ail --core-file <core-file>
```

to emit the same verified AIL-Bytecode as direct source-package lowering,
including write payloads that produce `SET_FIELD` instructions.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_accepts_saved_core_file_artifact -- --nocapture
```

Expected: compile failure because `parse_ail_core_text` and `--core-file` do
not exist.

- [x] **Step 3: Serialize and parse compiler-significant core edges**

Expose `parse_ail_core_text`, add edge attributes to `render_ail_core`, keep
serialized edges in graph order so execution-relevant requirement/read/write
ordering survives the artifact boundary, and route `ail-lower --core-file`
through parse, `check_ail_core`, `compile_ail_core_bytecode`, and bytecode
verification without loading the source package spec.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_accepts_saved_core_file_artifact -- --nocapture
```

Expected: saved AIL-Core artifacts parse cleanly, check cleanly, and lower to
the same verified AIL-Bytecode as the source package.

### Task 87: Saved AIL-Core Artifact Input For Compiler Passes

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-core pass test**

Add a CLI test that compiles `examples/compiler_pass.ail` to a saved
Compiler-profile AIL-Bytecode artifact, renders `examples/support_ticket.ail`
to a saved checked AIL-Core artifact, and requires:

```bash
eigl ail-pass <saved-pass.ailbc.json> --core-file <core-file> --action InferReadPermissions
```

to run the pass over the saved IR artifact without requiring a target source
package argument.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_accepts_saved_core_file_artifact -- --nocapture
```

Expected: usage failure because `ail-pass` requires a positional target package
and rejects `--core-file`.

- [x] **Step 3: Implement saved target-core input**

Allow `--core-file` for `ail-pass`, make the positional target package optional
when that flag is present, parse and check the saved AIL-Core artifact, and run
the selected Compiler-profile bytecode pass over that IR. Preserve existing
source-package target behavior and `--artifact-dir` outputs.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_accepts_saved_core_file_artifact -- --nocapture
```

Expected: saved compiler-pass bytecode transforms the saved target AIL-Core
artifact, writes auditable pass artifacts when requested, and prints the
transformed AIL-Core artifact on stdout.

### Task 88: Saved AIL-Requirements Artifact Input For Build

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-requirements build test**

Add a CLI test that writes a checked AIL-Requirements artifact to a temp file
and requires:

```bash
eigl ail-build examples/support_ticket.ail --prompt "Build from saved requirements" --requirements-file <requirements-file> --artifact-dir <dir> --llm-endpoint <mock>
```

to skip requirements capture, send exactly one requirements-grounded AIL-Spec
draft request, emit verified AIL-Bytecode, and write build artifacts containing
the saved requirements.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_accepts_saved_requirements_file_artifact -- --nocapture
```

Expected: usage failure because `--requirements-file` is accepted for
`ail-spec` but not `ail-build`.

- [x] **Step 3: Implement saved requirements input**

Allow `--requirements-file` for `ail-build`, share checked requirements-file
loading with `ail-spec`, skip the requirements-capture LLM call when the file is
present, and continue through spec drafting, AIL-Core checking, optional build
pass execution, and bytecode lowering.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_accepts_saved_requirements_file_artifact -- --nocapture
```

Expected: `ail-build --requirements-file` emits verified bytecode, writes the
saved requirements into `requirements.ail-requirements.md`, and makes no
requirements-capture LLM request.

### Task 89: Saved AIL-Spec Artifact Input For Build

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-spec build test**

Add a CLI test that writes a checked AIL-Spec artifact to a temp file and
requires:

```bash
eigl ail-build examples/support_ticket.ail --spec-file <spec-file> --artifact-dir <dir>
```

to skip requirements capture and spec drafting, emit verified AIL-Bytecode, and
write accepted spec, checked core, and bytecode artifacts without writing a
requirements artifact.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_accepts_saved_spec_file_artifact -- --nocapture
```

Expected: usage failure because `--spec-file` is accepted by lower-level AIL
commands but not by `ail-build`.

- [x] **Step 3: Implement saved spec input**

Allow `--spec-file` for `ail-build`, parse the saved AIL-Spec artifact against
the package metadata, skip all LLM calls, check the elaborated AIL-Core, and
continue through optional compiler-pass execution and bytecode lowering.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_accepts_saved_spec_file_artifact -- --nocapture
```

Expected: `ail-build --spec-file` emits verified bytecode, writes
`accepted.ail-spec.md`, `checked.ail-core.txt`, and `artifact.ailbc.json`, and
does not write `requirements.ail-requirements.md` for a skipped requirements
stage.

### Task 90: Saved AIL-Core Artifact Input For Build

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-core build test**

Add a CLI test that writes a checked AIL-Core artifact to a temp file and
requires:

```bash
eigl ail-build examples/support_ticket.ail --core-file <core-file> --artifact-dir <dir>
```

to skip requirements capture and spec drafting, emit verified AIL-Bytecode, and
write checked core plus bytecode artifacts without writing requirements or
accepted-spec artifacts.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_accepts_saved_core_file_artifact -- --nocapture
```

Expected: usage failure because `--core-file` is accepted by `ail-lower` and
`ail-pass` but not by `ail-build`.

- [x] **Step 3: Implement saved core input**

Allow `--core-file` for `ail-build`, parse and check the saved AIL-Core
artifact before loading the source package, skip all LLM and AIL-Spec stages,
and continue through optional compiler-pass execution and bytecode lowering.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_accepts_saved_core_file_artifact -- --nocapture
```

Expected: `ail-build --core-file` emits verified bytecode, writes
`checked.ail-core.txt` and `artifact.ailbc.json`, and does not write
`requirements.ail-requirements.md` or `accepted.ail-spec.md` for skipped stages.

### Task 91: AIL-Authored Toolchain Agent Build Stage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing build-agent test**

Add a CLI test that runs:

```bash
eigl ail-build examples/support_ticket.ail --core-file <core-file> --agent examples/ail_toolchain_agent.ail --artifact-dir <dir>
```

and requires `ail-build` to compile the AIL-authored toolchain agent into
AIL-Bytecode, run the `CompileApplication` action, write `agent.ailbc.json` and
`agent-trace.txt`, and keep stdout as the target package's verified bytecode.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_toolchain_agent_bytecode -- --nocapture
```

Expected: usage failure because `--agent` is not accepted by `ail-build`.

- [x] **Step 3: Implement build-agent bytecode execution**

Parse `--agent` for `ail-build`, accept an AIL Application-profile package or
saved bytecode artifact, verify the agent bytecode, require a
`CompileApplication` action, run that action against the completed build state,
and write the agent bytecode plus trace artifacts when `--artifact-dir` is
present.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_toolchain_agent_bytecode -- --nocapture
```

Expected: `ail-build --agent` emits the target package bytecode, writes verified
agent bytecode, and records a trace with the agent's
`ApplicationBytecodeCompiled` event.

### Task 92: AIL Build Agent Requirements-Capture Stage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing build-agent capture test**

Add a prompt-driven `ail-build --agent` CLI test where the base LLM returns
requirements and a valid spec. Require `agent-trace.txt` to record the
AIL-authored agent's `CaptureRequirements` action before its
`CompileApplication` action.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_records_requirements_capture_before_compile -- --nocapture
```

Expected: failure because `ail-build --agent` only runs `CompileApplication`
after bytecode generation.

- [x] **Step 3: Implement capture action execution**

When `ail-build` captures requirements from a prompt, thread that prompt into
the agent runner, require `CaptureRequirements`, execute it against the
developer prompt and build request state, then execute `CompileApplication` over
the completed build state. Keep saved-requirements, saved-spec, and saved-core
resumes on the compile-only agent path.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_records_requirements_capture_before_compile -- --nocapture
```

Expected: the agent trace records `CaptureRequirements`, status
`RequirementsCaptured`, then `CompileApplication` and status `BytecodeReady`.

### Task 93: AIL Build Agent Prompt-Portability Stage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing build-agent portability test**

Add a prompt-driven `ail-build --agent --target-model <name>` CLI test where
the base LLM returns requirements and a valid spec. Require `agent-trace.txt`
to record the AIL-authored agent's `CompareAgentPromptPortability` action after
requirements capture and before `CompileApplication`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_compares_prompt_portability_before_compile -- --nocapture
```

Expected: failure because `--target-model` is not accepted by `ail-build`.

- [x] **Step 3: Implement target-model agent execution**

Parse `--target-model` for `ail-build`, require `--agent` when it is supplied,
thread the target model into the build-agent runner, require
`CompareAgentPromptPortability`, execute it over the requirements-aware build
state, and append its trace before `CompileApplication`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_compares_prompt_portability_before_compile -- --nocapture
```

Expected: the agent trace records `CaptureRequirements`,
`CompareAgentPromptPortability`, `AgentPromptPortabilityCompared`, and then
`ApplicationBytecodeCompiled`.

### Task 94: AIL Build Agent Capture Preflight

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing agent-preflight test**

Add a prompt-driven `ail-build --agent` CLI test that uses an Application
package without `CaptureRequirements` as the build agent and a mock base LLM
endpoint. Require the command to fail with the capture-action diagnostic before
the mock endpoint receives any request.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_capture_failure_happens_before_llm_request -- --nocapture
```

Expected: failure because the current command sends the base LLM requirements
request before validating or running the AIL-authored agent.

- [x] **Step 3: Implement early capture preflight**

For prompt-driven builds with `--agent` and no saved requirements file, load and
verify the agent before requirements drafting, require `CaptureRequirements`,
run that bytecode action against the developer prompt, and thread the resulting
agent state and trace into the later compile/portability stage.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_capture_failure_happens_before_llm_request -- --nocapture
```

Expected: the command fails before any LLM request when the agent cannot run
`CaptureRequirements`.

### Task 95: AIL Build Agent Compile Preflight

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing compile-preflight test**

Add an `ail-build --core-file --agent` CLI test with a target core that would
fail bytecode lowering and an Application-profile agent that lacks
`CompileApplication`. Require the command to fail with the missing-agent-action
diagnostic before the target lowering error appears.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_compile_failure_happens_before_bytecode_lowering -- --nocapture
```

Expected: failure because target bytecode lowering currently runs before the
AIL-authored build agent's `CompileApplication` action is validated.

- [x] **Step 3: Implement compile preflight**

After core checking and any AIL compiler pass, run the build agent's
`CompileApplication` action before target bytecode lowering. Keep stdout as the
target bytecode artifact after the Rust bootstrap compiler emits verified
AIL-Bytecode.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_compile_failure_happens_before_bytecode_lowering -- --nocapture
```

Expected: the invalid agent fails before the unsupported target profile reaches
bytecode lowering.

### Task 96: AIL Build Agent Bytecode Verification Stage

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bytecode-verification test**

Add a prompt-driven `ail-build --agent` CLI test that requires
`agent-trace.txt` to record `VerifyBytecodeArtifact` after
`CompileApplication`, read the emitted bytecode artifact, write a bytecode
verification report, and record `BytecodeArtifactVerified`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: failure because the agent trace currently stops after
`ApplicationBytecodeCompiled`.

- [x] **Step 3: Implement post-emission verification**

Extend the AIL-authored toolchain agent with `VerifyBytecodeArtifact`, preserve
the agent bytecode program and state across `CompileApplication`, and after the
Rust bootstrap compiler emits and verifies the target AIL-Bytecode artifact,
run `VerifyBytecodeArtifact` and append its trace.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: the agent trace records `VerifyBytecodeArtifact` after
`CompileApplication` and includes `BytecodeArtifactVerified`.

### Task 97: AIL Build Agent Requirements Prompt Context

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing prompt-context test**

Add a prompt-driven `ail-build --agent` CLI test that requires the first base
LLM requirements request to include an `AGENT REQUIREMENTS CONTEXT` section and
the AIL agent's `buildrequest.requirements coverage checklist=Prepared` state.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_threads_capture_checklist_into_requirements_prompt -- --nocapture
```

Expected: failure because the current command runs `CaptureRequirements` before
the LLM call but does not include the resulting agent state in the requirements
prompt.

- [x] **Step 3: Implement prompt context threading**

Extend the AIL-authored toolchain agent with a requirements coverage checklist
field written by `CaptureRequirements`. Render the preflight agent state as a
small requirements context block and pass that grounded prompt to both the
initial requirements draft and the requirements repair pass. Tighten the
requirements prompts to require the exact `AIL-Requirements:` header and `- `
bullet shape expected by the checker.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_threads_capture_checklist_into_requirements_prompt -- --nocapture
```

Expected: the first base LLM requirements request includes the agent context and
the agent trace records the checklist state write before compilation.

### Task 98: AIL Build Agent Spec Prompt Context

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing spec-prompt context test**

Add a prompt-driven `ail-build --agent` CLI test that requires the second base
LLM request, the AIL-Spec draft prompt, to include an `AGENT SPEC CONTEXT`
section and the AIL agent's `buildrequest.spec coverage checklist=Prepared`
state. Require the agent trace to record `PrepareSpecDraft` before
`CompileApplication`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_threads_spec_checklist_into_spec_prompt -- --nocapture
```

Expected: failure because the command drafts AIL-Spec directly after
requirements checking without running an AIL-authored spec-preparation action or
including agent state in the spec prompt.

- [x] **Step 3: Implement spec prompt context threading**

Extend the AIL-authored toolchain agent with a `spec coverage checklist` field
and `PrepareSpecDraft` action. After requirements are checked, run that action
against the captured build state, append its trace, and pass the resulting
checklist context to both initial AIL-Spec drafting and spec repair.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_threads_spec_checklist_into_spec_prompt -- --nocapture
```

Expected: the AIL-Spec prompt includes `AGENT SPEC CONTEXT`, and the agent trace
records `PrepareSpecDraft` before `CompileApplication`.

### Task 99: AIL Build Agent Accepts Checked Spec Drafts

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing accepted-spec agent test**

Add a prompt-driven `ail-build --agent` CLI test that requires the agent trace
to record `AcceptSpecDraft` before `CompileApplication`, read the checked
`BuildRequest` spec, write `buildrequest.spec review report=Accepted`, change
status to `SpecCaptured`, and record `SpecDraftAccepted`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_spec_draft_before_compile -- --nocapture
```

Expected: failure because the command currently jumps from `PrepareSpecDraft` to
`CompileApplication`; the host accepts the checked spec without an AIL-authored
state transition.

- [x] **Step 3: Implement accepted-spec bytecode checkpoint**

Extend the AIL-authored toolchain agent with a `spec review report` field and
`AcceptSpecDraft` action. After AIL-Spec drafting succeeds, run that action with
the checked requirements and accepted spec before parsing and elaborating to
AIL-Core, then pass the resulting agent state into `CompileApplication`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_spec_draft_before_compile -- --nocapture
```

Expected: the agent trace records `AcceptSpecDraft`, `SpecCaptured`, and
`SpecDraftAccepted` before `CompileApplication`.

### Task 100: AIL Build Agent Accepts Checked AIL-Core IR

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing accepted-core agent test**

Add a prompt-driven `ail-build --agent` CLI test that requires the agent trace
to record `AcceptCoreIR` after `AcceptSpecDraft` and before
`CompileApplication`, read the checked `BuildRequest` core ir, write
`buildrequest.core review report=Accepted`, change status to `CoreChecked`, and
record `CoreIrAccepted`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_checked_core_before_compile -- --nocapture
```

Expected: failure because the command currently lets `CompileApplication` be
the first AIL bytecode action to observe checked AIL-Core.

- [x] **Step 3: Implement accepted-core bytecode checkpoint**

Extend the AIL-authored toolchain agent with a `core review report` field and
`AcceptCoreIR` action. After AIL-Core checking and any compiler pass, render the
checked core, run that action with the captured build state, and pass the
resulting `CoreChecked` state into `CompileApplication`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_checked_core_before_compile -- --nocapture
```

Expected: the agent trace records `AcceptCoreIR`, `CoreChecked`, and
`CoreIrAccepted` before `CompileApplication`.

### Task 101: AIL Build Agent Accepts Saved AIL-Core Artifacts

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-core agent test**

Add an `ail-build --core-file --agent` CLI test that requires the agent trace to
record `AcceptCoreIR` before `CompileApplication`, read the loaded
`BuildRequest` core ir, write `buildrequest.core review report=Accepted`,
change status to `CoreChecked`, and record `CoreIrAccepted`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_saved_core_before_compile -- --nocapture
```

Expected: failure because saved-core agent builds currently jump directly from
the host-loaded AIL-Core artifact to `CompileApplication`.

- [x] **Step 3: Implement saved-core agent acceptance**

Add a `CoreLoaded` status for loaded checked-core artifacts, allow
`AcceptCoreIR` to accept `SpecCaptured` or `CoreLoaded`, synthesize an agent
`BuildRequest` state for saved-core builds, and run `AcceptCoreIR` before
`CompileApplication`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_saved_core_before_compile -- --nocapture
```

Expected: saved-core builds record `AcceptCoreIR`, `CoreChecked`, and
`CoreIrAccepted` before `CompileApplication`.

### Task 102: AIL Build Agent Accepts Saved AIL-Spec Artifacts

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-spec agent test**

Add an `ail-build --spec-file --agent` CLI test that requires the agent trace
to record `AcceptSpecDraft` before `AcceptCoreIR` and `CompileApplication`,
read the loaded `BuildRequest` spec, write
`buildrequest.spec review report=Accepted`, change status to `SpecCaptured`,
and record `SpecDraftAccepted`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_saved_spec_before_core_lowering -- --nocapture
```

Expected: failure because saved-spec agent builds currently start agent
execution at `AcceptCoreIR` after AIL-Core elaboration.

- [x] **Step 3: Implement saved-spec agent acceptance**

Add a `SpecLoaded` status for loaded checked-spec artifacts, allow
`AcceptSpecDraft` to accept `RequirementsCaptured` or `SpecLoaded`, synthesize
an agent `BuildRequest` state for saved-spec builds, and run `AcceptSpecDraft`
before AIL-Core elaboration.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_saved_spec_before_core_lowering -- --nocapture
```

Expected: saved-spec builds record `AcceptSpecDraft`, `SpecCaptured`, and
`SpecDraftAccepted` before `AcceptCoreIR` and `CompileApplication`.

### Task 103: AIL Build Agent Prepares Saved AIL-Requirements Artifacts

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-requirements agent test**

Add an `ail-build --requirements-file --agent` CLI test that requires the
single base LLM spec prompt to include the AIL agent's spec checklist context,
then requires the agent trace to record `PrepareSpecDraft` before
`AcceptSpecDraft`, `AcceptCoreIR`, and `CompileApplication`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_prepares_saved_requirements_before_spec_drafting -- --nocapture
```

Expected: failure because saved-requirements agent builds currently pass the
checked requirements directly to the base LLM spec prompt without running
`PrepareSpecDraft`.

- [x] **Step 3: Implement saved-requirements agent preparation**

Add a `RequirementsLoaded` status for loaded checked-requirements artifacts,
allow `PrepareSpecDraft` and `AcceptSpecDraft` to accept that state, synthesize
an agent `BuildRequest` state for saved-requirements builds, run
`PrepareSpecDraft` before the base LLM spec prompt, and run `AcceptSpecDraft`
after the generated spec checks.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_prepares_saved_requirements_before_spec_drafting -- --nocapture
```

Expected: saved-requirements builds record `PrepareSpecDraft`,
`SpecDraftPrepared`, `AcceptSpecDraft`, and `SpecDraftAccepted` before
`AcceptCoreIR` and `CompileApplication`.

### Task 104: AIL Build Agent Accepts Compiler-Pass Output

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing compiler-pass agent test**

Add an `ail-build --pass --agent` CLI test that requires the AIL-authored
compiler pass bytecode and trace to be written, then requires the build-agent
trace to record `AcceptCompilerPassOutput` after `AcceptSpecDraft` and before
`AcceptCoreIR` and `CompileApplication`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_compiler_pass_output_before_core -- --nocapture
```

Expected: failure because `ail-build --pass --agent` currently runs the pass
bytecode but the build-agent trace jumps from `AcceptSpecDraft` to
`AcceptCoreIR`.

- [x] **Step 3: Implement compiler-pass output acceptance**

Extend the AIL-authored toolchain agent with compiler-pass artifact, trace, and
review-report fields plus an `AcceptCompilerPassOutput` action. After an
AIL-authored compiler pass transforms checked AIL-Core and the transformed core
re-checks, pass the compiler-pass bytecode boundary and VM trace into that
agent action before `AcceptCoreIR`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_compiler_pass_output_before_core -- --nocapture
```

Expected: `AcceptCompilerPassOutput`, `PassApplied`, and
`CompilerPassOutputAccepted` appear before `AcceptCoreIR` and
`CompileApplication`.

### Task 105: AIL Build Agent Verifies Bytecode Fingerprint

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bytecode-fingerprint test**

Extend the `ail-build --agent --artifact-dir` verification test to require the
AIL-authored build-agent trace to read `BuildRequest.bytecode fingerprint`, and
require the artifact directory to include `artifact.fingerprint.txt` matching
the deterministic fingerprint of stdout bytecode.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: failure because `VerifyBytecodeArtifact` only reads the bytecode
artifact summary and `ail-build --artifact-dir` writes no bytecode fingerprint
artifact.

- [x] **Step 3: Implement dependency-free bytecode fingerprinting**

Add a `BuildRequest.bytecode fingerprint` field to the AIL-authored toolchain
agent and require `VerifyBytecodeArtifact` to read it. Compute a stable FNV-1a
fingerprint over the emitted bytecode text in the Rust bootstrap, pass it into
the AIL bytecode agent action, and write `artifact.fingerprint.txt` beside
`artifact.ailbc.json`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: the agent trace reads `buildrequest.bytecode fingerprint`, the final
verification trace still records `BytecodeArtifactVerified`, and the fingerprint
artifact matches stdout bytecode.

### Task 106: AIL Build Agent Accepts Compiler-Pass Fingerprints

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing compiler-pass fingerprint test**

Extend the `ail-build --pass --agent --artifact-dir` test to require
`pass.fingerprint.txt` beside `pass.ailbc.json`, with the same deterministic
FNV-1a fingerprint format as final bytecode, and require the AIL-authored build
agent trace to read `BuildRequest.compiler pass fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_compiler_pass_output_before_core -- --nocapture
```

Expected: failure because `ail-build --pass --artifact-dir` writes the pass
bytecode and trace but no pass fingerprint, and `AcceptCompilerPassOutput` does
not read one.

- [x] **Step 3: Implement compiler-pass fingerprint propagation**

Compute a stable fingerprint over the compiler-pass bytecode text, write
`pass.fingerprint.txt` when pass artifacts are emitted, add
`BuildRequest.compiler pass fingerprint` to the AIL-authored toolchain agent,
and pass the fingerprint into `AcceptCompilerPassOutput` before `AcceptCoreIR`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_compiler_pass_output_before_core -- --nocapture
```

Expected: the pass fingerprint artifact matches `pass.ailbc.json`, the agent
trace reads `buildrequest.compiler pass fingerprint`, and compiler-pass output
acceptance still precedes `AcceptCoreIR`.

### Task 107: Standalone AIL Pass Writes Bytecode Fingerprint

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing standalone pass fingerprint test**

Extend the `ail-pass --artifact-dir` test to require `pass.fingerprint.txt`
beside `pass.ailbc.json`, using the same deterministic FNV-1a fingerprint
format as `ail-build` pass artifacts.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: failure because standalone `ail-pass --artifact-dir` writes the pass
bytecode, input core, output core, and trace, but no pass fingerprint.

- [x] **Step 3: Implement standalone pass fingerprint output**

Reuse the dependency-free bytecode fingerprint helper in the `ail-pass`
artifact writer and write `pass.fingerprint.txt` whenever `pass.ailbc.json` is
written.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: `pass.fingerprint.txt` matches `pass.ailbc.json` while stdout remains
the transformed AIL-Core artifact.

### Task 108: AIL Build Writes Agent Bytecode Fingerprint

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing agent fingerprint test**

Extend the `ail-build --agent --artifact-dir` test to require
`agent.fingerprint.txt` beside `agent.ailbc.json`, using the same deterministic
FNV-1a fingerprint format as final and compiler-pass bytecode artifacts.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_toolchain_agent_bytecode -- --nocapture
```

Expected: failure because `ail-build --agent --artifact-dir` writes
`agent.ailbc.json` and `agent-trace.txt` but no agent bytecode fingerprint.

- [x] **Step 3: Implement agent fingerprint output**

Reuse the dependency-free bytecode fingerprint helper in the `ail-build`
artifact writer and write `agent.fingerprint.txt` whenever `agent.ailbc.json`
is written.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_runs_toolchain_agent_bytecode -- --nocapture
```

Expected: `agent.fingerprint.txt` matches `agent.ailbc.json`, the agent
bytecode still verifies, and the agent trace still records `CompileApplication`.

### Task 109: AIL Build Writes Artifact Manifest

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing manifest test**

Extend the `ail-build --pass --agent --artifact-dir` test to require
`manifest.ail-build.txt`. The manifest must list the requirements, accepted
spec, checked core, final bytecode fingerprint, compiler-pass bytecode
fingerprint, compiler-pass trace, agent bytecode fingerprint, and agent trace.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_compiler_pass_output_before_core -- --nocapture
```

Expected: failure because `ail-build --artifact-dir` writes individual
review artifacts and fingerprints but no manifest tying them together.

- [x] **Step 3: Implement manifest output**

Add a deterministic `manifest.ail-build.txt` renderer to the `ail-build`
artifact writer. Keep stdout unchanged and write the manifest after the
individual artifacts, using the emitted final fingerprint and deterministic
fingerprints for pass and agent bytecode artifacts.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_accepts_compiler_pass_output_before_core -- --nocapture
```

Expected: the manifest lists the review artifacts, traces, and bytecode
fingerprints while the existing pass and agent acceptance flow still verifies.

### Task 110: Standalone AIL Pass Writes Artifact Manifest

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing pass manifest test**

Extend the `ail-pass --artifact-dir` test to require
`manifest.ail-pass.txt`. The manifest must list the pass bytecode fingerprint,
input core artifact, output core artifact, and pass execution trace.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: failure because standalone `ail-pass --artifact-dir` writes the
individual artifacts and pass fingerprint but no manifest tying them together.

- [x] **Step 3: Implement pass manifest output**

Add a deterministic `manifest.ail-pass.txt` renderer to the standalone pass
artifact writer. Keep stdout unchanged and write the manifest after the pass
bytecode, pass fingerprint, input core, output core, and trace artifacts.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: the manifest lists the pass bytecode fingerprint, core artifacts, and
execution trace while stdout remains the transformed AIL-Core artifact.

### Task 111: AIL Build Agent Verifies Build Manifest

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing AIL-agent manifest verification tests**

Extend the toolchain-agent bytecode test to require a `VerifyBuildManifest`
action. Extend the `ail-build --agent --artifact-dir` test to require the agent
trace to run `VerifyBuildManifest` after `VerifyBytecodeArtifact`, read the
artifact manifest and manifest fingerprint, write an accepted manifest
verification report, and emit `BuildManifestVerified`. Require
`manifest.fingerprint.txt` to match `manifest.ail-build.txt`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: the package test first fails because the AIL-authored toolchain agent
has no `VerifyBuildManifest` action; after the agent spec is extended, the CLI
test fails because the Rust bootstrap does not yet call that action or write a
manifest fingerprint artifact.

- [x] **Step 3: Implement build manifest verification**

Add manifest fields and a `Verify build manifest` action to the AIL-authored
toolchain agent. Render the build manifest before artifact writing, compute its
deterministic fingerprint, run `VerifyBuildManifest` when `--agent` and
`--artifact-dir` are both present, and write `manifest.fingerprint.txt` beside
`manifest.ail-build.txt`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: the agent bytecode includes and runs `VerifyBuildManifest`, the trace
records manifest verification after bytecode verification, and the manifest
fingerprint file matches the manifest artifact.

### Task 112: Standalone AIL Pass Writes Manifest Fingerprint

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing pass manifest fingerprint test**

Extend the `ail-pass --artifact-dir` test to require
`manifest.fingerprint.txt` beside `manifest.ail-pass.txt`, using the same
deterministic FNV-1a fingerprint format as bytecode artifacts.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: failure because standalone `ail-pass --artifact-dir` writes the pass
manifest but no manifest fingerprint.

- [x] **Step 3: Implement pass manifest fingerprint output**

Reuse the dependency-free fingerprint helper in the `ail-pass` artifact writer
and write `manifest.fingerprint.txt` whenever `manifest.ail-pass.txt` is
written.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
```

Expected: `manifest.fingerprint.txt` matches `manifest.ail-pass.txt` while
stdout remains the transformed AIL-Core artifact.

### Task 113: Standalone AIL Pass Runs AIL Agent Acceptance

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-pass --agent` test**

Add a CLI test that runs `ail-pass` with `--agent
examples/ail_toolchain_agent.ail` and `--artifact-dir <dir>`. Require stdout to
remain transformed AIL-Core, require `agent.ailbc.json`,
`agent.fingerprint.txt`, and `agent-trace.txt`, require the agent trace to run
`AcceptCompilerPassOutput`, and require `manifest.ail-pass.txt` to index the
agent bytecode and trace.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_agent_accepts_pass_artifacts -- --nocapture
```

Expected: failure because `--agent` is accepted only by `ail-build`, so
standalone `ail-pass` cannot yet route pass artifacts through an AIL bytecode
agent.

- [x] **Step 3: Implement standalone pass agent acceptance**

Allow `--agent` for `ail-pass`, load and verify the AIL-authored Application
agent, require `AcceptCompilerPassOutput`, run that bytecode action against the
transformed core, pass bytecode fingerprint, and pass VM trace, and write the
agent bytecode, fingerprint, and trace artifacts when `--artifact-dir` is
present. Extend the standalone pass manifest to include agent bytecode and
trace entries.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_agent_accepts_pass_artifacts -- --nocapture
```

Expected: `ail-pass --agent` succeeds, stdout remains transformed AIL-Core, the
agent trace records `AcceptCompilerPassOutput`, and the manifest fingerprints
the pass-plus-agent artifact set.

### Task 114: AIL Pass Agent Verifies Pass Manifest

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing pass manifest verification tests**

Extend the toolchain-agent bytecode test to require a `VerifyPassManifest`
action. Extend the `ail-pass --agent --artifact-dir` test to require the agent
trace to run `VerifyPassManifest` after `AcceptCompilerPassOutput`, read the
pass manifest and manifest fingerprint, write an accepted manifest verification
report, and emit `PassManifestVerified`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_pass_agent_accepts_pass_artifacts -- --nocapture
```

Expected: the package test first fails because the AIL-authored toolchain agent
has no `VerifyPassManifest` action; after the agent spec is extended, the CLI
test fails because standalone `ail-pass --agent` accepts pass output but does
not verify the pass manifest.

- [x] **Step 3: Implement pass manifest verification**

Add a `Verify pass manifest` action to the AIL-authored toolchain agent. When
`ail-pass` is run with both `--agent` and `--artifact-dir`, render the pass
manifest, compute its deterministic fingerprint, run `VerifyPassManifest`, and
persist the resulting trace through `agent-trace.txt`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_pass_agent_accepts_pass_artifacts -- --nocapture
```

Expected: the agent bytecode includes and runs `VerifyPassManifest`, and the
standalone pass agent trace records manifest verification after pass output
acceptance.

### Task 115: AIL Lower Writes Auditable Bytecode Artifacts

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing `ail-lower --artifact-dir` test**

Extend the saved-core `ail-lower` test to run with `--artifact-dir <dir>`.
Require stdout to remain parseable AIL-Bytecode, and require
`checked.ail-core.txt`, `artifact.ailbc.json`, `artifact.fingerprint.txt`,
`manifest.ail-lower.txt`, and `manifest.fingerprint.txt`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_accepts_saved_core_file_artifact -- --nocapture
```

Expected: failure because `--artifact-dir` is accepted for `ail-build` and
`ail-pass`, but not for direct `ail-lower`.

- [x] **Step 3: Implement lower artifact output**

Allow `--artifact-dir` for `ail-lower`. Write the checked AIL-Core input,
lowered bytecode artifact, deterministic bytecode fingerprint, lower manifest,
and manifest fingerprint for both source-package and `--core-file` lowering
paths while keeping stdout unchanged.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_accepts_saved_core_file_artifact -- --nocapture
```

Expected: `ail-lower --core-file --artifact-dir` writes the auditable direct
IR-to-bytecode artifact set and stdout still equals `artifact.ailbc.json`.

### Task 116: AIL Compile Emits Native Linux ELF Executables

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native executable test**

Add a Linux x86_64 CLI test for `ail-compile <package> --action CloseTicket
--target linux-x86_64-elf --out <path>`. Require the output file to have ELF
magic, ELFCLASS64, little-endian encoding, `EM_X86_64`, executable permission,
and a successful process exit when run.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_emits_runnable_linux_x86_64_elf_executable -- --nocapture
```

Expected: failure because `ail-compile`, `--target`, and `--out` do not exist
yet.

- [x] **Step 3: Implement direct ELF emission**

Add `compile_ail_core_native_elf` as the first native compiler backend. It
checks the requested target, validates the selected action through the existing
checked AIL-Core to VM-instruction lowering path, emits ELF64 bytes directly,
writes them with executable permissions, and uses direct Linux x86_64 syscall
machine code without Rust codegen, libc, a linker, LLVM, or an assembler.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_emits_runnable_linux_x86_64_elf_executable -- --nocapture
```

Expected: the native output is a runnable Linux x86_64 ELF executable and exits
successfully.

### Task 117: Native ELF Enforces First AIL Requirements

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native requirement test**

Extend native ELF coverage with a `CloseTicket` executable run under three
argv states: `ticket.id=T-1 ticket.status=Open` must exit successfully,
missing `ticket.id` must fail, and `ticket.status=Closed` must fail.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_close_ticket_requirements -- --nocapture
```

Expected: failure because the first ELF slice exits `0` without inspecting
runtime state.

- [x] **Step 3: Generate native argv requirement checks**

Translate supported `REQUIRE_EXISTS` instructions into native argv prefix
searches for `key=`, and supported `REQUIRE_FIELD_NOT_EQUALS` instructions
into native argv exact-match rejection for the forbidden `key=value`. Keep the
ELF writer direct and dependency-free: no generated Rust, no libc, no linker,
no LLVM, and no assembler.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_close_ticket_requirements -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_emits_runnable_linux_x86_64_elf_executable -- --nocapture
```

Expected: native `CloseTicket` exits `0` for an open ticket, exits nonzero for
missing `ticket.id`, exits nonzero for a closed ticket, and remains a valid
runnable Linux x86_64 ELF executable.

### Task 118: Native ELF Emits First AIL State Writes

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native state-write test**

Extend native ELF coverage so successful `CloseTicket` execution with
`ticket.id=T-1 ticket.status=Open` must print `ticket.status=Closed` to
stdout, while failed requirement execution must print nothing.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_close_ticket_state_write -- --nocapture
```

Expected: failure because the native executable enforces requirements but does
not yet emit compiled `SET_FIELD` writes.

- [x] **Step 3: Generate native stdout writes**

Translate supported `SET_FIELD` instructions into embedded `key=value\n`
strings and direct Linux x86_64 `write(1, ...)` syscalls on the success path.
Keep failure paths silent and keep the ELF writer dependency-free.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_close_ticket_state_write -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_close_ticket_requirements -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_emits_runnable_linux_x86_64_elf_executable -- --nocapture
```

Expected: native `CloseTicket` emits `ticket.status=Closed` only on successful
execution and preserves requirement exit-status behavior and ELF validity.

### Task 119: Native ELF Enforces Field Allow-List Requirements

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing field allow-list test**

Compile `AssignTicket` to native ELF and run it with `ticket.status=Open`,
`ticket.status=New`, missing `ticket.status`, and `ticket.status=Closed`.
Require allowed statuses to exit successfully and emit `ticket.status=Assigned`,
and require missing or disallowed statuses to exit nonzero with no stdout.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
```

Expected: failure because the native executable ignores `REQUIRE_FIELD_IN` and
continues to the success/write path for missing or disallowed field values.

- [x] **Step 3: Generate native allow-list checks**

Translate supported `REQUIRE_FIELD_IN` instructions into exact argv
allow-list searches for `key=value`, using the VM instruction artifact's
encoded value list as the native compiler input. Continue only when at least
one allowed value is present.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_close_ticket_state_write -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_close_ticket_requirements -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_emits_runnable_linux_x86_64_elf_executable -- --nocapture
```

Expected: native `AssignTicket` enforces `New or Open` before emitting
`ticket.status=Assigned`, while existing native `CloseTicket` behavior remains
unchanged.

### Task 120: `ail-build` Emits Native ELF Targets

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-spec build test**

Add an `ail-build --spec-file` test that requests `AssignTicket` with
`--target linux-x86_64-elf --out <path>`. Require the command to write a native
ELF executable, avoid printing the VM artifact on stdout, and require the
executable to enforce `ticket.status=Open` before emitting
`ticket.status=Assigned`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_saved_spec_can_emit_native_linux_x86_64_elf -- --nocapture
```

Expected: usage failure because `ail-build` rejects `--action`, `--target`,
and `--out`.

- [x] **Step 3: Route native output through `ail-build`**

Allow `ail-build` to parse `--action`, `--target`, and `--out` together. After
requirements/spec/core checking and any build pass, call the native ELF emitter
for the selected action and write the executable path instead of printing the
VM artifact.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_saved_spec_can_emit_native_linux_x86_64_elf -- --nocapture
cargo test --test ail_toolchain cli_ail_build_accepts_saved_spec_file_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
```

Expected: saved-spec `ail-build` can emit a native Linux x86_64 ELF executable
for `AssignTicket` while the default saved-spec build still emits the verified
VM artifact and existing native requirement enforcement remains unchanged.

### Task 121: Toolchain Agent Verifies Native Target Artifacts

**Files:**
- Modify: `src/main.rs`
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native-agent trace test**

Add an `ail-build --spec-file --agent --target linux-x86_64-elf` test that
writes a native executable and requires `agent-trace.txt` to record
`VerifyTargetArtifact`, read the emitted target artifact summary and
fingerprint, write a target artifact verification report, and trace
`TargetArtifactVerified`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: failure because the agent trace stops at `VerifyBytecodeArtifact` and
`VerifyBuildManifest`; no native target artifact verification action exists.

- [x] **Step 3: Add target artifact verification**

Extend the AIL-authored toolchain agent with `VerifyTargetArtifact` and target
artifact fields. Compute a deterministic fingerprint over emitted ELF bytes,
run the new agent action for native builds, and only then write the executable
to disk.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
```

Expected: native `ail-build --agent` records target artifact verification after
`CompileApplication`, and the AIL-authored agent still lowers to verified
bytecode with the new action.

### Task 122: Native Build Artifacts Are Manifested

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native manifest test**

Add an `ail-build --spec-file --target linux-x86_64-elf --artifact-dir` test
that requires the artifact directory to contain `target.elf`,
`target.fingerprint.txt`, and a `manifest.ail-build.txt` target entry tying the
native executable bytes to the deterministic fingerprint.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_target_is_in_artifact_manifest -- --nocapture
```

Expected: failure because the native ELF is written only to `--out`; the build
artifact directory has no `target.elf` and the manifest does not index the
native target.

- [x] **Step 3: Persist native target artifacts**

Extend the build artifact set with the selected target name and emitted
executable bytes. Write `target.elf`, mark it executable, write
`target.fingerprint.txt`, and include `target linux-x86_64-elf target.elf
<fingerprint>` in `manifest.ail-build.txt`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_target_is_in_artifact_manifest -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
```

Expected: native builds persist and manifest the target ELF while agent-native
verification and the existing VM artifact directory path remain unchanged.

### Task 123: LLM-Style Positive Field Requirements Compile

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Probe real Qwen native build behavior**

Run a prompt-driven `ail-build --agent --target linux-x86_64-elf` through
`http://inteligentia-pro-1:8080/v1/chat/completions`. The generated
requirements and spec include `the ticket status is Open`, but the bytecode
lowers it as `OBSERVE_RULE`, so the native executable accepts
`ticket.status=Closed`.

- [x] **Step 2: Write failing LLM-style requirement test**

Add a saved-spec native build test that rewrites the CloseTicket requirement to
`the ticket status is Open`. Require CloseTicket bytecode to contain
`REQUIRE_FIELD_IN` for that rule, require `ticket.status=Open` to succeed, and
require `ticket.status=Closed` to fail without stdout.

- [x] **Step 3: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_executable_enforces_llm_style_is_field_requirement -- --nocapture
```

Expected: failure because CloseTicket contains `OBSERVE_RULE` for
`the ticket status is Open`, and native execution accepts `ticket.status=Closed`.

- [x] **Step 4: Compile `is <value>` field requirements**

Extend positive field requirement detection and requirement-field diagnostics to
recognize `<field> is <value>` as the same allow-list shape as
`<field> to be <value>`.

- [x] **Step 5: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_executable_enforces_llm_style_is_field_requirement -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_close_ticket_state_write -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics -- --nocapture
```

Expected: LLM-style `is` requirements compile into enforceable
`REQUIRE_FIELD_IN` bytecode/native checks while existing `to be` and
`not to be` requirement behavior remains unchanged.

### Task 124: Agent Manifest Verification Reads Native Target Fingerprints

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native manifest-agent trace test**

Extend the native `ail-build --spec-file --agent --target linux-x86_64-elf`
test so `VerifyBuildManifest` must run after `VerifyTargetArtifact` and must
read `buildrequest.target artifact fingerprint` during the manifest
verification action, not only during target artifact verification.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: failure because the AIL-authored agent's `VerifyBuildManifest` action
reads only the build manifest, manifest fingerprint, and bytecode fingerprint.

- [x] **Step 3: Extend the AIL-authored manifest verifier**

Update `examples/ail_toolchain_agent.ail` so `VerifyBuildManifest` reads
`BuildRequest.target artifact fingerprint` and its guarantee names native target
artifacts alongside requirements, spec, AIL-Core, compiler-pass, agent, and
bytecode artifacts.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
```

Expected: native build-agent traces show manifest verification reads the native
target fingerprint, and the updated AIL-authored agent still lowers to verified
bytecode.

### Task 125: Native ELF Emits AIL Trace Events

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native trace test**

Add native ELF coverage so successful `CloseTicket` execution with
`ticket.id=T-1 ticket.status=Open` must keep state writes on stdout and emit
`trace TicketClosed` to stderr, while failed requirement execution must emit no
stdout or stderr.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
```

Expected: failure because the native executable emits `SET_FIELD` stdout but
does not yet emit compiled `EMIT_TRACE` events.

- [x] **Step 3: Generate native stderr traces**

Translate supported `EMIT_TRACE` instructions into embedded
`trace <EventName>\n` strings and direct Linux x86_64 `write(2, ...)`
syscalls on the success path. Keep stdout reserved for parseable `SET_FIELD`
state writes and keep failure paths silent.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_close_ticket_state_write -- --nocapture
```

Expected: native `CloseTicket` emits `ticket.status=Closed` on stdout and
`trace TicketClosed` on stderr only after requirements pass, while existing
state-write behavior remains unchanged.

### Task 126: Native ELF Rejects Unsupported VM Opcodes

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing unsupported-opcode native test**

Compile the `network_driver.ail` System package to
`--target linux-x86_64-elf` and require failure with an unsupported native
opcode diagnostic for `SYSTEM_BEGIN`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unsupported_system_opcodes -- --nocapture
```

Expected: failure because the native backend silently ignores System VM opcodes
and writes a runnable ELF.

- [x] **Step 3: Guard native opcode coverage**

Keep supported Application opcodes and explicit no-op Application metadata
opcodes in the native lowering boundary. Reject unsupported System, AgentTool,
CompilerPass, or future unknown VM opcodes with a stable native target error
instead of emitting a partial executable.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unsupported_system_opcodes -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
```

Expected: native System compilation is rejected with an unsupported-opcode
diagnostic while supported Application native actions continue to compile and
run.

### Task 127: Native ELF Emits Success Semantic Traces

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing full semantic trace test**

Extend native `CloseTicket` coverage so successful execution must write the VM
success trace entries to stderr in order: action start, passed requirements,
state write, effect, guarantee check, and explicit trace event. Keep stdout as
`ticket.status=Closed` and keep failed requirement execution silent.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
```

Expected: failure because the native backend emits only `trace TicketClosed`
and drops the other supported Application semantic trace entries.

- [x] **Step 3: Emit success-path semantic traces**

During native lowering, collect supported Application trace entries from
`ACTION_BEGIN`, requirement pass opcodes, `OBSERVE_RULE`, reads, writes,
effects, guarantees, and `EMIT_TRACE`, then emit them with direct Linux x86_64
`write(2, ...)` syscalls after requirements pass. Preserve silent failure paths
for this slice.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unsupported_system_opcodes -- --nocapture
```

Expected: native `CloseTicket` stderr mirrors supported VM success trace
entries while unsupported non-Application native opcodes still fail at compile
time.

### Task 128: Native ELF Emits Requirement Failure Traces

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native failure trace test**

Extend native `CloseTicket` coverage so closed-ticket execution emits
`action CloseTicket started`, the passed ticket-exists rule, and
`failure RequirementFailed` to stderr, and missing-ticket execution emits
`action CloseTicket started`, `failure NotFound`, and `trace TicketNotFound`.
Stdout must remain empty on both failure paths.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
```

Expected: failure because native requirement failures exit nonzero with empty
stderr.

- [x] **Step 3: Generate per-requirement failure branches**

Pass the bytecode failure table into native lowering. For each supported
requirement check, generate a distinct failure label that writes the VM-style
trace prefix, failure name, and declared failure trace events to stderr before
exiting `1`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_executable_emits_trace_to_stderr -- --nocapture
```

Expected: native `CloseTicket` now emits VM-style semantic traces for both
success and supported requirement failure paths while keeping failure stdout
empty.

### Task 129: Nested Field Requirements Compile

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing nested requirement tests**

Require the Support Ticket `AssignTicket` bytecode to lower
`the assignee role to be SupportAgent or SupportManager` into
`REQUIRE_FIELD_IN` on `ticket.assignee.role`. Extend native `AssignTicket`
coverage so successful execution must supply `ticket.assignee.role`, and
missing or `Customer` roles fail without stdout.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
```

Expected: failure because `assignee role` currently lowers to `OBSERVE_RULE`
and native execution accepts missing assignee-role input.

- [x] **Step 3: Resolve nested typed field phrases**

Teach field-reference resolution to follow declared typed fields through
wrappers such as `Option<T>`, `List<T>`, and `Secret<T>`. Phrases like
`assignee role` now resolve through `Ticket.assignee: Option<User>` to the
explicit runtime key `ticket.assignee.role`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
```

Expected: `AssignTicket` bytecode contains an enforceable nested
`REQUIRE_FIELD_IN`, and native execution enforces SupportAgent/SupportManager
assignee roles.

### Task 130: Native ELF Rejects Unlowered Observed Requirements

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing unlowered-rule native test**

Compile `CreateTicket` to `--target linux-x86_64-elf` and require failure
because its `the customer id and title` requirement still lowers to
`OBSERVE_RULE`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unlowered_observed_requirements -- --nocapture
```

Expected: failure because the native backend silently emits an executable for
an action with an unlowered observed requirement.

- [x] **Step 3: Reject observed rules in native lowering**

Treat `OBSERVE_RULE` as an unsupported native machine-code opcode with a clear
diagnostic that includes the rule text and action name. Supported native actions
must lower requirements to executable opcodes before ELF emission.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unlowered_observed_requirements -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_field_in_requirements -- --nocapture
```

Expected: native `CreateTicket` is rejected until its requirement is lowered,
while fully lowered native `AssignTicket` still compiles and runs.

### Task 131: CreateTicket Input Requirements Compile

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing input-requirement tests**

Require the Support Ticket bytecode for `CreateTicket` to lower
`the customer id and title` into executable `REQUIRE_EXISTS` checks for
`customer.id` and `ticket.title`. Add runtime and native ELF coverage so
`CreateTicket` succeeds with both argv entries and exits nonzero when either
input is missing. Retarget the observed-rule native rejection test to
`MarksOverdueTickets`, whose time comparison is still intentionally unlowered.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_create_ticket_inputs -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unlowered_observed_requirements -- --nocapture
```

Expected: bytecode still contains `OBSERVE_RULE` for `CreateTicket`, native
`CreateTicket` compilation is rejected as an unlowered observed rule, and the
retargeted overdue-time rejection remains green.

- [x] **Step 3: Preserve users in AIL-Core and lower input fields**

Represent application users as `User` nodes in checked AIL-Core, reconstruct
them when compiling from core, and resolve compound input requirements only
when every conjunct maps to an application user field or unique runtime field.
Emit one `REQUIRE_EXISTS` check per resolved input key with
`RequirementFailed` for missing input.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_create_ticket_input_requirements -- --nocapture
cargo test --test ail_toolchain ail_core_elaboration_serializes_support_ticket_graph -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_create_ticket_inputs -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unlowered_observed_requirements -- --nocapture
```

Expected: `CreateTicket` compiles to executable input checks, native ELF
execution enforces `customer.id` and `ticket.title`, and still-unlowered
observed rules are rejected before ELF emission.

### Task 132: CreateTicket Status Writes Compile

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing create-write tests**

Extend `CreateTicket` runtime, bytecode, and native ELF coverage so
`the system creates a Ticket with status New` must materialize
`ticket.status=New` as final state, `SET_FIELD` bytecode, and native stdout.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_create_ticket_input_requirements -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_create_ticket_inputs -- --nocapture
```

Expected: runtime final state has no `ticket.status`, bytecode still emits
`WRITE_FIELD` for `a Ticket with status New`, and native stdout is empty.

- [x] **Step 3: Lower creation field initializers**

Extend supported write assignment parsing from `<field> to <value>` to
`<Thing> with <field> <value>` when the thing and field resolve uniquely.
Reuse the existing `SET_FIELD` opcode so the interpreter, VM, and native ELF
backend share the same state-write path.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_create_ticket_input_requirements -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_create_ticket_inputs -- --nocapture
```

Expected: `CreateTicket` produces `ticket.status=New` in runtime state,
bytecode, and native stdout while preserving input requirement enforcement.

### Task 133: CreateTicket Customer Copy Compiles

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing customer-copy tests**

Extend `CreateTicket` runtime, VM, bytecode, and native ELF coverage so
`the system records the customer as the ticket customer` copies
`customer.id=C-1` into `ticket.customer.id=C-1`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_create_ticket_input_requirements -- --nocapture
cargo test --test ail_toolchain ail_bytecode_vm_executes_create_ticket_state_writes -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_create_ticket_inputs -- --nocapture
```

Expected: the customer assignment is still a trace-only `WRITE_FIELD`,
runtime and VM state do not contain `ticket.customer.id`, bytecode has no
`COPY_FIELD`, and native stdout only contains `ticket.status=New`.

- [x] **Step 3: Add dynamic copy lowering**

Add `COPY_FIELD` bytecode with `source`, `key`, and `text` operands. Resolve
application-user copies like `the customer as the ticket customer` from
`customer.id` to `ticket.customer.id`, execute the copy in the interpreter and
VM, and teach the native ELF backend to find the source `key=` argv value and
write it under the destination key on stdout.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_create_ticket_input_requirements -- --nocapture
cargo test --test ail_toolchain ail_bytecode_vm_executes_create_ticket_state_writes -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_create_ticket_inputs -- --nocapture
```

Expected: `CreateTicket` writes both `ticket.status=New` and
`ticket.customer.id=C-1` through runtime state, VM state, bytecode, and native
ELF stdout while missing input still fails before state output.

### Task 134: Overdue Time Requirements Compile

**Files:**
- Modify: `src/ail.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing overdue-time tests**

Extend `MarksOverdueTickets` runtime, VM, bytecode, and native ELF coverage so
`the current time to be later than due_at` lowers from deterministic runtime
input `current.time` to `ticket.due_at`, succeeds only when the current time is
later, and writes `ticket.status=Overdue` on success. Move the native
observed-rule rejection guard to a synthetic unsupported approval rule so
still-unlowered requirements remain rejected.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_overdue_time_requirement -- --nocapture
cargo test --test ail_toolchain ail_bytecode_vm_enforces_overdue_time_requirement -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_overdue_time_requirement -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unlowered_observed_requirements -- --nocapture
```

Expected: the overdue rule still lowers to `OBSERVE_RULE`, VM execution accepts
not-overdue input, native compilation rejects `MarksOverdueTickets`, and the
synthetic observed-rule guard still rejects unsupported rules after using the
correct saved-spec build path.

- [x] **Step 3: Add time-after requirement lowering**

Add `REQUIRE_FIELD_AFTER` bytecode with `source`, `key`, `rule`, and `failure`
operands. Resolve `current time` to `current.time`, resolve `due_at` to
`ticket.due_at`, execute the comparison in the interpreter and VM, and teach
the native ELF backend to find both argv values and compare their UTC timestamp
tokens before emitting state writes.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_runtime_enforces_overdue_time_requirement -- --nocapture
cargo test --test ail_toolchain ail_bytecode_vm_enforces_overdue_time_requirement -- --nocapture
cargo test --test ail_toolchain ail_compiler_lowers_checked_application_to_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_executable_enforces_overdue_time_requirement -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_native_rejects_unlowered_observed_requirements -- --nocapture
```

Expected: `MarksOverdueTickets` compiles to native ELF, succeeds only for
`current.time > ticket.due_at`, writes `ticket.status=Overdue`, and unsupported
observed rules remain rejected before target emission.

### Task 135: Direct Native Compile Accepts Saved Artifacts

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing saved-artifact compile tests**

Add native `ail-compile` coverage for saved AIL-Spec and saved checked
AIL-Core artifacts. Each test emits a `CloseTicket` ELF and executes it to
prove the direct compiler path is not only argument parsing.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_accepts_saved_spec_file_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_accepts_saved_core_file_artifact -- --nocapture
```

Expected: both commands fail at the CLI boundary because `ail-compile` rejects
`--spec-file` and `--core-file`.

- [x] **Step 3: Wire saved artifacts into `ail-compile`**

Allow `ail-compile --spec-file <path>` to reuse the package metadata with a
saved spec artifact, and allow `ail-compile --core-file <path>` to compile a
saved checked AIL-Core artifact directly to native ELF. Share the native
compile path so diagnostics, required `--action`, required `--target`, and
required `--out` handling stay consistent.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_accepts_saved_spec_file_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_accepts_saved_core_file_artifact -- --nocapture
```

Expected: both saved-artifact `ail-compile` commands emit runnable ELF
executables without requiring an `ail-build` wrapper.

### Task 17: Declared Failure Handling Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/failure-without-handling.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing rejected-fixture tests**

Add a rejected Support Ticket fixture where a declared `Failure NotFound`
records a trace event but has no handling text. Extend stable diagnostic and
conformance tests to require `AIL008`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because trace-only failure sections are treated as handled.

- [x] **Step 3: Implement declared failure handling validation**

Require every `Failure` node to have at least one `handles_failure` edge.
Trace edges remain useful for runtime explanations, but they no longer satisfy
the handling requirement. Upgrade placeholder failure nodes when their declared
failure section is later elaborated, and emit stable `AIL008` diagnostics only
for declared failures without handling.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the focused diagnostic and conformance tests pass.

### Task 136: AIL Agent Records Native Target Compilation

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native-target compile action tests**

Extend the toolchain-agent bytecode test to require a `CompileNativeTarget`
action that reads the bytecode artifact, bytecode fingerprint, target platform,
native target artifact, and native target fingerprint before recording a
`NativeTargetCompiled` trace. Extend the native `ail-build --agent --target
linux-x86_64-elf` test to require that action between
`VerifyBytecodeArtifact` and `VerifyTargetArtifact`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
```

Expected: failure because the AIL-authored toolchain agent has no
`CompileNativeTarget` action.

- [x] **Step 3: Add the AIL-authored native target compile handoff**

Add `target platform` and `target artifact compilation report` to
`BuildRequest`, define `CompileNativeTarget` in the AIL package, and run it
after the bootstrap compiler emits native executable bytes and after bytecode
verification, but before target verification.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_toolchain_agent_package_lowers_to_verified_bytecode -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: the AIL-authored agent bytecode includes and runs
`CompileNativeTarget`, and native `ail-build --agent` traces the executable-byte
handoff before verifying the Linux ELF target artifact.

### Task 137: Native Bootstrap Bundle Command

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap bundle test**

Add a Linux x86_64 CLI test for `ail-bootstrap <toolchain-agent> --pass
<compiler-pass> --target linux-x86_64-elf --agent <toolchain-agent>
--artifact-dir <dir>`. Require bytecode artifacts, native ELF artifacts for the
AIL-authored toolchain agent and AIL-Meta compiler pass, an AIL-authored
`VerifyBootstrapManifest` trace, native agent verifier executable, and a
fingerprinted `manifest.ail-bootstrap.txt` with `no-host-backend-source true`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure at the CLI boundary because `ail-bootstrap` is not recognized.

- [x] **Step 3: Implement native bootstrap bundle artifacts**

Add `VerifyBootstrapManifest` to the AIL-authored toolchain agent. Add the
`ail-bootstrap` command, require `--pass`, `--agent`, `--target`, and
`--artifact-dir`, compile the toolchain agent and compiler pass to verified
AIL-Bytecode, emit native Linux ELF artifacts for every action in both packages,
run the AIL-authored bootstrap manifest verifier, and write deterministic
bytecode, native, trace, manifest, and fingerprint artifacts.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle contains AIL-authored toolchain and compiler-pass
bytecode, native ELF executable bytes, and an AIL-verified bootstrap manifest
with no host-language backend source entries.

### Task 138: Bootstrap Bundle Includes Conformance Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap conformance assertions**

Extend the native `ail-bootstrap` bundle test to require
`toolchain-agent-conformance-report.txt`,
`compiler-pass-conformance-report.txt`, their fingerprint files, manifest
entries for both reports, and AIL agent trace reads for
`buildrequest.conformance report` and
`buildrequest.conformance report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write conformance
report artifacts.

- [x] **Step 3: Generate and verify bootstrap conformance reports**

Run `run_ail_conformance` for the AIL-authored toolchain agent package and the
AIL-Meta compiler pass package during `ail-bootstrap`. Fail the command if
either conformance report fails. Write both reports and fingerprints, include
them in `manifest.ail-bootstrap.txt`, and pass a combined conformance report
fingerprint into `VerifyBootstrapManifest`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle carries conformance evidence for both bundled
AIL packages and the AIL-authored manifest verifier reads that evidence before
acceptance.

### Task 139: Bootstrap Bundle Includes Checked IR Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap IR assertions**

Extend the native `ail-bootstrap` bundle test to require
`toolchain-agent.checked.ail-core.txt`,
`compiler-pass.checked.ail-core.txt`, their fingerprint files, manifest entries
for both checked AIL-Core artifacts, and AIL agent trace reads for
`buildrequest.core ir` and `buildrequest.core ir fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write checked
AIL-Core IR artifacts.

- [x] **Step 3: Generate and verify bootstrap checked IR artifacts**

Render checked AIL-Core for the AIL-authored toolchain agent package and the
AIL-Meta compiler pass package during `ail-bootstrap`. Write both checked-core
artifacts and fingerprints, include them in `manifest.ail-bootstrap.txt`, and
pass a combined checked-core fingerprint into `VerifyBootstrapManifest`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle carries checked AIL-Core evidence for both
bundled AIL packages and the AIL-authored manifest verifier reads that evidence
before acceptance.

### Task 140: Bootstrap Bundle Includes Source Package Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap source assertions**

Extend the native `ail-bootstrap` bundle test to require source package
snapshots for the AIL-authored toolchain agent and AIL-Meta compiler pass,
source fingerprint files, manifest entries for both source package bundles, and
AIL agent trace reads for `buildrequest.source package` and
`buildrequest.source package fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write source
package snapshot artifacts.

- [x] **Step 3: Generate and verify bootstrap source package artifacts**

Copy the source `ail-package.md` and entry `spec.ail-spec.md` for both bundled
AIL packages during `ail-bootstrap`. Write deterministic source fingerprints,
include them in `manifest.ail-bootstrap.txt`, and pass a combined source
package fingerprint into `VerifyBootstrapManifest`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle carries source package evidence for both bundled
AIL packages and the AIL-authored manifest verifier reads that evidence before
acceptance.

### Task 141: Bootstrap Bundle Runs AIL-Meta Pass Over Toolchain IR

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap compiler-pass assertions**

Extend the native `ail-bootstrap` bundle test to require
`toolchain-agent.pass-output.ail-core.txt`,
`toolchain-agent.pass-output.fingerprint.txt`,
`toolchain-agent.pass-trace.txt`, and
`toolchain-agent.pass-trace.fingerprint.txt`. Require manifest entries for the
pass output core and pass trace, require the AIL-authored bootstrap verifier to
read `buildrequest.compiler pass trace`, and prove
`toolchain-agent.ailbc.json` recompiles from the pass output core.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet run the AIL-Meta
compiler pass over the toolchain agent checked IR.

- [x] **Step 3: Run the bundled AIL-Meta pass before toolchain bytecode emission**

During `ail-bootstrap`, load the toolchain agent source into checked AIL-Core,
compile the AIL-Meta compiler pass to bytecode, run its single compiler-pass
action over the toolchain agent checked IR, check the transformed IR, write the
pass output core and pass trace artifacts, and compile the toolchain agent
bytecode from the transformed IR.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves the AIL-authored compiler pass executed
inside the bootstrap chain before bytecode and native ELF emission.

### Task 142: Bootstrap Bundle Records Compiler-Pass Fixed Point

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap fixed-point assertions**

Extend the native `ail-bootstrap` bundle test to require
`bootstrap-fixed-point-report.txt`, its fingerprint file, a manifest entry for
the fixed-point report, and AIL verifier trace reads for
`buildrequest.fixed point report` and
`buildrequest.fixed point report fingerprint`. Require the report to show that
the first and second compiler-pass output fingerprints are equal.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write a fixed-point
report.

- [x] **Step 3: Run the fixed-point check**

After the first AIL-Meta pass transforms the toolchain agent checked IR, rerun
the same compiler-pass bytecode over the transformed IR. Fail the bootstrap
command if the second output differs from the first output or if the second
output has diagnostics. Write a deterministic fixed-point report and include it
in the bootstrap manifest and AIL-authored verifier state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves the AIL-Meta compiler pass reaches a
stable output before bytecode and native ELF emission.

### Task 143: Bootstrap Bundle Records Native Machine Bytecode Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native-bytecode evidence assertions**

Extend the native `ail-bootstrap` bundle test to require
`bootstrap-native-bytecode-report.txt`, its fingerprint file, a manifest entry
for the native-bytecode report, and AIL verifier trace reads for
`buildrequest.native bytecode report` and
`buildrequest.native bytecode report fingerprint`. Require the report to show
that the toolchain agent, compiler pass, and AIL verifier target artifacts are
Linux x86_64 ELF executable bytes.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write a
native-bytecode report.

- [x] **Step 3: Generate the native-bytecode report**

After compiling the toolchain agent, compiler pass, and bootstrap verifier to
native artifacts, inspect each emitted byte buffer for ELF magic, ELFCLASS64,
little-endian encoding, executable type, and x86_64 machine identity. Write a
deterministic native-bytecode report, fingerprint it, include it in the
bootstrap manifest, and add the report plus fingerprint to the AIL-authored
bootstrap verifier state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves the emitted Linux target artifacts are
machine-level ELF executable bytes before accepting the manifest.

### Task 144: Direct Native Compile Records Machine Bytecode Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing direct compile native-bytecode assertions**

Extend the direct native `ail-compile --artifact-dir` test to require
`native-bytecode-report.txt`, `native-bytecode-report.fingerprint.txt`, and a
`native-bytecode` manifest entry. Extend the AIL-authored
`VerifyCompileManifest` trace test to require reads for
`buildrequest.native bytecode report` and
`buildrequest.native bytecode report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_saved_bytecode_native_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_manifest_artifacts -- --nocapture
```

Expected: the first test fails because the report does not exist, and the
second test fails because the AIL-authored verifier does not read the report.

- [x] **Step 3: Generate the direct compile native-bytecode report**

When direct `ail-compile` writes a single native target artifact, inspect the
emitted executable for ELF magic, ELFCLASS64, little-endian encoding,
executable type, and x86_64 machine identity. Write a deterministic
native-bytecode report, fingerprint it, include it in the compile manifest, and
pass the report plus fingerprint into the AIL-authored `VerifyCompileManifest`
state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_saved_bytecode_native_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_manifest_artifacts -- --nocapture
```

Expected: direct native compile artifacts prove the emitted target is
machine-level ELF executable bytecode before the manifest verifier accepts it.

### Task 145: All-Action Native Compile Records Machine Bytecode Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing all-action native-bytecode assertions**

Extend the direct `ail-compile --all-actions --target linux-x86_64-elf
--artifact-dir` bundle test to require `native-bytecode-report.txt`,
`native-bytecode-report.fingerprint.txt`, and a `native-bytecode` manifest
entry. Extend the AIL-authored `VerifyCompileBundleManifest` trace test to
require reads for `buildrequest.native bytecode report` and
`buildrequest.native bytecode report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_all_action_native_bundle -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_all_action_native_bundle -- --nocapture
```

Expected: the first test fails because the bundle report does not exist, and
the second test fails because the AIL-authored verifier does not read the
report.

- [x] **Step 3: Generate the all-action bundle native-bytecode report**

When `ail-compile --all-actions` writes native target artifacts, inspect every
target and agent verifier byte buffer for ELF magic, ELFCLASS64, little-endian
encoding, executable type, and x86_64 machine identity. Write a deterministic
native-bytecode report, fingerprint it, include it in the bundle manifest, and
pass the report plus fingerprint into the AIL-authored
`VerifyCompileBundleManifest` state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_all_action_native_bundle -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_all_action_native_bundle -- --nocapture
```

Expected: all-action native compile bundles prove every emitted target is
machine-level ELF executable bytecode before the manifest verifier accepts it.

### Task 146: Native `ail-build` Records Machine Bytecode Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native build bytecode assertions**

Extend the native `ail-build --target linux-x86_64-elf --artifact-dir` test to
require `native-bytecode-report.txt`,
`native-bytecode-report.fingerprint.txt`, and a `native-bytecode` manifest entry.
Extend the AIL-authored `VerifyBuildManifest` trace test to require reads for
`buildrequest.native bytecode report` and
`buildrequest.native bytecode report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_target_is_in_artifact_manifest -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: the first test fails because the build report does not exist, and the
second test fails because the AIL-authored manifest verifier does not read the
report.

- [x] **Step 3: Generate the build native-bytecode report**

When `ail-build --target linux-x86_64-elf --artifact-dir` writes native target
artifacts, inspect the target, native compiler-pass, and native agent executable
buffers for ELF magic, ELFCLASS64, little-endian encoding, executable type, and
x86_64 machine identity. Write a deterministic native-bytecode report, fingerprint
it, include it in the build manifest, and pass the report plus fingerprint into
the AIL-authored `VerifyBuildManifest` state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_target_is_in_artifact_manifest -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: native `ail-build` artifacts prove every emitted native target is
machine-level ELF executable bytecode before the manifest verifier accepts it.

### Task 147: Lower And Build Fingerprint Checked AIL-Core

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing checked-core fingerprint assertions**

Extend `ail-lower --artifact-dir` and `ail-build --agent --artifact-dir` tests
to require `checked.ail-core.fingerprint.txt`, `core checked.ail-core.txt
<fingerprint>` manifest entries, and AIL-authored lower/build manifest verifier
reads for `buildrequest.core ir fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_accepts_saved_core_file_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_lower_agent_verifies_manifest_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: lower/build tests fail because checked AIL-Core fingerprint artifacts
are missing and the AIL-authored lower/build manifest verifiers do not read the
core fingerprint.

- [x] **Step 3: Generate checked-core fingerprints**

When lower/build artifact directories persist checked AIL-Core, write
`checked.ail-core.fingerprint.txt`, include the checked-core fingerprint in the
manifest, and pass that fingerprint into the AIL-authored lower/build manifest
verifier state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_accepts_saved_core_file_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_lower_agent_verifies_manifest_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: lower/build artifact manifests now tie checked AIL-Core IR to
bytecode artifacts with deterministic fingerprints before the AIL-authored
manifest verifier accepts the bundle.

### Task 148: Build Fingerprints Requirements And Accepted Spec Artifacts

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing requirements/spec fingerprint assertions**

Extend `ail-build --artifact-dir` tests to require
`requirements.fingerprint.txt`, `accepted.ail-spec.fingerprint.txt`,
fingerprinted manifest entries for the requirements and accepted spec artifacts,
and AIL-authored build manifest verifier reads for
`buildrequest.requirements fingerprint` and `buildrequest.spec fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: the non-agent artifact test fails because requirements/spec
fingerprint artifacts are missing, while the agent verifier coverage documents
the required manifest reads.

- [x] **Step 3: Generate requirements/spec fingerprints**

When `ail-build --artifact-dir` persists captured or loaded requirements and
accepted AIL-Spec artifacts, write deterministic fingerprint sidecars, include
those fingerprints in `manifest.ail-build.txt`, and pass them into the
AIL-authored `VerifyBuildManifest` state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: build artifacts now tie requirements, accepted spec, checked AIL-Core
IR, and bytecode artifacts together with deterministic fingerprints before the
AIL-authored manifest verifier accepts the bundle.

### Task 149: Build Records Source Package Fingerprints

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing source package fingerprint assertions**

Extend `ail-build --artifact-dir` tests to require `source.ail-package.md`,
`source.ail-spec.md`, `source.fingerprint.txt`, a fingerprinted
`source-package` manifest entry, and AIL-authored build manifest verifier reads
for `buildrequest.source package` and
`buildrequest.source package fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: the non-agent artifact test fails because source package snapshot
artifacts are missing, and the agent verifier test fails because
`VerifyBuildManifest` does not read the source package fingerprint.

- [x] **Step 3: Generate source package snapshots**

When `ail-build --artifact-dir` runs from a source package, write the package
manifest and entry AIL-Spec snapshot, fingerprint the deterministic source
bundle, include that fingerprint in `manifest.ail-build.txt`, and pass the
source bundle plus fingerprint into the AIL-authored `VerifyBuildManifest`
state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_bytecode_artifact_after_compile -- --nocapture
```

Expected: package-backed build artifacts now tie source package, requirements,
accepted spec, checked AIL-Core IR, and bytecode artifacts together with
deterministic fingerprints before the AIL-authored manifest verifier accepts
the bundle.

### Task 150: Lower Records Source Package Fingerprints

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing lower source fingerprint assertions**

Extend `ail-lower --agent --artifact-dir` tests to require
`source.ail-package.md`, `source.ail-spec.md`, `source.fingerprint.txt`, a
fingerprinted `source-package` lower manifest entry, and AIL-authored
`VerifyLowerManifest` reads for `buildrequest.source package` and
`buildrequest.source package fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_agent_verifies_manifest_artifacts -- --nocapture
```

Expected: the lower-agent test fails because package-backed lower artifacts do
not write source package snapshots and the lower manifest verifier does not
read the source package fingerprint.

- [x] **Step 3: Generate lower source package snapshots**

When package-backed `ail-lower --artifact-dir` persists lowering artifacts,
write the package manifest and entry AIL-Spec snapshot, fingerprint the
deterministic source bundle, include that fingerprint in
`manifest.ail-lower.txt`, and pass the source bundle plus fingerprint into the
AIL-authored `VerifyLowerManifest` state. Saved-core lowering remains a
source-free artifact boundary.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_lower_agent_verifies_manifest_artifacts -- --nocapture
```

Expected: package-backed lower artifacts now tie source package, checked
AIL-Core IR, and AIL-Bytecode artifacts together with deterministic
fingerprints before the AIL-authored lower manifest verifier accepts the
bundle.

### Task 151: Compile Records Source Package Fingerprints

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing compile source fingerprint assertions**

Add a package-backed `ail-compile --agent --artifact-dir --target
linux-x86_64-elf` test requiring `source.ail-package.md`,
`source.ail-spec.md`, `source.fingerprint.txt`, a fingerprinted
`source-package` compile manifest entry, and AIL-authored
`VerifyCompileManifest` reads for `buildrequest.source package` and
`buildrequest.source package fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_package_agent_records_source_package_fingerprints -- --nocapture
```

Expected: the package-backed compile test fails because direct compile
artifacts do not write source package snapshots and the compile manifest
verifier does not read the source package fingerprint.

- [x] **Step 3: Generate compile source package snapshots**

When package-backed `ail-compile --artifact-dir` persists native ELF artifacts,
write the package manifest and entry AIL-Spec snapshot, fingerprint the
deterministic source bundle, include that fingerprint in
`manifest.ail-compile.txt`, and pass the source bundle plus fingerprint into
the AIL-authored `VerifyCompileManifest` and `VerifyCompileBundleManifest`
states. Saved bytecode and saved-core compile paths remain source-free artifact
boundaries.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_package_agent_records_source_package_fingerprints -- --nocapture
```

Expected: package-backed compile artifacts now tie source package, checked
AIL-Core IR, AIL-Bytecode, native-bytecode report, and Linux ELF executable
artifacts together with deterministic fingerprints before the AIL-authored
compile manifest verifier accepts the bundle.

### Task 152: Pass Records Source Package Fingerprints

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing pass source fingerprint assertions**

Extend package-backed `ail-pass --artifact-dir` and
`ail-pass --agent --artifact-dir` tests to require compiler-pass source
snapshots, target source snapshots, deterministic fingerprint sidecars,
fingerprinted `compiler-pass-source` and `target-source` manifest entries, and
AIL-authored `VerifyPassManifest` reads for compiler-pass source and target
source package fingerprints.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_pass_agent_accepts_pass_artifacts -- --nocapture
```

Expected: the non-agent pass artifact test fails because source package
snapshots are missing, and the agent pass test fails because
`VerifyPassManifest` does not read source package fingerprints.

- [x] **Step 3: Generate pass source package snapshots**

When package-backed `ail-pass --artifact-dir` persists compiler-pass artifacts,
write source snapshots and fingerprints for the compiler pass package and the
target package, include those fingerprints in `manifest.ail-pass.txt`, and pass
the source bundles plus fingerprints into the AIL-authored `VerifyPassManifest`
state. Saved compiler-pass bytecode and saved target core inputs remain
source-free artifact boundaries.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_auditable_intermediate_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_pass_agent_accepts_pass_artifacts -- --nocapture
```

Expected: package-backed pass artifacts now tie compiler-pass source, target
source, pass bytecode, input AIL-Core IR, transformed AIL-Core IR, and pass
trace artifacts together with deterministic fingerprints before the
AIL-authored pass manifest verifier accepts the bundle.

### Task 153: Prompt Portability Records Base Model

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing base-model portability assertions**

Extend the prompt-driven `ail-build --agent --target-model <name>` test to
also pass `--base-model <name>`, require the AIL-authored
`CompareAgentPromptPortability` action to read `BuildRequest base model`, and
require `prompt-portability.txt` to record both base and target model labels.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_compares_prompt_portability_before_compile -- --nocapture
```

Expected: failure before the LLM request because `--base-model` is not accepted
by `ail-build`.

- [x] **Step 3: Thread source model evidence through the agent**

Parse `--base-model` for `ail-build`, require it to be paired with
`--target-model`, insert the base model into the AIL-authored portability
comparison state, make `CompareAgentPromptPortability` read it, and include
both base and target model labels in the deterministic portability report. When
`--base-model` is omitted, use the active LLM endpoint label as the source side
of the comparison.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_agent_compares_prompt_portability_before_compile -- --nocapture
```

Expected: the agent trace records base-model and target-model reads before
compile, and `prompt-portability.txt` fingerprints the source and target model
labels alongside the portability status.

### Task 154: Bootstrap Records Host Boundary Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap host-boundary assertions**

Extend the native `ail-bootstrap` bundle test to require
`bootstrap-host-boundary-report.txt`,
`bootstrap-host-boundary-report.fingerprint.txt`, a fingerprinted
`bootstrap-host-boundary` manifest entry, and AIL-authored
`VerifyBootstrapManifest` reads for `BuildRequest host boundary report` and
`BuildRequest host boundary report fingerprint`. Require the report to state
that generated host-language backend source is absent and that the generated
artifact boundary contains AIL source, checked AIL-Core, AIL-Bytecode, reports,
and ELF machine bytecode.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write the
host-boundary report.

- [x] **Step 3: Generate and verify the host-boundary report**

Render a deterministic bootstrap host-boundary report, fingerprint it, include
it in `manifest.ail-bootstrap.txt`, write its sidecar fingerprint, and pass the
report plus fingerprint into the AIL-authored `VerifyBootstrapManifest` state.
The report records `no-host-backend-source true`, forbidden host-language source
suffixes, generated-host-language-source `none`, AIL source files, checked
AIL-Core files, AIL-Bytecode files, report files, and ELF machine-bytecode
artifacts.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves its generated boundary contains no
host-language backend source before the AIL-authored bootstrap verifier accepts
the manifest.

### Task 155: Bootstrap Records Zero Dependency Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap dependency assertions**

Extend the native `ail-bootstrap` bundle test to require
`bootstrap-dependency-report.txt`,
`bootstrap-dependency-report.fingerprint.txt`, a fingerprinted
`bootstrap-dependencies` manifest entry, and AIL-authored
`VerifyBootstrapManifest` reads for `BuildRequest dependency report` and
`BuildRequest dependency report fingerprint`. Require the report to state that
host-language runtime, dynamic linker, shared libraries, library dependencies,
and linker invocation are all absent, and that each emitted ELF is a standalone
Linux syscall executable.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write the
dependency report.

- [x] **Step 3: Inspect ELF dependency boundaries**

Render a deterministic bootstrap dependency report, fingerprint it, include it
in `manifest.ail-bootstrap.txt`, write its sidecar fingerprint, and pass the
report plus fingerprint into the AIL-authored `VerifyBootstrapManifest` state.
The report inspects each emitted ELF program header table and rejects dynamic
interpreter or dynamic-section entries before recording
`standalone-linux-syscall-elf`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves its emitted ELF artifacts have no
host-language runtime, dynamic linker, shared-library, library, or linker
dependency before the AIL-authored bootstrap verifier accepts the manifest.

### Task 156: Bootstrap Runs Native Handoff Smoke Tests

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing bootstrap handoff assertions**

Extend the native `ail-bootstrap` bundle test to require
`bootstrap-handoff-report.txt`,
`bootstrap-handoff-report.fingerprint.txt`, a fingerprinted
`bootstrap-handoff` manifest entry, and AIL-authored
`VerifyBootstrapManifest` reads for `BuildRequest handoff report` and
`BuildRequest handoff report fingerprint`. Require the report to show that
generated native AIL toolchain actions and the AIL-Meta compiler pass ran
through the Linux syscall argv ABI.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the bootstrap bundle does not yet write the handoff
report.

- [x] **Step 3: Execute representative generated native tools**

Render a deterministic bootstrap handoff report by writing selected generated
ELF bytes to a temporary executable, running
`toolchain-agent-CompileApplication.elf`,
`toolchain-agent-CompileNativeTarget.elf`, and
`compiler-pass-InferReadPermissions.elf` with deterministic argv state, and
requiring their expected AIL trace markers before recording
`handoff-native-action ... ok trace ...` entries. Fingerprint the report,
include it in `manifest.ail-bootstrap.txt`, write its sidecar fingerprint, and
pass the report plus fingerprint into the AIL-authored
`VerifyBootstrapManifest` state.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves representative generated native AIL
toolchain actions and the native AIL-Meta compiler pass execute successfully
before the AIL-authored bootstrap verifier accepts the manifest.

### Task 157: Bootstrap Handoff Covers Every Generated Native Tool

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/13-bootstrap-self-hosting.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing all-action handoff assertions**

Extend the native `ail-bootstrap` bundle test so
`bootstrap-handoff-report.txt` must contain role summary entries for every
generated `toolchain-agent-*`, `compiler-pass-*`, and `agent-*` native ELF
artifact. Require the report to include handoff traces for previously unproven
actions such as `VerifyConformanceManifest`, `CompareAgentPromptPortability`,
and the native verifier-agent `VerifyBootstrapManifest`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: failure because the handoff report only runs representative native
actions.

- [x] **Step 3: Run every generated native action**

Replace the representative handoff runner with a role-based runner. Derive each
action name from the emitted ELF filename, select deterministic argv state for
that action's AIL requirements and trace, execute the temporary ELF, require
the expected trace marker, record stdout/stderr fingerprints, and finish each
role with `handoff-native-role ... all-actions ok count ...`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_bootstrap_writes_native_toolchain_bundle -- --nocapture
```

Expected: the bootstrap bundle proves every generated native AIL toolchain
action, verifier-agent action, and AIL-Meta compiler pass action executes
successfully before the AIL-authored bootstrap verifier accepts the manifest.

### Task 158: Direct Native Compile Records Dependency Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing direct compile dependency assertions**

Extend direct `ail-compile --target linux-x86_64-elf --artifact-dir` coverage
to require `dependency-report.txt`, `dependency-report.fingerprint.txt`, and a
fingerprinted `dependencies` entry in `manifest.ail-compile.txt`. Extend the
AIL-authored `VerifyCompileManifest` flow to read `BuildRequest dependency
report` and `BuildRequest dependency report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_saved_bytecode_native_artifacts -- --nocapture
```

Expected: failure because direct native compile artifacts do not yet write the
dependency report.

- [x] **Step 3: Generate the direct compile dependency report**

Render a deterministic `AIL-Compile-Dependency-Report`, fingerprint it, include
it in `manifest.ail-compile.txt`, write its sidecar fingerprint, and pass the
report plus fingerprint into the AIL-authored `VerifyCompileManifest` state.
The report inspects the selected `target.elf` and any native verifier-agent ELF
artifacts for standalone Linux syscall ELF identity with no dynamic linker,
shared libraries, host-language runtime, or linker invocation.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_saved_bytecode_native_artifacts -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_manifest_artifacts -- --nocapture
```

Expected: direct native compile artifacts prove their emitted target and
verifier-agent ELFs have no host-language runtime, dynamic linker,
shared-library, library, or linker dependency before the AIL-authored compile
manifest verifier accepts the manifest.

### Task 159: All-Action Native Compile Records Dependency Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing all-action dependency assertions**

Extend direct `ail-compile --all-actions --target linux-x86_64-elf
--artifact-dir` bundle coverage to require `dependency-report.txt`,
`dependency-report.fingerprint.txt`, and a fingerprinted `dependencies` entry
in `manifest.ail-compile.txt`. Extend the AIL-authored
`VerifyCompileBundleManifest` flow to read `BuildRequest dependency report`
and `BuildRequest dependency report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_all_action_native_bundle -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_all_action_native_bundle -- --nocapture
```

Expected: the first test fails because the all-action bundle does not yet write
the dependency report, and the second test fails because the AIL-authored
bundle verifier does not receive dependency report state.

- [x] **Step 3: Generate the all-action dependency report**

Render a deterministic `AIL-Compile-Bundle-Dependency-Report`, fingerprint it,
include it in `manifest.ail-compile.txt`, write its sidecar fingerprint, and
pass the report plus fingerprint into the AIL-authored
`VerifyCompileBundleManifest` state. The report inspects every generated
`target-<Action>.elf` and native verifier-agent ELF artifact for standalone
Linux syscall ELF identity with no dynamic linker, shared libraries,
host-language runtime, or linker invocation.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_compile_writes_all_action_native_bundle -- --nocapture
cargo test --test ail_toolchain cli_ail_compile_agent_verifies_all_action_native_bundle -- --nocapture
```

Expected: all-action native compile bundles prove every emitted target and
verifier-agent ELF has no host-language runtime, dynamic linker,
shared-library, library, or linker dependency before the AIL-authored compile
bundle manifest verifier accepts the bundle.

### Task 160: Native Build Records Dependency Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing native build dependency assertions**

Extend `ail-build --target linux-x86_64-elf --artifact-dir` coverage to
require `dependency-report.txt`, `dependency-report.fingerprint.txt`, and a
fingerprinted `dependencies` entry in `manifest.ail-build.txt`. Require the
report to cover the emitted `target.elf`, native compiler-pass ELFs, and native
AIL verifier-agent ELFs when present. Extend the AIL-authored
`VerifyBuildManifest` flow to read `BuildRequest dependency report` and
`BuildRequest dependency report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_target_is_in_artifact_manifest -- --nocapture
cargo test --test ail_toolchain cli_ail_build_with_pass_writes_native_pass_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: native build artifact tests fail because the dependency report is not
written yet, and the AIL-authored build manifest verifier lacks dependency
report state.

- [x] **Step 3: Generate the native build dependency report**

Render a deterministic `AIL-Build-Dependency-Report`, fingerprint it, include
it in `manifest.ail-build.txt`, write its sidecar fingerprint, and pass the
report plus fingerprint into the AIL-authored `VerifyBuildManifest` state. The
report inspects the emitted `target.elf`, native compiler-pass ELFs, and native
verifier-agent ELF artifacts for standalone Linux syscall ELF identity with no
dynamic linker, shared libraries, host-language runtime, or linker invocation.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_build_native_target_is_in_artifact_manifest -- --nocapture
cargo test --test ail_toolchain cli_ail_build_with_pass_writes_native_pass_artifact -- --nocapture
cargo test --test ail_toolchain cli_ail_build_agent_verifies_native_target_artifact -- --nocapture
```

Expected: native `ail-build` artifacts prove the emitted target,
compiler-pass, and verifier-agent ELFs have no host-language runtime, dynamic
linker, shared-library, library, or linker dependency before the AIL-authored
build manifest verifier accepts the manifest.

### Task 161: Standalone Pass Records Native Bytecode Evidence

**Files:**
- Modify: `examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- Modify: `src/main.rs`
- Modify: `tests/ail_toolchain.rs`
- Modify: `README.md`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`

- [x] **Step 1: Write failing standalone pass native-bytecode assertions**

Extend `ail-pass --target linux-x86_64-elf --agent --artifact-dir` coverage to
require `native-bytecode-report.txt`, `native-bytecode-report.fingerprint.txt`,
and a fingerprinted `native-bytecode` entry in `manifest.ail-pass.txt`.
Require the report to cover native AIL-Meta compiler-pass ELFs and native
AIL-authored pass-agent ELFs. Extend the AIL-authored `VerifyPassManifest`
flow to read `BuildRequest native bytecode report` and
`BuildRequest native bytecode report fingerprint`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_native_tool_artifacts -- --nocapture
```

Expected: the standalone native pass test fails because the native-bytecode
report is not written yet and the AIL-authored pass manifest verifier lacks
native-bytecode report state.

- [x] **Step 3: Generate the pass native-bytecode report**

Render a deterministic `AIL-Pass-Native-Bytecode` report, fingerprint it,
include it in `manifest.ail-pass.txt`, write its sidecar fingerprint, and pass
the report plus fingerprint into the AIL-authored `VerifyPassManifest` state.
The report inspects native compiler-pass ELFs and native pass-agent ELF
artifacts for ELF64 little-endian x86_64 executable identity.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_pass_writes_native_tool_artifacts -- --nocapture
```

Expected: standalone native `ail-pass` artifacts prove the emitted
compiler-pass and verifier-agent tools are machine-level ELF executable bytes
before the AIL-authored pass manifest verifier accepts the manifest.

### Task 18: Declared Failure Trace Coverage Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/failure-without-trace.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing rejected-fixture tests**

Add a rejected Support Ticket fixture where a declared `Failure NotFound` has
handling text but does not record a trace event. Extend stable diagnostic and
conformance tests to require `AIL009`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because declared failures with handling but no trace are not
diagnosed.

- [x] **Step 3: Implement declared failure trace validation**

Require every declared `Failure` node to have at least one `records_trace`
edge. Runtime-generated or placeholder failures remain outside this diagnostic;
emit stable `AIL009` diagnostics only for failures marked `declared=true`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the focused diagnostic and conformance tests pass.

### Task 19: Action Trace Coverage Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing rejected-fixture tests**

Add a rejected Support Ticket fixture where an action has checked behavior but
does not record any trace event. Extend stable diagnostic and conformance tests
to require `AIL010`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because action trace coverage is diagnosed without the stable
`AIL010` code.

- [x] **Step 3: Implement stable action trace diagnostics**

Keep the existing action trace coverage check, but emit stable
`AIL010 action <ActionName> is missing trace coverage` diagnostics so
conformance can identify this failure class.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the focused diagnostic and conformance tests pass.

### Task 20: Behavior Bullet Provenance In AIL-Core

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing provenance assertions**

Add an AIL-Core elaboration test requiring behavior bullets to attach
`has_provenance` edges for representative action requirements, writes,
guarantees, traces, failure handling, and failure trace bullets.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_elaboration_preserves_provenance_for_behavior_bullets
```

Expected: failure because behavior-derived nodes do not yet have source-bullet
provenance.

- [x] **Step 3: Attach behavior-bullet provenance**

Add a shared AIL-Core provenance helper and attach provenance to semantic nodes
created from application views, action requirements, effect writes/reads,
secret-protection effects, action failure mentions, guarantees, traces, failure
handling, and failure traces.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_elaboration_preserves_provenance_for_behavior_bullets
cargo test --test ail_toolchain
```

Expected: the focused provenance test and full AIL toolchain integration target
pass.

### Task 21: Accepted Fixture Conformance

**Files:**
- Modify: `src/ail.rs`
- Modify: `src/main.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/accepted/close-ticket-minimal.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing accepted-fixture conformance test**

Add a valid accepted Support Ticket fixture and require `ail-conformance` to
report `accepted: close-ticket-minimal.ail-spec.md`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: failure because conformance only checks the entry spec and rejected
fixtures.

- [x] **Step 3: Validate accepted fixture directory**

Extend the conformance result model and CLI output with accepted fixtures from
`examples/accepted`. Accepted fixtures succeed only when their diagnostics are
empty; any accepted-fixture diagnostic fails conformance.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
cargo test --test ail_toolchain
```

Expected: conformance reports the accepted fixture and the full AIL toolchain
integration target passes.

### Task 22: Enforce Semantic Node Provenance

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing missing-provenance test**

Start from a valid Support Ticket AIL-Core graph, remove the `has_provenance`
edge from a rule node, and require stable `AIL011` diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_missing_provenance_for_semantic_nodes
```

Expected: failure because the checker does not enforce provenance on semantic
nodes.

- [x] **Step 3: Implement semantic provenance checking**

Emit `AIL011 <kind> '<name>' is missing provenance` for semantic nodes without
`has_provenance`, excluding `Provenance` nodes themselves. Attach field
provenance to generated `Secret` helper nodes so valid cores remain clean.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_missing_provenance_for_semantic_nodes
cargo test --test ail_toolchain
```

Expected: the focused missing-provenance test and full AIL toolchain
integration target pass.

### Task 23: Guarantee Attachment Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing unattached-guarantee test**

Start from a valid Support Ticket AIL-Core graph, remove the `guarantees` edge
from a guarantee node, and require stable `AIL012` diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_guarantees
```

Expected: failure because the checker does not enforce guarantee attachment.

- [x] **Step 3: Implement guarantee attachment checking**

Emit `AIL012 guarantee '<name>' is not attached to an action or tool` for every
`Guarantee` node without an incoming `guarantees` edge from an `Action` or
`Tool`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_guarantees
cargo test --test ail_toolchain
```

Expected: the focused unattached-guarantee test and full AIL toolchain
integration target pass.

### Task 24: Trace Attachment Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing orphan-trace test**

Start from a valid Support Ticket AIL-Core graph, add a `Trace` node with
provenance but no incoming `records_trace` edge, and require stable `AIL013`
diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_traces
```

Expected: failure because the checker does not enforce trace attachment.

- [x] **Step 3: Implement trace attachment checking**

Emit `AIL013 trace '<name>' is not recorded by an action or failure` for every
`Trace` node without an incoming `records_trace` edge from an `Action` or
`Failure`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_traces
cargo test --test ail_toolchain
```

Expected: the focused orphan-trace test and full AIL toolchain integration
target pass.

### Task 25: Rule Attachment Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing orphan-rule test**

Start from a valid Support Ticket AIL-Core graph, add a `Rule` node with
provenance but no incoming `requires` edge, and require stable `AIL014`
diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_rules
```

Expected: failure because the checker does not enforce rule attachment.

- [x] **Step 3: Implement rule attachment checking**

Emit `AIL014 rule '<name>' is not required by an action` for every `Rule` node
without an incoming `requires` edge from an `Action`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_rules
cargo test --test ail_toolchain
```

Expected: the focused orphan-rule test and full AIL toolchain integration
target pass.

### Task 26: Effect Attachment Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing orphan-effect test**

Start from a valid Support Ticket AIL-Core graph, add an `Effect` node with
provenance but no incoming semantic edge, and require stable `AIL015`
diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_effects
```

Expected: failure because the checker does not enforce effect attachment.

- [x] **Step 3: Implement effect attachment checking**

Emit `AIL015 effect '<name>' is not attached to an action or failure` for every
`Effect` node without an incoming action `reads`, action `writes`, action
`protects_secret`, or failure `handles_failure` edge.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_effects
cargo test --test ail_toolchain
```

Expected: the focused orphan-effect test and full AIL toolchain integration
target pass.

### Task 27: Secret Attachment Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing orphan-secret test**

Start from a valid Support Ticket AIL-Core graph, add a `Secret` node with
provenance but no field or action attachment, and require stable `AIL016`
diagnostics.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_secrets
```

Expected: failure because the checker does not enforce secret attachment.

- [x] **Step 3: Implement secret attachment checking**

Emit `AIL016 secret '<name>' is not attached to a field or action` for every
`Secret` node that neither protects a declared `Field` nor is the target of an
action `protects_secret` edge.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_unattached_secrets
cargo test --test ail_toolchain
```

Expected: the focused orphan-secret test and full AIL toolchain integration
target pass.

### Task 16: Requirement Field Reference Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/unknown-requirement-field.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing rejected-fixture tests**

Add a rejected Support Ticket fixture where a requirement references
`ticket priority` even though `Ticket.priority` is undeclared. Extend stable
diagnostic and conformance tests to require `AIL007`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because requirement field references are not checked.

- [x] **Step 3: Implement requirement field validation**

Inspect requirement rules shaped like field comparisons (`to be` and
`not to be`). When the left side looks like a declared thing field reference
but no declared field resolves, emit stable `AIL007` diagnostics.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the focused diagnostic and conformance tests pass.

### Task 15: Field Type Validation Diagnostics

**Files:**
- Modify: `src/ail.rs`
- Modify: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `examples/support_ticket.ail/examples/rejected/unknown-field-type.ail-spec.md`
- Test: `tests/ail_toolchain.rs`

- [x] **Step 1: Write failing rejected-fixture tests**

Add a rejected Support Ticket fixture with `Ticket.metadata: MysteryBox`.
Extend stable diagnostic and conformance tests to require `AIL006` for unknown
field types.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
```

Expected: failure because the checker accepts arbitrary type names.

- [x] **Step 3: Implement field type validation**

Validate `Field` node types against supported scalar types, `State<...>`,
generic wrappers (`Option<T>`, `List<T>`, `Secret<T>`), and declared AIL thing
types, including imported namespaced things. Emit stable `AIL006` diagnostics
for unknown types.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test ail_toolchain ail_core_reports_stable_invalid_fixture_diagnostics
cargo test --test ail_toolchain cli_ail_conformance_checks_valid_and_rejected_fixtures
```

Expected: the focused diagnostic and conformance tests pass.
