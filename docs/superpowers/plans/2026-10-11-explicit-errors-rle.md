# Explicit Errors and RLE Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans and
> superpowers:dispatching-parallel-agents for the disjoint tasks below. Keep
> RED/GREEN evidence and interface rulings in this ledger. No nested agents.

**Goal:** Let AIL libraries raise domain errors and implement the R027 binary codec
entirely in AIL, usable as imported functions and a binary-file tool.

**Architecture:** Extend existing control/recovery with one checked RAISE opcode;
reuse current native unwinding. A pure AIL RLE32 library exercises the new error
path and Bytes through all existing compiler stages.

**Tech Stack:** Dependency-free Rust2024, VM, direct Linux x86-64 ELF; AIL library;
independent sequential Python reference and actual binary-file comparisons.

**Spec:** [Explicit errors and RLE](../specs/2026-10-11-explicit-errors-rle-design.md).

## Global constraints

- Preserve shared worktree and all prior work; .git read-only, no commits/worktrees.
- RAISE category is exactly one of the existing14; same indices/any16383; no payload.
- Exact source, Core and STE schema from spec, all reachable/dead instructions checked.
- RLE32 count is nonzero big-endian u32; packet size5, canonical maximal encoder runs.
- Bytes raw bound1,048,576/shared arena67,108,864; no smaller arbitrary codec bound.
- No dependencies or host codec/eval/process shortcut. Old ladder inputs frozen.

## Review focus

1. A raise-only function is valid, but an unraised fallthrough path still rejects.
2. Raise inside recursive/recovery frames unwinds output/depth/handlers correctly;
   later side effects are suppressed and earlier effects remain.
3. Malformed reserved forms/dead bytecode/saved Core cannot smuggle a dynamic,
   fatal or multiple category through existing metadata/ownership schemas.
4. Equal runs spanning any encoder split merge canonically; valid adjacent decoder
   runs and all256 payload values survive exact file and imported-call routes.
5. Complete decoder format validation precedes expansion; large counts/size sums
   cannot overflow, exhaust memory before the size check or truncate an output file.

## Task 1: Raise frontend and VM

**Owner:** function_core.
**Files:** src/control_flow.rs, recovery.rs, ail.rs, integer_function.rs,
integer_vm.rs if present, value_function.rs, value_protocol.rs, value_vm.rs, ste.rs;
tests/explicit_errors.rs. No native files.
**Interface:** AilControl::Raise { category:String }; opcode RAISE exactly
category; control_kind=raise, error_category; STE `Raise error {category}.`.

- [x] Write public source/Core/STE/protocol/error tests; record expected RED.
- [x] Implement parser/render/Core validation/ordering/import/function classification
  and protocol validation; reject invalid/dead forms; all-raise terminal paths valid.
- [x] Execute on action/integer/value VM through existing category propagation.
- [x] Run focused frontend/VM tests; report owned files, checks, hashes to
  /tmp/ail-rle-task-1-report.md. Coordinate native compile interfaces with Task2.

## Task 2: Native raises and independent protocol acceptance

**Owner:** native_control.
**Files:** src/native_expr.rs, native_value_function.rs, native_recovery.rs and
other native-only files as needed; tests/explicit_errors_native.rs.
**Consumes:** Task1 RAISE category and unchanged recovery categories.

- [x] Write RED native tests for all14 categories, nested/re-raised/recursive and
  integer/value function paths, raise-only bodies, later effects/result suppression.
- [x] Emit direct category error jumps through existing recovery machinery; preserve
  frames/fuel/output and fatal diagnostics. No new ABI or category index.
- [x] Confirm whole-program malformed raises reject before native effects and run
  focused native/recovery tests; report /tmp/ail-rle-task-2-report.md.

## Task 3: Independent R027 oracle and differential acceptance

