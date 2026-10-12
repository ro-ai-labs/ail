# Typed lists implementation plan

**Design:** `docs/superpowers/specs/2026-10-11-typed-lists-design.md`.
Continue the user's authorized active goal in the current shared worktree.
Use the parallel-agent skill for independent frontend/native ownership; root
owns integration and end-to-end validation. Preserve all earlier work.

- [x] Establish failing source/Core/STE/VM/native list tests.
- [x] `src/list_operations.rs`: AST, source parsing/rendering, typed Core
  elaboration/reconstruction/checking, imports, derived access edges, strict
  bytecode encoding/verification, bounded flat JSON list parser and VM execution.
  Public AST contracts are in the design; module helper signatures mirror text
  operations, with instruction(op,document) for inferred element type and
  execute(instruction,state,text_allocated,list_allocated) for shared budgets.
- [x] `src/native_list.rs` and `src/native_expr.rs`: direct native list parsing,
  canonicalization, indexing, append/set, immutable arena and bounded scratch.
  Add real-ELF unit tests; preserve integer/text/fuel/output semantics.
- [x] Root integrates model/order/parser/render/import, Core catalog/saved edges,
  checking and secret-access rules, STE templates, bytecode verifier, source
  compilation and both shared VM budgets. Expose existing text budget helpers
  for List<Text> get. Do not permit silent omission by unsupported profiles.
- [x] End-to-end tests and independent Python comparisons, review and fixes,
  full suite, Clippy, formatting, frozen ladder and corpus differential.
- [x] Runnable examples, agent guide and typed-list reference; record verified
  results and keep the broader goal open.

Review focus: strict flat JSON grammar without recursive attacker input; copied
value lifetime; evaluation/error priority; canonical Unicode escaping; limits
and budget accounting across calls; secret expression leaves; saved duplicate
edges, profile omission and mismatched inferred element types.

## Verified outcome

- **661 tests passed** in `cargo test --all-targets`, including 9 list module
  tests, 11 new native list tests and 10 list integration tests. Existing mock
  HTTP tests used the authorized localhost sandbox escalation. Full log:
  `/tmp/ail-list-verified-tests.log`.
- Clippy with `--all-targets -- -D warnings`, formatting and diff checks passed.
- Frozen capability ladder: **16/16**, with no case or oracle changes.
- Existing corpus differential: **33 agree, 30 not natively compilable,
  0 diverged**. The existing 605 Unicode text oracle cases still pass, as do
  their malformed-UTF-8 and saved-Core secret-access probes.
- Independent review found no reproducible remaining defects. Its probes are
  preserved in `scripts/ail_list_differential.py`: **600 VM/native/Python list
  pipelines**, **1,600 JSON grammar mutations**, 37 malformed lists, six index
  failures and nine invalid native UTF-8 inputs all pass.
- Integration verifies integer map/reduction with function calls, Unicode
  escaping, source/saved-Core/STE round trips, imported actions, immutable
  copies, read-before-write aliases, exact operand checks, derived secret
  accesses, error priority and budgets shared across repeated action calls.
- Native read-only list operations accept valid transport whose canonical
  representation would exceed the result limit. A bounded descriptor/decoded
  scratch mapping defers canonical-size checking until a result is produced;
  replacing the expanding item with a small value therefore succeeds.
- Source and direct STE CLI execution/compilation produce the documented
  results in `examples/typed-lists` and `examples/ste/typed-lists.ail-ste`.
  The compile bundle's STE matches the exported example. Syntax, transport,
  Core semantics and limits are in `docs/ail/38-typed-lists.md`.

The broader active objective remains open. Flat typed lists add batch processing;
general source literals, nested collections, dictionaries, general I/O, richer
function/value types, arbitrary precision, Python ecosystem compatibility and
full ASD-STE100 conformity are not established by this milestone.
