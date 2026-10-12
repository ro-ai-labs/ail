# Structured runtime recovery implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development for the
> disjoint frontend, native and independent-oracle tasks. Root owns shared layout,
> VM/protocol integration and examples. Reuse available workers and independently
> review each scope after release. No nested delegation.

**Goal:** Recover from explicit data-error families in real batch programs across
Core, STE, VM and standalone native ELF, including calls and resource cleanup.
**Architecture:** Structured delimiters lower to a shared verified lexical layout.
VM execution catches at instruction boundaries; native handler records unwind
frames/output scopes and reclaim tracked temporary mmap payloads.
**Tech Stack:** Dependency-free Rust2024, direct Linux x86-64 ELF, Python oracle.
**Spec:** ../specs/2026-10-11-runtime-recovery-design.md

## Global constraints and rulings

Catch only INT,OVERFLOW,DIVZERO,FLOAT,BOOL,TEXT,LIST,INDEX,JSON,JSON-PATH.
`any` means that set. Store static `AIL-RUNTIME-<CATEGORY>` in declared Text.
No implicit retries, operand-content exposure, refunds of fuel/arena charges,
rollback of successful writes, dependency or runtime compiler/interpreter.
Existing64 control depth,64 function depth,1MiB value bounds and64MiB Text and
List/Json arenas remain. Limits/IO/capability/return/fuel errors stay fatal.
Malformed raw scope/jump protocol must fail before either backend runs.

Ruling: the continuing user goal and developer autonomy authorize this
architectural increment without repeating approval gates. Preserve this shared
uncommitted worktree and read-only .git; no worktree creation, commits, resets,
cleanup or branch integration menu. The plan is the persistent ledger and
`/tmp/ail-recovery-baseline` replaces a commit baseline. Wrong design choices
cost reversible rework; no external effects are authorized by this ruling.
The preceding turn is verified progress from current files, the completed list
plan and `/tmp/ail-list-transform-post-review-tests.log` (1,108 tests).

Ruling: disjoint frontend/native/oracle ownership permits parallel implementation
without shared-file edits. Existing workers are reused because fresh slots were
exhausted in earlier work. Root coordinates interfaces and performs only its
assigned integration; different workers review completed implementations.

## Preflight interface/ownership table

| Tasks | Shared interface | Ruling |
| --- | --- | --- |
| Root/frontend | exact AilControl variants and recovery.rs API in spec | root owns recovery.rs; frontend owns control_flow/integer_function/value_function |
| Root/native | Layout, masks, static codes and three opcodes | native calls shared analysis; root owns ail.rs entry/inlining guards |
| Root/frontend | ValueFunctionStep recovery field schema | frontend owns local Core; root owns value_protocol/value_vm |
| Native/oracle | scratch cleanup, fuel/frame/output restoration | oracle uses public behavior and independent resource limits |
| Root/oracle | recovering-json-lines source/report contract | root writes example, oracle derives report expectations |
| All | file ownership within each task | no overlap; request narrow handoff if one becomes necessary |

Read-only feasibility found and resolved the load-bearing design issues before
implementation: native throw-time fuel registers are scratch; use live cells.
Temporary JSON frees are non-LIFO; use a doubly-linked registry. Action VM calls
lose moved state on errors; retain mutable state/trace. Return errors must be
caught before leaving the frame. Function Text locals already promote execution.
Frontend structural checks must retain a single output Vec for begin patching.

## Tasks

- [x] 1. Root: create src/recovery.rs and register it in src/ail.rs. Write failing
  tests for exact family/mask parsing and lexical layout before implementation.
  Implement the exact shared spec API, balanced delimiters/depth, Protected vs
  Recover identities, jump-prefix checks, exact operands/key syntax and matching.
  Connect AILBC019 verification, Application/C interop guards, required operands,
  action/native-entry/inlining admission and STE templates in src/ste.rs.
  Add key:Text schema in src/value_protocol.rs; reject own INTEGER_FUNCTION
  handlers in the general verifier and existing integer opcode allowlist.
  Run focused recovery/STE protocol tests in tests/recovery_protocol.rs.
- [x] 2. Frontend worker: own src/control_flow.rs, src/integer_function.rs and
  src/value_function.rs. RED source/Core tests first. Add source/enum/rendering,
  strict global/local Core roles, namespace/access semantics, lexical lowerer
  begin patching and correct nesting with if/while/break/continue/return. Preserve
  Text-based function promotion. Use the exact spec APIs/templates/metadata;
  signal root's required catalog/semantic-write/legacy guard integration points.
  Cover direct/saved Core forgeries, normal source/render roundtrips, imports and
  malformed forms. Focused tests and owned-file formatting only; no full suite.