**Owner:** condition_ast.
**Files:** bench/R027-run-length-codec/{intent.md,reference.py,cases/*.args},
scripts/ail_rle_differential.py. Do not write program.ail or library files.
**Consumes:** spec's format/entrypoints only, not AIL algorithm for expectations.

- [x] Author sequential Python oracle and broad cases; freeze hashes and run rung
  RED as unimplemented before notifying root implementation can begin.
- [x] Build independent source/Core/rendered/STE/bytecode VM/native routes and file
  checks, including all256 values, seeded runs, malformed inputs/size boundaries,
  exact1MiB decoded bytes and output-preservation failures. Use bounded subprocesses.
- [x] Add mutation sensitivity, run acceptance after integration, preserve frozen
  rung inputs; report exact checks/seed/fingerprint /tmp/ail-rle-task-3-report.md.

## Task 4: Pure AIL library and binary-file app

**Owner:** root.
**Files:** examples/ail_std_rle.ail/{ail-package.md,spec.ail-spec.md},
examples/run-length-codec/{ail-package.md,spec.ail-spec.md,README.md},
bench/R027-run-length-codec/program.ail/{ail-package.md,spec.ail-spec.md},
examples/ste/run-length-codec.ail-ste.

- [x] Wait for frozen R027 RED; implement pure encodedSize/encode/decodedSize/decode and helpers
  using balanced composition, bounded leaves and repetition doubling.
- [x] Implement Transform/EncodeFile/DecodeFile, preserve outputs until successful
  decode/encode, verify hand-derived and frozen cases then actual binary file checks.
- [x] Export exact STE and document runnable commands, format and cumulative limits.

## Task 5: Independent review, documentation and final gates

**Owner:** root with reciprocal worker review after owned work is released.
**Files:** active references/indexes, docs/ail/57-explicit-errors.md,
docs/ail/58-run-length-codec.md, bench/LADDER.md/JOURNAL.md/SCORE.lock.

- [x] Review frontend/native/library/oracle independently, resolve findings with
  RED/GREEN evidence and freeze reviewed implementation.
- [x] Run final fmt/diff/build/strict Clippy/all-target tests, new and prior
  differential scripts, prior17+rung benchmark; promote only new rung to18/18.
- [x] Verify exact STE/docs commands/links and unchanged baseline old oracle/bench
  inputs; record evidence and leave the full Python-replacement goal active.

## Ledger

Previous goal turn: progress. Rehashed146 Bytes milestone inputs unchanged.
Baseline: /tmp/ail-rle-baseline.json and /tmp/ail-rle-baseline/ copies.
Ruling: continuing authorization supersedes repeated skill approval requests.
Use explicit parallel agent tasks for separate frontend/native/oracle ownership;
root owns pure AIL application and docs. No product files shared between workers.
Spec/plan self-review: exact category/protocol/format/bounds/entrypoint contracts
cover all tasks; validation and runtime interfaces agree. No unresolved choices.

| Tasks | Interface check |
| --- | --- |
| 1↔2 | One RAISE/category opcode, existing error labels and14 indices; native files separate. |
| 1↔4 | Same source raise syntax in actions/functions; all-raise functions accepted. |
| 3↔4 | Freeze format/mode/result fields before library; Python sequential oracle independent. |
| 5↔1/2/3/4 | Review after release; preserve baseline and old rung score entries. |

Ruling: use existing Core kind Control for raise, correcting the design's initial
ControlOperation spelling; adding a separate family would duplicate the existing
control schema. Reject reachable fallthrough in raise-containing functions without
changing legacy no-raise functions. Preflight encodedSize before constructing an
encoding so an oversized result rejects before expensive output allocation.

R027 RED:146 independent cases/148 frozen inputs, fingerprint
sha256:2084c99467f7ff315c452cd604436b08; runner exit1 unimplemented before
program.ail existed (/tmp/ail-rle-rung-red.log). Freeze manifest
/tmp/ail-rle-rung-frozen.json. Root library/app implementation now present; first
check correctly rejects unsupported raise while frontend integration is pending.

Root initial GREEN: source check and hand-derived AAABB packet/restore passed.
Documented file commands independently produced exact bytes in VM/native; exact
STE export/execution also passed (/tmp/ail-rle-doc-example.json,5 checks). First
independent fixed oracle:146 cases×12 routes=1752 checks passed; broader random,
size, mutation and file checks remain in progress. Native Task2 released with
7 public tests,14-category raw ELF probe,15 recovery regressions and strict Clippy.

Review resolution: missing data.mode originally raised TEXT while the frozen
reference returned MODE. Root RED /tmp/ail-rle-mode-red.log, narrow app/benchmark
mode classification try/recover added before codec dispatch,6 VM cases GREEN
/tmp/ail-rle-mode-green.log. Unexpected TEXT from inside the codec still escapes.
STE regenerated. Oracle review found prefix category matching could accept
BYTES-LIMIT for BYTES; Task3 corrected it with exact-boundary RED/GREEN checks.
R027 initial benchmark now passes146cases VM/native (/tmp/ail-rle-bench-first.log).

Verification scheduling: all implementation and oracle files released/frozen.
Start final Rust checks and suite against these hashes while the last independent
frontend/docs review finishes; a review-driven product edit will invalidate the
affected checks and require rerun. Native/oracle review approved after exact
error-boundary fix; native/library large-value probes GREEN. Manifest
/tmp/ail-rle-reviewed-inputs.json records the inputs before final gates.

Task3 released:149 owned hashes unchanged; fixed1788, generated2880, real-file264,
size-helper24, exact-category18 checks and both mutations/budget probes GREEN.
/tmp/ail-rle-task-3-report.md records details. Independent native/oracle review
approved /tmp/ail-rle-review-native-oracle.md. Docs review approved after fixing
stale no-raise wording and central diagnostics/STE entries;17 reviewed documents,
258 resolving local links (/tmp/ail-rle-review-docs.md). Final global gates running.

First full gate stopped on15 ail_toolchain failures caused by unregistered
support library and missing package guide/count metadata. Preserved evidence:
/tmp/ail-rle-attempt-1-tests.log and -gates.json. Added support manifest entry,
required library README guide and inventory/list coverage. Existing package-count
assertions now compare documentation to the discovered directory count, preserving
all per-package coverage and the frozen128 corpus invariants. Focused3 package
inventory tests pass; real corpus replay also rerun before restarting final gates.
Final runner now promotes the new rung with --update-lock only after all previous
gates pass; no --bless-ladder and all17 old score entries must stay exact.

All independent reviews approved: frontend and library reports from native_control;
native/oracle and docs reports from function_core. Missing-mode, exact-category
and inventory fixes all rechecked independently. Inventory appendix confirms28
packages,4 distinct focused tests,9 new guide links and493 frozen replay inputs
unchanged. Compiler/library production hashes remain frozen; only registration,
guides and test documentation-count assertions changed for the rerun.

Final rerun Rust gates GREEN: fmt/diff/build/strict all-target Clippy and all1372
Rust tests in42targets,0failed/0ignored (276.87s). Test counts stored in
/tmp/ail-rle-final-test-counts.json. All21 differential suites now running;
benchmark promotion and broad comparison follow only after successful suites.
Current docs check:20 documents,272 local links, all resolve.

Final milestone gates GREEN: all28 commands pass. All21 focused differential
suites pass, including RLE5064checks/5113commands, seed0xa11e027 (690.57s).
The full benchmark passes18/18 (429.58s); R027's146 frozen cases pass VM/native.
SCORE.lock retains all17 previous rung entries exactly and adds only R027's
frozen fingerprint. Broad comparison:33agree,49not-root-compilable,0diverged
(62.11s); the nine additional non-root entries are imported library functions
covered by the dedicated suite. All275 prior benchmark/script inputs and all148
frozen R027 inputs remain unchanged. Cargo dependencies remain unchanged.

README score and append-only benchmark journal updated. Exact regenerated STE
matches the checked-in16211bytes. The final closeout report and input hashes are
/tmp/ail-rle-final-summary.json and /tmp/ail-rle-final-inputs.json; refreshed local
documentation links are recorded in /tmp/ail-rle-doc-checks.json. The resolved
inventory failure remains preserved in attempt1 evidence; the final Rust rerun
passes1372tests in42targets. All independent reviews are approved, with no open
findings. This completes the explicit-errors/RLE milestone; the full user's
Python-replacement and AI-agent language goal remains active.
