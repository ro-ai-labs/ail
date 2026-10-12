# Typed Control Flow Implementation Plan

> Execute in the current authorized worktree using test-driven changes and
> independent module work, then review the integrated result.

**Goal:** Typed comparisons and real source branches/loops through STE and ELF.
**Architecture:** Extend ordered actions with typed control markers, validate
structured blocks in Core, lower to verified branches and jumps.
**Tech stack:** Dependency-free Rust, existing VM and direct x86-64 emitter.
**Spec:** `docs/superpowers/specs/2026-10-11-control-flow-design.md`

## Constraints and review focus

Keep frozen oracle/case fingerprints. Preserve previous arithmetic/STE changes.
Test MIN/MAX signed comparisons without overflow, skipped erroneous expressions,
nested loop targets, per-action label scope, duplicate/invalid delimiters,
Core root mutation and missing operand edges, imported field references, malformed
step metadata, and nonterminating loops. Output declarations stay outside blocks.

- [x] Add failing end-to-end control-flow tests and frozen-rung implementations.
- [x] Implement bounded IntCondition parser/encoding/evaluation in condition.rs;
  unit-test all six comparisons, precedence, ambiguous chains and bad operands.
- [x] Extend native_expr.rs with BRANCH_INT; test actual ELF results and errors
  against independent expectations and the VM, including signed limits.
- [x] Add AilControl, typed Core nodes/edges, source rendering/imports and ordered
  control steps. Check block structure/types/limits before compilation.
- [x] Lower if/else/while/break/continue to labels/jumps; integrate STE, verifier,
  VM, native dispatch and inlined branch-label prefixes.
- [x] Run focused/full tests, Clippy/formatting and independent review; fix defects.
- [x] Record verified capability score and remaining full-goal gaps in docs.

## Verified outcome

2026-10-11: typed signed comparisons, source branches and loops, break/continue,
saved Core/source round trips and imported conditions execute through STE on
the VM and native ELF backend. Source block structure and types are checked
before execution; invalid bytecode operands and labels are rejected.

Independent review found and regressions now cover malformed source/Core repeat
counts, misplaced comparison edges, compiler-pass actions that would discard
controls, and native fuel accounting for calls inside loops. Native calls now
save and restore per-invocation budgets using the original instruction counts,
including removed returns and repeated calls. Return fuel is checked before
callee output validation. An existing corpus form, `times sequentially`, now
executes its repeat instead of being silently discarded.

Validation:

- `cargo test -- --test-threads=1`: 570 passed (90 library, 434 existing
  integration, 17 control-flow, 17 integer and 12 STE integration tests).
  Local mock HTTP listeners required the previously authorized sandbox
  escalation. Log: `/tmp/ail-control-flow-verified-tests.log`.
- The initial parallel full run exposed the repeat corpus form and one
  transient Linux `ETXTBSY` in an existing native executable test. The corpus
  defect was fixed; the complete final run was serialized. The new native
  integration helper also retries only that transient OS error, with a bound.
- `cargo clippy --all-targets -- -D warnings`, formatting and diff checks pass.
- `python3 scripts/ail_differential.py`: 33 agree, 30 not natively compilable,
  zero diverged.
- Frozen capability ladder: 14/16, with R003, R004, R007, R008, R009 and R010
  added. Case/oracle fingerprints are unchanged.
- `examples/ste/sum.ail-ste`: checked and executed on VM/native with
  `n.value=100`, yielding `out.total=5050` on both.

The broader goal stays active. R005 recursive factorial and R006 Fibonacci
still require actual value-returning call frames; legacy function trace
instructions do not implement them. Collections, general I/O, arbitrary
precision, ecosystem compatibility and full ASD-STE100 conformance remain open.
