# List transformations implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development for disjoint
> frontend/runtime, native and independent oracle work; root integrates. Reuse
> available workers, and review each scope with a different implementer.

**Goal:** Efficient typed-list concat/slice/reverse/stable sort through Core/STE and real reports.
**Architecture:** Extend existing ListOperation and immutable list codec; native
sort rearranges bounded descriptors then produces one canonical owned result.
**Tech Stack:** Dependency-free Rust2024, direct x86-64 ELF, independent Python oracle.
**Spec:** ../specs/2026-10-11-list-transformations-design.md

## Global constraints

Input/result bytes1,048,576, items65,536, shared List/Json arena67,108,864.
No Text charge; empty result costs2; returned list adds one owned copy. Stable
numeric/text sorting, NaNs last in both directions, signed zeros equal/stable.
Complete validation before bounds/secondary inputs as specified; atomic failure,
immutable aliases, no operand-content diagnostics, no dependencies/runtime files.

Ruling: the continuing goal and developer autonomy authorize this architectural
increment without repeated approval gates. Preserve the existing worktree and
read-only .git; no commits, resets, cleanup or integration menu. This plan is the
persistent ledger; baseline snapshots replace commit ranges for review. Cost if
wrong is a reversible language/API design change, not an external side effect.
Previous turn is verified progress from current text-transformation files and
the1,064-test final log, not a completion claim about the full goal.

## Interfaces and preflight ownership

| Tasks | Shared interface | Ruling |
| --- | --- | --- |
| Frontend/root | four AilListOperation variants and exact opcodes from spec | frontend owns enum/list_operations; root owns call-site lists/schemas |
| Frontend/native | runtime limits/order/codec, no new public runtime signature | native reuses existing descriptor parse/encode/allocate |
| Root/native | opcode routing, local-field discovery, Float helper discovery | native owns native_*; root owns ail/value_* |
| Oracle/all | public source/Core/STE/CLI behavior | separate tests/script; no production edits |
| Root/example | functions and JSON report with all four operations | root owns docs/examples, oracle computes expected reports |

All tasks agree with the spec's immutable outputs and no intermediate charges.
Only native worker may create native_list_transform.rs. Frontend may create a
list_transformations.rs submodule if it keeps runtime algorithms focused, while
retaining existing public enum/functions. Root will not edit worker files.

## Tasks

- [x] 1. Frontend/runtime: own src/list_operations.rs and optional focused
  submodule. Add tests that fail on missing source/opcodes and missing strict Core
  roles, then implement enum/source/Core/check/namespace/access/protocol/runtime.
  Use stable numeric/text comparator with NaNs-last; parse once, one result charge.
  Cover all types, i64-extreme slicing/bound ordering, concat second input errors,
  canonical/item/arena boundaries, signed-zero stability, aliases, metadata/secret
  and no input leakage. Run focused lib list tests and strict library Clippy.
- [x] 2. Native: own src/native_expr.rs/native_list.rs/native_value_function.rs
  and optional new native_list_transform.rs. Raw-opcode RED first, then fields,
  dispatch, bounded descriptor algorithms and helper emission. Stable bottom-up
  mergesort is the chosen native algorithm. Preserve RBX/RBP/R12–R15, release
  scratch after owned copy, respect parsed input validation and separate arenas.
  Cover all types/order/NaNs/zeros, UTF-8/priority/limits/aliases, 65,536-item stress,
  body-only helper discovery and recursive frames. Focused tests only.
- [x] 3. Independent oracle: own tests/list_transformations.rs and
  scripts/ail_list_transform_differential.py. Observe RED, then independent Python
  expectations across all types/operations/sort directions/aliases. Test Core
  metadata/types/secret/imports/function locals, malformed descriptive bodies,
  profiles/exact protocol, native UTF-8, limits/both arenas/return costs, canonical
  expansion and worst-case sort datasets. Independently calculate actual example
  reports through all six routes×VM/native. No production edits/full suite.
