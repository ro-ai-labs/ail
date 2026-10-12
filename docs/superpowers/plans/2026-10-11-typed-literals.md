# Typed Literals Implementation Plan

**Goal:** Let source programs create typed constants that execute through checked
Core, STE, VM and native ELF, furthering the active Python-replacement objective.

**Architecture:** A focused literal module validates JSON against declared field
types and owns Core semantics. Root integrates ordered actions, STE, VM and native
dispatch; the native emitter embeds decoded immutable values.

**Tech Stack:** Dependency-free Rust; direct Linux x86-64 ELF; Python oracles.
**Spec:** `docs/superpowers/specs/2026-10-11-typed-literals-design.md`.

The user has authorized autonomous continuation. Preserve the current shared
worktree and all earlier uncommitted milestones. Use the parallel-agent skill
for independent module ownership and review; root performs integration.

## Global constraints and review focus

- Supported types: Int, Bool, Text, List<Int>, List<Text>; exact JSON types.
- Text maximum 1,048,576 decoded bytes; lists maximum 65,536 items and 1,048,576
  input/canonical bytes; raw literal maximum 6,291,458 bytes.
- Static assignments do not charge text/list result budgets.
- Source keywords/field-like strings inside literals must remain opaque data.
- Secret writes must derive from the executable destination even without writes.
- Saved duplicate edges, wrong types, unsupported profiles and function locals
  must reject before execution.
- Branch/call order, immutable value copies and imports must retain semantics.

## Tasks

- [x] Root: add `tests/literal_assignments.rs` with a real source-to-native
  constant program; run it and confirm failure at the missing source grammar.
- [x] Module: create `src/literal_assignment.rs` and the list canonicalization
  wrapper. Implement the spec's exact AST/API, strict type/limit validation,
  Core decoding/checking, imports and derived writes. Add focused boundary and
  malformed-input tests; confirm they pass after integration.
- [x] Root: integrate AilAction/AilActionStep, elaboration/reconstruction,
  source parse/render/import, saved edge multiplicity, schema/checker/accesses,
  action order, bytecode verifier/runtime, STE template and native emitter.
  Run the failing integration test to green.
- [x] Root: extend integration coverage for round trips, immutable values,
  nested action/loop ordering, imports, exact protocols and malformed Core,
  secret destination protection, size boundaries and repeated static constants.
  Run focused tests and independent review/probes; fix reproduced findings.
- [x] Root: run formatting, Clippy, full Rust suite, frozen capability ladder,
  corpus differential and text/list oracles. Record exact results.
- [x] Root: add runnable source/STE example and agent-facing literal guide;
  verify documented commands. Keep the broader active goal open.

Verification commands: `cargo test --test literal_assignments`, focused module
tests, `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --all-targets` with localhost escalation for existing HTTP mocks,
`python3 scripts/ail_bench.py`, `python3 scripts/ail_differential.py`,
`python3 scripts/ail_text_differential.py`, `python3 scripts/ail_list_differential.py`.

## Review fixes

- Compiler-pass source bodies previously converted literal statements to
  descriptive `PASS_STEP` records. A failing regression now passes after the
  parser rejects literal syntax in that context. The same omission affected
  text/list statements; those now reject too. The guard recognizes executable
  text phrases precisely, preserving legacy prose such as `the system finds
  the actor...`. The first broad guard broke 33 existing compiler/corpus tests;
  the narrowed guard restores that parser path.
- The saved-Core parser treated commas/key assignments and brackets inside
  literals as Core syntax. Two red-to-green regressions cover delimiter-like
  string data and duplicate attribute rejection. JSON-aware suffix/comma scanning
  now applies to `LiteralAssignment` nodes. A third regression preserves the
  existing unquoted action-label and Effect-name grammar; arbitrary legacy prose
  quotes must not acquire JSON delimiter meaning.
- Core literal nodes reject a second node type, unknown attributes, duplicate
  destinations and unknown edge attributes. The established optional numeric
  `operation_index` metadata is preserved alongside required `step_index`.
- Independent review found no remaining reproduced defects after these fixes.

## Verified outcome

- **684 tests passed** in `cargo test --all-targets -- --test-threads=1` with
  authorized localhost access for existing HTTP mocks. Includes 10 new module
  tests and 13 literal integration tests. Log:
  `/tmp/ail-literal-verified-tests.log`. A preceding parallel run reached the STE
  suite with all preceding tests green but hit transient `ExecutableFileBusy`
  in `ste_comparison_order_matches_the_sentence_in_vm_and_native`; that test
  passed immediately in isolation and the complete sequential run passed.
- `cargo clippy --all-targets -- -D warnings`, formatting and diff checks pass.
- Frozen numeric ladder remains **16/16**, without case or oracle changes.
- Corpus differential: **33 agree, 30 not natively compilable, 0 diverged**.
- Existing text oracle: **605 VM/native/Python cases**, five saved-Core secret
  probes and nine invalid UTF-8 probes pass. Existing list oracle: **600 list
  pipelines**, **1,600 grammar mutations**, 37 malformed inputs, six index
  errors and nine invalid UTF-8 inputs pass.
- New durable `scripts/ail_literal_differential.py`: **500 typed values** with
  immutable copies and composition; **50 source values** through source, saved
  Core, rendered source, STE, VM and native; **85 malformed/type cases** and
  **13 exact/overflow limits** pass. Includes 1 MiB decoded text, a 6,291,458-byte
  escaped-NUL token, canonical list expansion and 65,536-item boundaries.
- `examples/typed-constants` constructs its own values, joins Unicode text and
  squares/sums a list through a function and loop without runtime arguments.
  Source and direct STE execution/compilation produce message `Hello, world! 🙂`,
  ready `true`, squares `[1,4,9]` and total `14`. Exported STE matches
  `examples/ste/typed-constants.ail-ste`. Guide: `docs/ail/39-typed-literals.md`.

The active objective remains open. This increment supplies source constants and
repairs their serialization/unsupported-profile boundaries. General I/O, nested
collections/maps, richer function/value types, arbitrary precision, the wider
Python ecosystem and full ASD-STE100 conformity remain unproven.
