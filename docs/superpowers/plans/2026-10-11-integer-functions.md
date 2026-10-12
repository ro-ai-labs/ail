# Executable Integer Functions Plan

**Goal:** Real recursive value-returning functions through Core, STE, VM and ELF.
**Spec:** `docs/superpowers/specs/2026-10-11-integer-functions-design.md`
**Architecture:** Typed ordered function body; isolated VM frames and native
machine calls; exact shared bytecode call protocol.
**Constraints:** Dependency-free Rust, checked i64, maximum 64 function frames,
unchanged frozen benchmark inputs, no legacy trace reinterpretation.

- [x] Establish failing factorial/Fibonacci and frame-isolation tests.
- [x] Implement source/body AST and typed Core module in `src/integer_function.rs`.
  Expose parsing, rendering, elaboration/reconstruction/checking, compilation,
  and argument encoding/decoding. Root integrates existing parser/model hooks.
- [x] Implement native function calls in `src/native_expr.rs`; expose
  `emit_with_functions(action, failures, functions)`, retain `emit` for existing
  callers. Use actual machine frames, checked depth and per-invocation fuel.
- [x] Integrate source/Core ordering and imports, exact STE templates, bytecode
  verification and isolated VM function execution. Test errors and early returns.
- [x] Review integration and fix defects; run full tests, Clippy, formatting,
  frozen ladder and corpus differential. Preserve unsupported diagnostics.
- [x] Record verified evidence, document function syntax and remaining goal gaps.

Review focus: duplicate names/arity and cross-function local edges; both recursive
results surviving one frame; root/global state unaffected by callees; missing
return and nontermination fail explicitly; saved/imported graph semantics match
source. Native and source/Core work use independent agents with exclusive files;
root owns integration and VM. This continues the user's authorized active goal.

## Verified outcome

- Full suite: **601 tests passed** with `cargo test -- --test-threads=1`.
  Localhost mock tests required the authorized sandbox escalation.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`,
  and `git diff --check` passed.
- Frozen computation ladder: **16/16**, with unchanged case/oracle fingerprints;
  `python3 scripts/ail_bench.py --update-lock` recorded the new best score.
- Corpus differential: **33 agree, 30 not natively compilable, 0 diverged**.
- Review regressions cover function dependencies in CLI bundles, unexpected
  function bytecode operands, integer-callee trace reports, mixed legacy and
  executable headers, duplicate saved-Core operands, and local ownership.
- The source, Core, STE and runtime contract is documented in
  `docs/ail/36-integer-functions.md`, with agent authoring guidance updated.

This completes the integer-function milestone, not the broader active goal.
Collections, useful text operations, general I/O, arbitrary precision and
broader Python compatibility remain open. Full ASD-STE100 conformity is not
established by the fixed executable STE templates.
