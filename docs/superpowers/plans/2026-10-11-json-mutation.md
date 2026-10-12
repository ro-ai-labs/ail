# JSON mutation implementation plan

**Goal:** Construct and transform nested Json records with immutable set/remove.
**Architecture:** Focused frontend/Core module reuses the recursive value engine;
native mutation reuses the direct x86 tree/encoder helpers. Root wires ordering,
STE, profiles, secret analysis and documentation.
**Tech stack:** Existing dependency-free Rust compiler, VM and direct Linux ELF.
**Spec:** ../specs/2026-10-11-json-mutation-design.md

## Constraints and review focus

Preserve all existing work; .git is read-only. No new runtime dependency.
Use source/path/target/value/result/budget validation order from the spec.
Pay particular attention to replacement/removal at structural maxima, aliases,
duplicate decoded keys, root paths, UTF-8 boundaries and hidden arena charges.
Tests must exercise these classes rather than merely mirror helper functions.

- [x] Frontend/runtime task: create `src/json_mutation.rs`, modify
  `src/structured_json.rs` to expose `mutate(input:&str,path:&str,replacement:Option<(&str,&str)>)->Result<String,String>`
  (replacement is type and runtime transport). Add AilJsonMutation enum
  Set{source,path,value,key}, Remove{source,path,key}, Invalid(String), plus
  source/parser/Core/check/from_core/namespace/instruction/verify/accesses/execute
  APIs following existing structured_json. Write failing tests then implement.
- [x] Native task: new `src/native_json_mutation.rs` and focused edits in
  native_structured_json/native_json/native_expr. Reuse bounded parsing/encoding,
  validate final structure, charge only one final result, preserve registers.
  Write red native tests then prove semantics/limits/budgets.
- [x] Root task: integrate in ail.rs/action_order.rs/ste.rs; expose
  `json_codec::format_runtime_value(type_name:&str,input:&str)->Result<String,String>`
  without budget charge. Ensure direct profile and function/pass rejection.
- [x] Independent tests `tests/json_mutation.rs` and
  `scripts/ail_json_mutation_differential.py`: full pipeline, negative Core/protocol,
  permissions/imports, independent Python mutation and file transforms.
- [x] Runnable record-transform package, exported STE, guide and updated indexes.
- [x] Independent read-only review; full serial Rust suite, strict lint/format,
  recursive/typed JSON and literal differential checks, frozen benchmark/corpus.
  Record evidence and keep the broader objective active.

Agents own disjoint domains above; root handles cross-module integration and
verification. Preserve exact error codes and limits from the spec. No approval
pause or commit is needed for this already-authorized implementation work.

## Verification evidence

- Integration RED: the initial mutation pipeline test failed with AIL-PARSE-001
  on the new source statement. Engine/frontend tests and the native unsupported
  opcode test also failed before implementation, then passed.
- `cargo test --all-targets -- --test-threads=1`: 822 tests across 14 targets,
  all passed (`/tmp/ail-mutation-full-tests.log`). Two further library regression
  tests were added during that run. The final `cargo test --lib --
  --test-threads=1` passed all 241 library tests, including both additions
  (`/tmp/ail-mutation-final-lib.log`). This verifies **824 current unique tests**.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo build`
  and `git diff --check` pass. Final Clippy log:
  `/tmp/ail-mutation-final-clippy.log`.
- Mutation integration: 13 tests. Independent Python oracle: 558 mutations,
  18 destination aliases plus source-as-value, source/path/target/value error
  ordering, native invalid UTF-8, 15 exact limits, shared 64 MiB final-only
  allocation, and 50 file transformations. Logs:
  `/tmp/ail-json-mutation-tests.log`, `/tmp/ail-json-mutation-oracle.log`.
- Independent read-only reviews found no defects in VM/frontend/root integration
  or native mutation. The native reviewer separately checked 931 cases against
  a pair-preserving Python implementation, including 374 exact-byte successes.
- Previous recursive JSON oracle passes 253 round trips, 666 typed lookups,
  100 counts, 27 malformed matrices, limits/budgets and 40 nested file datasets
  (`/tmp/ail-mutation-recursive-oracle.log`). Typed JSON and literal differential
  scripts also pass (`/tmp/ail-mutation-codec-oracle.log`,
  `/tmp/ail-mutation-literal-oracle.log`).
- Frozen benchmark remains 16/16. Corpus comparison: 33 agree, 30 unsupported
  native actions, zero divergences (`/tmp/ail-mutation-bench.log`,
  `/tmp/ail-mutation-corpus.log`). No benchmark inputs or lock changed here.
- `examples/json-transform` removes a private field, computes new amounts, adds
  flags, appends nested records and writes a complete report. Exact output and
  count/total are verified through source, saved Core and direct STE on both
  VM and native backends (`/tmp/ail-mutation-example.log`). Exported STE is
  `examples/ste/json-transform.ail-ste`; the guide is
  `docs/ail/43-json-mutation.md`.

The broader goal remains active: general value functions, additional numeric
computation and wider Python ecosystem compatibility are still required.