- [x] 4. Root: ail.rs edge catalog/saved-Core preservation/native-entry/opcode/
  profile/trace/VM routes and malformed-body guards; value_protocol.rs typed
  fields/Int bounds, value_function.rs reconstruction, value_vm.rs routing;
  ste.rs exact four templates/fixture support. Compile after interfaces arrive.
- [x] 5. Root: docs/ail/50-list-transformations.md and indices/schema/source/STE/
  lists/Float/functions references; examples/list-statistics and exported STE.
  Exercise sorted median/top-window report through reusable local functions and
  actual bound files; oracle owns independent expectations.
- [x] 6. Independent frontend/root/native reviews, focused fixes and re-review;
  final serial all-target Rust suite, strictClippy/fmt/diff, new/prior oracles and
  frozen benchmark. Record counts/logs and preserve the full active objective.

## Review focus

- Descending Float sorts must preserve both zero signs' relative order; reversing
  an ascending array would violate this. NaNs always last, including descending.
- Entire invalid source must fail even for empty slices; both bounds evaluate in
  order, and concat's second list is validated before combined count/byte errors.
- Escaped Text/Float borrowed lists may exceed canonical bounds; small slices
  succeed while oversized complete outputs fail, with no Text allocation.
- Direct aliases may reuse any list input destination; local recursive frames and
  result copies must not point into released scratch.
- Preserve strict Core ownership/secret reads for both Int bound trees, reject
  wrong sort metadata/orders and punctuated malformed descriptive forms.

## Evidence ledger

Preflight: current code/docs examined; Python primary sequence/sort documentation
confirms clamped step1 slicing and stable ascending/descending sorting. Explicit
AIL NaN-last policy resolves the unordered Float case. No unresolved interface
or ownership conflict before execution. Baseline: /tmp/ail-list-transform-baseline.

### Implementation progress

- Frontend RED: 19 existing list tests passed, two new tests failed for missing
  source/opcode support; `/tmp/ail-list-transform-frontend-red.log`.
- Independent RED: public source concat rejected with AIL-PARSE-001 before hooks;
  `/tmp/ail-list-transform-red.log`.
- Native RED: raw LIST_CONCAT was unsupported before fields/dispatch/helpers.
- Root wiring is implemented across all planned files. Value function parsing
  now tries List before Text, matching action parsing, because the Text parser's
  malformed-slice guard otherwise catches valid `slices list` forms. List slice
  recognition is specific enough to preserve `slices text`.
- List Statistics source and STE are written. Source, saved Core and rendered
  source, each directly and via STE, produce the expected VM JSON report under
  `/tmp/ail-list-transform-example`: count5, median2, sorted[1.5,1.5,2,3,9],
  highest[9,3,2]. Native and independent generated report checks remain pending.
- Chapter50, source/schema/STE/list/Float/function references and indices are
  written; relative links in the chapter, example and changed indices resolve.
- Native worker's first raw-operation test is GREEN. Focused boundaries and
  independent reviews are still in progress; no task is marked complete yet.

### Verification and reviews

Task3: complete. 12 independent integration tests pass in
`/tmp/ail-list-transform-integration-final.log`; strict focused Clippy and Rust/
Python syntax checks pass. Full oracle `/tmp/ail-list-transform-oracle.log` passes:
1,500 typed datasets covering both sort directions, concat/slice/reverse and six
alias/snapshot variations; 29 ordered-error cases; 20 malformed UTF-8 probes;
19 exact byte/item/canonical-expansion/priority checks; exact64MiB arena/return
boundaries for all12 operation/type combinations and no Text charge; twelve
65,536-item sorts; 180 generated source operations through all12 public routes;
55 independently calculated statistics reports through those routes plus four
malformed/overflow datasets that preserve previous report bytes.

Root independently checked the sample report on all six native artifact routes,
bringing its example check to12 executions. All match the expected count5,median2,
sorted and highest lists. All13 prior differential runners pass in
`/tmp/ail-list-transform-regression-*.log`, including text transformations.
The frozen benchmark remains16/16; broad differential remains33agreements,
30unsupported native cases and zero divergences. No benchmark content changed.