- [x] 3. Native worker: own src/native_*.rs and optional src/native_recovery.rs.
  RED raw recovery execution first. Implement handler records/dispatch, memory
  fuel cells, output-scope restorers, successful-return handler removal and jump
  exit cleanup. Add tracked temporary mmap/free helpers across all scratch users;
  persistent arenas stay untracked. Ensure Float result commit is atomic on all
  catchable paths. Cover category matching, nested/nonmatching handlers, calls,
  return-expression recovery, scope exits/reentry, output restoration, resource
  cleanup under constrained address space/FDs, fuel and repeated unwind depth.
  Do not edit ail.rs; report inlining/entry integration needs to root. Focused
  tests/Clippy only; no full suite or nested delegation.
- [x] 4. Root: refactor action VM internal execution to keep state/trace through
  nested action errors without per-step/full-state cloning. Add per-instruction
  result boundaries and recovery layout lookup; implement value_vm equivalent
  while keeping borrowed local ownership. Retain successful writes/effects and
  exact fuel/arena charges, errors on returns and fatal named-rule failures.
  Normalize ADD_INT_FIELD data errors to INT for VM/native parity. Complete Core
  edge catalog/secret-write/legacy guards from frontend handoff. Run focused
  nested-call recovery and old action/value regression tests.
- [x] 5. Oracle worker: own tests/runtime_recovery.rs and
  scripts/ail_recovery_differential.py. Observe RED via current public CLI first,
  then calculate independent expectations for every catchable/fatal class,
  normal/nested/recursive control, all scope exits and raw bytecode forgeries.
  Verify prior writes, destination atomicity, traces, exact arenas and repeated
  mapping/FD cleanup with native resource limits; bounded child timeouts. Test
  real JSON-lines reports across source/saved Core/rendered source, each directly
  and via STE, VM/native. No production edits or full suite. Document seed/counts.
- [x] 6. Root: write examples/recovering-json-lines, exported STE and chapter51;
  update source/Core/STE/function/failure references, indices and agent guide.
  Specify actual report/error-count behavior and independently exercise it.
- [x] 7. Independent frontend/root/native reviews, scoped fixes and re-review;
  final serial all-target Rust suite, strictClippy/fmt/diff/build, new/prior
  differential runners and frozen16/16 benchmark. Record evidence and leave the
  full Python/agent objective active.

## Review focus

1. Catch after repeated malformed list/JSON parses must free every temporary
   mapping, including older mappings removed before newer JSON mutation buffers.
2. A failed return or argument validation can recover in the correct live frame;
   successful returns/breaks/continues must never leave stale handler pointers.
3. Same-depth sibling jumps and jumps into recovery bodies are invalid, even
   when stack depths happen to match. Valid jumps exiting nested regions unwind.
4. Error handling inside tight loops consumes fuel; throw-site scratch registers
   must not reset it. Recursive propagation must restore depth and locals.
5. Nested action errors retain earlier data/file writes but restore caller output
   selections; catchable errors remain atomic at each failing primitive/call.

## Evidence ledger

Preflight native and frontend inspections completed read-only. Python primary
try-statement semantics consulted for termination/nearest-handler behavior;
AIL's fixed categories and fatal resource policies are explicit differences.
Spec/plan self-review confirms the source, Core, protocol and native APIs agree.

Shared protocol tests observed RED (three failing cases), then VM integration
observed RED (two failures). All five recovery_protocol tests now pass, alongside
two recovery unit tests and two STE template tests. Logs:
`/tmp/ail-recovery-protocol-red.log`, `/tmp/ail-recovery-vm-red.log`,
`/tmp/ail-recovery-vm-green.log`, `/tmp/ail-recovery-layout-unit.log`,
`/tmp/ail-recovery-ste-unit.log`.

Frontend implementation passed 29 focused tests, including 12 new recovery
tests, strict library Clippy and owned-file formatting. Inferred ordinary write
edges initially introduced action-order errors; recovery writes now use dedicated
derived access semantics. Report: `/tmp/ail-recovery-frontend-report.md`.

VM and native executions of recovering-json-lines produce the expected sample
report (seven rows, two accepted, five rejected). Independent Python expectations
pass for 60 generated reports across 12 routes, including malformed and oversized
input behavior. Evidence: `/tmp/ail-recovery-example-vm.json`,
`/tmp/ail-recovery-example-native.json`, `/tmp/ail-recovery-reports-oracle.log`.
All 31 checked documentation links resolve; SCORE.lock matches the baseline.

Independent integration tests exposed a pre-existing qualified integer-update
ambiguity when Data.value and Output.value coexist. Function_core temporarily
owns only the relevant src/ail.rs resolver/update functions and
tests/qualified_integer_updates.rs. Five tests reproduced the defect before the
fix (`/tmp/ail-qualified-updates-red.log`); root will wait for explicit release
before editing ail.rs. Existing source fixtures will remain unchanged.

