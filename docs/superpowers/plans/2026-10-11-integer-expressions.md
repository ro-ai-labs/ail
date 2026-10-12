# Integer Expressions Implementation Plan

> Execute against the fixed interfaces in the design, with test-driven changes
> and an independent code review. Work in the current authorized worktree.

**Goal:** Runtime integer expressions across Spec, Core, STE, VM, and native ELF.
**Spec:** `docs/superpowers/specs/2026-10-11-integer-expressions-design.md`
**Tech stack:** Dependency-free Rust and existing direct x86-64 emitter.

## Constraints and review focus

Preserve previous STE work and frozen benchmark inputs. Do not substitute report
generation for computation. Check integer bounds, operand order, assignment
order, changed Core ASTs, output selection, Text preservation, and VM/native
agreement. The broader Python and full STE objective remains active.

## Tasks

- [x] Establish failing end-to-end arithmetic tests and frozen benchmark programs.
- [x] Implement the bounded expression AST, parser, encoding, and typed evaluator.
- [x] Implement native expression code generation with mutable runtime state.
- [x] Integrate typed Core nodes, source statements, STE, verifier, VM, and outputs.
- [x] Review fixes; run focused/full tests, Clippy, formatting, and benchmarks.
- [x] Record observed capability changes, semantics, and remaining goal gaps.

## Verified outcome

2026-10-11: source integer arithmetic, typed Core expressions, STE evaluation,
VM/native execution, explicit result fields and ordered writes/calls/repeats
are implemented. Native literal increments also use the checked scalar backend.
Forward action references and repeated operations survive saved Core. Callee
output scopes validate results and restore caller selection.

Read-only review found and regressions now cover Text literal increments,
imported dotted fields, source call ordering, missing callee outputs, Core depth
limits, and malformed ordered edges that previously caused indexing panics.
Typed-expression compilation now returns errors rather than emitting opaque
writes after a parser failure. Legacy Core serialization assertions now include
operation indices; native repeated arithmetic stdout asserts final state once,
while its existing trace assertion still checks all intermediate updates.

Validation:

- `cargo test`: 529 tests passed (66 library, 434 existing integration,
  17 integer integration, 12 STE integration). Full run permitted localhost
  mock servers; log `/tmp/ail-integer-verified-tests.log`.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo fmt -- --check` and `git diff --check`: passed.
- `python3 scripts/ail_bench.py --update-lock`: 8/16; frozen case/oracle
  fingerprints unchanged. Only implementation programs and score statuses
  changed. R000 through R002 pass their declared VM/native lanes.
- `examples/ste/add.ail-ste`: checked and executed with 7 and 9, result 16.

The overall goal remains active. The next capability gap is R003 comparison
and branching over computed values, followed by R004 sign/absolute value,
source loops and value-returning functions. General Python replacement,
arbitrary precision, collections, I/O, ecosystem compatibility, and full
ASD-STE100 conformance are not established by this stage.