Native implementation is released with12 new focused tests,11 existing list
regressions and strict library Clippy passing. Frontend implementation is released
with32 focused list tests and strict library Clippy passing. Both implementation
reviews are now in progress; final all-target suite follows review fixes, if any.

Native independent review approved the implementation after12 focused tests and
78 separately calculated comparator/parity probes. It checked merge-buffer
copy-back, descriptor/text lifetimes, signed Int and unsigned UTF-8 comparison,
Float zeros/NaNs, stable ties, arena accounting and recursive frames. Evidence:
`/tmp/ail-list-native-review-focused.log` and `/tmp/ail-list-native-review.log`.

The oracle worker approved root integration, parser ordering, documentation and
the example after26 existing Text compatibility tests,28 malformed-body rejection
probes,16 unrelated-identifier probes and byte-identical STE regeneration.
`/tmp/ail-list-transform-oracle-review.txt` records the review. Its minor finding
about stale agent-guide coverage has been addressed; its documentation recheck
approved the revised guidance and all24 relative links.

The frontend/root review identified two P2 local LIST_SLICE Core validation gaps:
the start/end operands may share an expression tree, and raw oversized integer
literals can normalize below the expression-byte bound. Both reproduce through
direct and saved Core in `/tmp/ail-list-frontend-review.rs`. The frontend worker
owns the scoped fix and regression tests in `tests/list_transform_boundaries.rs`;
the original reviewer will recheck it before the final serial suite. Other
reviewed frontend/runtime behavior passed, including seven focused tests.

The fix now validates each list-bound tree's raw token bytes, node/depth limits
and exact parent identity before decoding normalizes literals. Local slice roots
must have the correct named operand role and function owner; descendants cannot
be shared. Ordinary integer function decoding is unchanged. The new regression
target first produced3 passes/4 failures, then7 passes; exact65,536-byte inputs
and ordinary local function round trips remain valid. Together with32 list unit,
12 list integration and15 value-function integration tests,66 focused tests pass.
Strict focused Clippy and formatting pass. Evidence:
`/tmp/ail-list-local-bound-{red,green,list-regression,integration,clippy}.log`.

The original reviewer independently recompiled the original reproducer and
confirmed both findings are rejected through direct and saved Core, reran the
seven regressions and32 list unit tests, and approved the fix without new findings.
All implementation/review tasks are released. The final serial suite is running
against the file hashes in `/tmp/ail-list-transform-tested-inputs.json`;
`/tmp/ail-list-transform-post-review-tests.log` records its output.

### Final verification

The reviewed implementation passes **1,108 tests across25 targets**, with zero
failures and zero ignored tests. The complete serial command was
`cargo test --all-targets -- --test-threads=1`; it exited0, including the CLI
localhost/socket tests. All68 hashed compiler/test/build inputs remained
unchanged throughout this final run. Strict all-target Clippy, `cargo fmt --all
-- --check`, `git diff --check` and `cargo build` also exited0. Logs:
`/tmp/ail-list-transform-post-review-{tests,clippy,fmt,diff,build}.log`.

After the Core fix, the complete independent transformation oracle passed again,
including1,500 typed datasets, maximum65,536-item sorts, exact arena boundaries,
180 source operations on12 routes and55 independently calculated file reports.
The existing list and value-function oracles also passed again to cover the
shared validation helper and local decoding. Logs:
`/tmp/ail-list-transform-post-review-{oracle,list-oracle,value-oracle}.log`.
The other prior oracles and frozen16/16 benchmark passed earlier in this milestone;
the narrow Core fix introduced no runtime changes. Python oracle syntax and29
links across the new chapter, agent guide and example README were verified.

All six tasks and independent reviews are complete. The example demonstrates
actual typed collection processing and JSON file output through Specs, Core,
STE, VM and standalone native ELF. This completes the list-transformation
increment only. General Python replacement, broader types/libraries/effects
and wider agent workflows remain active objectives; no full-language or complete
ASD-STE100 conformance claim is made.
