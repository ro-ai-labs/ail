# JSON inspection implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development for the
> independent frontend/runtime, native and oracle tasks; root integrates shared
> protocol surfaces and verifies the complete result.

**Goal:** Enable generic JSON traversal through kind inspection and ordered keys.
**Architecture:** Extend JsonAccess with two operations and preserve the existing
lossless representation, ownership, bounded allocation and function contracts.
**Tech stack:** Dependency-free Rust 2024 VM/compiler and direct x86-64 ELF emitter.
**Spec:** ../specs/2026-10-11-json-inspection-design.md

## Global constraints and rulings

Preserve 1,048,576 raw/canonical/result bytes, 65,536 Json values, depth 64,
67,108,864-byte separate text and shared list/Json arenas, exact field layout,
immutable aliases and whole-source validation before type/allocation checks.
No external interpreter, dependencies, runtime files or compile subprocesses.

Ruling: continue in this authorized shared worktree, whose .git is read-only;
preserve all existing work and do not commit/reset/clean. The user's continuing
goal and developer autonomy authorize concrete implementation without repeating
skill approval pauses. Written spec/plan and review evidence remain reviewable.
Use existing agents for disjoint work because session spawning is exhausted;
reviewers must be different from the corresponding implementers.

## Ownership and interface checks

| Producer / consumer | Shared decision | Check |
| --- | --- | --- |
| Frontend / root | AilJsonAccess::Kind/Keys {source,key}; JSON_KIND/JSON_KEYS exact key,source | Root owns opcode allowlists/STE/local schema; worker owns structured_json.rs |
| VM / native | six lowercase kind words, ordered duplicate-preserving keys, owned result charge | Both validate complete source first; no per-key parse |
| Root / native | instruction registration and function dependencies | Native worker owns native_expr.rs/native_value_function.rs routing |
| Oracle / all | strict source/Core/STE/VM/native contract from spec | Oracle edits tests/script only; no production ownership overlap |

All task requirements agree with the spec. Type-kind and keys operations share
one source/destination schema but differ in fixed result types and arena.

## Tasks

- [x] 1. Frontend/runtime worker: src/structured_json.rs. Write/run failing tests,
  then extend source enum/parser/render/Core/compile/verify/runtime for both
  operations. Reuse unique global storage validation where relevant. Tests cover
  all kinds, ordered duplicates/Unicode/NUL keys, wrong root, exact arena
  accounting and atomic failure, whole-source limits and malformed Core. Report
  red/green commands and release owned file.
- [x] 2. Native worker: src/native_structured_json.rs, src/native_expr.rs,
  src/native_value_function.rs and src/native_json.rs routing. Write/run native red tests then implement root
  kind selection and one-pass key output, route both opcodes in actions/functions
  and helper dependency discovery. Test kinds, keys/aliases, source limits,
  one-copy budgets, functions, nonobject errors and no numeric dependencies.
- [x] 3. Root: src/ail.rs, src/ste.rs, src/value_protocol.rs,
  src/value_function.rs, src/value_vm.rs plus tests/json_inspection_pipeline.rs
  only if needed. Observe red through independent integration tests; wire all
  supported opcode lists, exact operands, profile gates, local typed operands,
  function reconstruction, trace strings and normalized descriptive-body guards.
- [x] 4. Independent oracle worker: tests/json_inspection.rs and
  scripts/ail_json_inspection_differential.py. No production edits. Test public
  pipelines, graph/protocol errors, secrets/imports, recursive functions, real
  files, source/result/budget boundaries and independent Python comparison.
- [x] 5. Root: examples/json-inspection, examples/ste/json-inspection.ail-ste,
  docs/ail/47-json-inspection.md and guide/index updates. Run recursive unknown
  key/heterogeneous-array file example through all six source/Core/STE routes.
- [x] 6. Independent reviews with precise scopes; fix findings; run full serial
  suite, strict Clippy/fmt, relevant prior oracles and unchanged16/16 ladder.
  Record final evidence and leave overall goal active.

