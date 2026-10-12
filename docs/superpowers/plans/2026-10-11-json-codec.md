# Typed JSON conversion implementation

Design: ../specs/2026-10-11-json-codec-design.md

- [x] Frontend/Core/VM module `src/json_codec.rs`: source enum/parser, strict graph validation, exact bytecode, typed conversion, shared budget accounting, tests. Reuse literal/list/text validators where suitable without changing prior behavior.
- [x] Native `src/native_json.rs` plus focused edits in native_expr/native_list: typed parse/format, shared helpers, precise diagnostics and budgets, register preservation and native tests.
- [x] Root integration in ail.rs/action_order.rs/ste.rs: model, ordering, parser/rendering, graph schema/checks, imports, compile/verify, VM calls, secret reads/writes, native routing.
- [x] Independent `tests/json_codec.rs` and `scripts/ail_json_differential.py`: full pipeline and Python oracle; immutable aliases, malformed/boundary cases and file compute pipeline.
- [x] Runnable JSON file computation example and guide, STE export, indexes/agent guide.
- [x] Independent review, required full suite, strict lint/format and frozen ladder/corpus/prior differential checks; record evidence and keep broader goal active.

Agents own disjoint modules and tests; root integrates and reviews. Write meaningful failing tests before implementation. Preserve the current shared worktree and all previous milestones. Commits are not required for this task and .git is read-only.

## Verification — 2026-10-11

- `cargo test --all-targets`: **752 passed**, zero failed. This includes 11 JSON module tests, nine focused native codec tests and 12 JSON integration tests. Log: `/tmp/ail-json-verified-tests.log`.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check`: passed.
- `python3 scripts/ail_json_differential.py`: 500 Python/VM/native round trips, 132 malformed conversion cases plus missing/invalid-UTF-8 probes, 60 actual-file datasets, 13 exact/overflow boundaries and 64/65-result allocation-budget checks passed. Log: `/tmp/ail-json-differential.log`.
- Read-only independent review found no concrete integration or native-helper defects; an additional independent 423-case native/Python probe passed.
- Frozen benchmark: **16/16**. Existing corpus differential: **33 agree, 30 not natively compilable, zero diverged**. Existing file, literal, list and text differential scripts all passed. Logs: `/tmp/ail-json-{bench,corpus-differential,file-differential,literal-differential,list-differential,text-differential}.log`.
- `examples/json-batch` ran through source, saved Core and direct STE on VM and native ELF. Actual files matched Python: `[1,4,9,144]` and `158`; selected state reported count 4 and total 158. Checked-in STE matches compiler export exactly.

This completes the typed JSON conversion slice. The broader active goal still includes objects/nested values, richer function types, wider numeric support, practical Python compatibility and evaluation of the STE profile; it is not marked complete.