Native implementation is released with 15 focused recovery tests passing,
183 native compatibility tests passing, strict library Clippy and scoped fmt.
The compatibility run preceded the final two test-only additions; all production
changes were included. Bounded native probes pass 1,500 catches under 192 MiB
address space and 24 file descriptors, and 100 repeated 64-frame unwinds.
Report: `/tmp/ail-recovery-native-report.md`. Reciprocal frontend/native reviews
are in progress.

Root action/control/function VM regression selection passes 51 tests across
four targets: `/tmp/ail-recovery-vm-regressions.log`.

Qualified-update fix passed all five new tests, 34 existing expression/control
regressions and strict library/new-target Clippy. The isolated four-hunk change
aligns exact qualified resolution, Core literal-update destinations and import
namespacing. `/tmp/ail-recovery-increment-fix-report.md` records diagnosis and
RED/GREEN evidence. Independent frontend/qualified review approved the result
after six valid routes, ten direct/saved Core forgery rejections and five malformed
source rejections: `/tmp/ail-recovery-frontend-review.md`.

The independent integration target passes all 14 tests. Full recovery oracle
passes with seed2931863058: 300 modeled nested-handler trees, 160 repeated deep
mixed-function unwinds, 4x800 constrained native cleanup probes, exact arena and
fatal allocation checks, repeated-action/jump-scope probes, public source routes
and 60 JSON-lines reports across 12 routes. Logs:
`/tmp/ail-recovery-integration-green.log`, `/tmp/ail-recovery-oracle-green.log`.
Example STE regeneration is byte-identical. Review corrected chapter51's Float
division-by-zero wording and chapter36's stale integer-only function claim.

All 14 previous differential runners pass, recorded in
`/tmp/ail-recovery-regression-summary.txt` and matching per-run logs. Frozen
benchmark remains16/16; broad differential remains33 agreements,30 unsupported
native cases,0 divergences. Strict all-target Clippy, fmt and diff checks pass.

Native independent review found one P2 boundary defect: a valid caller and
callee's recovery depths were added after action-call expansion, incorrectly
reapplying the per-action64 limit. Root added an internal expanded-layout entry
point that retains pairing/region/jump checks; public verification and value
functions still enforce64 for each original procedure. New regressions cover
caller1/64 plus callee64, both local and caller catches on VM/native, and continued
rejection of65 within one action. RED: six pass/one fail; GREEN: all seven protocol
tests pass. Logs: `/tmp/ail-recovery-inline-depth-red.log` and
`/tmp/ail-recovery-inline-depth-green.log`. Re-review is pending.

Native re-review approved the depth fix after the unchanged reproducer compiled,
15 native recovery tests and seven protocol tests passed, and nine independent
VM/native probes confirmed local return recovery for all eight value types and
caller output restoration. Report:
`/tmp/ail-recovery-native-independent-review.md`.

Root integration review also approved the final shared-layout API and VM/Core/
STE changes with no unresolved findings. Both documentation findings were fixed
and rechecked. Final-snapshot 14 integration plus seven protocol tests, fresh
build and complete recovery oracle pass: `/tmp/ail-recovery-final-scoped-tests.log`,
`/tmp/ail-recovery-final-oracle-build.log`, `/tmp/ail-recovery-final-oracle.log`.
Root independently inspected the oracle's Python category model, lexical-number
JSON decoding, duplicate-key handling, arithmetic and resource-limit assertions.
Report: `/tmp/ail-recovery-root-independent-review.md`.

Post-fix strict all-target Clippy, full formatting and diff checks pass. Captured
89 source/test/toolchain/oracle hashes in `/tmp/ail-recovery-reviewed-inputs.json`.

### Final verification

`cargo test --all-targets -- --test-threads=1` completed after all reviews and
fixes: **1,163 passed, zero failed, zero ignored across 28 targets**, exit0.
Evidence: `/tmp/ail-recovery-post-review-tests.log`. The final unchanged-input
check confirms all89 reviewed hashes still match. All39 relative documentation
links checked in chapters36/51, the STE authoring guide and example README resolve.
The frozen `bench/SCORE.lock` is byte-identical to the milestone baseline.

Post-fix logs: `/tmp/ail-recovery-final-clippy.log`,
`/tmp/ail-recovery-final-fmt.log`, `/tmp/ail-recovery-final-diff-check.log`.
The final rebuilt CLI passes the frozen benchmark16/16 and broad differential
33 agreements,30 unsupported native cases,zero divergences:
`/tmp/ail-recovery-final-bench.log`,
`/tmp/ail-recovery-final-broad-differential.log`.
The full new recovery oracle was rerun after the depth fix; all14 previous
runners had passed before that narrowly scoped recovery-only adjustment.

This completes the structured runtime recovery milestone, including the real
JSON-lines report example and qualified-update correction. It does not complete
the broader Python/AI-agent replacement objective or claim full ASD-STE100
conformance. The unbounded user goal remains active.