## Review focus

1. Root type inspection must reject malformed or oversized unselected children;
   test frontend/runtime and native complete-source validation.
2. Keys differing only by encoded escapes/Unicode normalization and duplicate
   members must preserve exact decoded names/order; oracle uses object_pairs_hook.
3. New source forms must not disappear into descriptive function/pass text;
   integration tests cover both subjects, malformed prefixes and profiles.
4. Repeated kind/keys results share the correct caller/function arena; unit and
   native tests seed counters or consume large prior results to reach exact limits.
5. Native key encoding must not overwrite unread decoded keys or borrow freed
   scratch; alias/function tests read outputs after subsequent parses/mutations.

## Progress and evidence

Previous turn classified as verified progress after inspecting its current
plan and complete 957-test log. No implementation tasks dispatched before this
spec/plan and interface scan. All goal completion claims remain unproven.


Tasks 1–3 and 5 implemented. The frontend worker passes 25 module tests and
27 prior structured-JSON/mutation integration tests, plus strict library/tests
Clippy. A new forged-global-operand test observed RED before switching access
operands to the shared unique-storage checker. Native observed RED for both
opcodes and now passes all 136 emitter tests, including six inspection tests.
Initial independent public integration has 10 passing tests. Root verified the
recursive example through all six source/Core/STE × VM/native routes with exact
report bytes, kind object and 8 leaves. Both production scopes are released for
independent reviews; full final checks and oracle are pending.


Independent oracle: `/tmp/ail-json-inspection-oracle.log` passes 607 kind/key
cases, 60 alias cases, 18 malformed documents, nine raw/canonical/depth/value
and maximum-result boundaries, exact 64 MiB arena/function-return accounting,
and 54 recursive unknown-key file datasets. The oracle computes expectations
with Python while retaining raw number tokens and object pairs. It covers
source, saved Core, rendered source and STE on both backends. Ten integration
tests pass in `/tmp/ail-json-inspection-integration-final.log`.

Frontend/root review approved without findings after focused checks for exact
protocol/Core forgery, secrets/descriptive guards and atomic arena accounting.
Native review approved without findings after 22 independent VM/native
executions and checking inspection-only ELF has no Float runtime segments.
The review packages are `/tmp/ail-json-inspection-{frontend,native}-review.diff`.

The example's six-route exact result is recorded in
`/tmp/ail-json-inspection-example-six-routes.log`. Additional root checks cover
null, empty containers, nested heterogeneous arrays and huge lossless numbers;
duplicate-name and malformed-source failures preserve an existing report on
both backends (`/tmp/ail-json-inspection-example-edge-cases.log`).

All prior structured JSON, mutation, JSON conversion, value-function, list,
floating-list, Float and literal oracles exit 0. Logs use
`/tmp/ail-json-inspection-regression-*.log`. The frozen ladder remains 16/16;
broader VM/native differential remains 33 agree, 30 unsupported, 0 diverged.
Strict all-target Clippy, formatting, diff whitespace, build and Python syntax
checks pass. The main documentation and example pages have 63 resolving local
links.

## Final verification and goal status

`cargo test --all-targets -- --test-threads=1` exits 0: **981 tests across 19
targets**, including 345 library tests, 434 toolchain tests and 10 JSON-inspection
integration tests (`/tmp/ail-json-inspection-final-tests.log`). Local-socket
fixtures use the previously authorized sandbox escalation. Strict all-target
Clippy, formatting and whitespace checks also exit 0; logs are
`/tmp/ail-json-inspection-final-{clippy,fmt,diff-check}.log`.

This milestone is verified progress toward the original goal. AIL now discovers
JSON kinds and unknown object names and can express a reusable recursive walker
through the checked Specs→Core→STE→VM/native pipeline. Full Python replacement
is not proved: broader collections, predicates, libraries and runtime facilities
remain unfinished. The overall goal remains active.
