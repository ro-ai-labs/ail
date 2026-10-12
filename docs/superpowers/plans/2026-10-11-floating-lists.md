# Floating lists implementation plan

Spec: ../specs/2026-10-11-floating-lists-design.md
Baseline: verified scalar Float milestone,923tests, frozen16/16. Prior goal turn
classified as progress using the current worktree and its verification record.

## Constraints and execution

Preserve existing Int/Text list grammar, numeric semantics, immutable aliases,
shared budgets, ownership, profiles and no-Float native path. List<Float> retains
IEEE specials internally; JSON/literals are strict and finite. No downloads,
external interpreter, runtime files or AIL-compile subprocesses. Existing Rust
build embeds the additional allocation-free numeric helper.

Ruling: continue authorized work in the existing shared worktree; .git is
read-only and prior uncommitted work must be preserved. User's ongoing goal and
developer autonomy authorize implementation without repeating skill approval
pauses. Keep spec/plan as reviewable repo artifacts; do not commit/reset/clean.
The subagent-driven-development workflow authorizes independent worker tasks.

## Ownership / interface map

1. List/frontend/VM worker: src/list_operations.rs and focused
   src/float_operations.rs parent-ownership extension. Add AilListValue::Floating
   (FloatExpr), element Float throughout source/Core/opcode/runtime, private
   Item::Floating(u64). canonical_literal(text,"Float") canonicalizes runtime
   list grammar including specials; validate_runtime_value(text,"Float") checks
   borrowed input without canonical bound. Add
   canonical_float_json(text:&str, parsing:bool)->Result<String,String> returning
   LIST/LIST-LIMIT errors: true requires strict finite JSON numeric tokens, false
   parses runtime Float tokens then rejects nonfinite values. Root maps these
   errors to JSON and uses this for literal/JSON boundaries. Use root's
   float_number::parse_json. Preserve65,536 flat items independent of Json's
   recursive node count. Extend accesses traversal for unary_operand. Focused
   tests owned in these modules.
2. Native worker: src/native_list.rs, src/native_expr.rs,
   src/native_value_function.rs, src/native_json.rs, src/native_json_mutation.rs,
   src/native_float.rs if needed, and focusednative tests. Element code2 Float;
   conditional helper emission preserves old integer-only programs. Use shared
   parse_json runtime symbol with the same integer-bits/status ABI as parse.
   Helpers can add separate strict/finite list parse modes for JSON; coordinate
   internal ABI explicitly. All scalar and list allocation/alias rules preserved.
3. Independent integration/oracle worker: tests/floating_lists.rs and
   scripts/ail_float_list_differential.py. No production edits. Exercise real
   source/Core/STE/VM/native and Python comparisons, malformed input/contracts,
   functions, JSON/file reports and boundaries. Report concrete defects.
4. Root: float_number.rs, runtime/float_runtime.rs, build.rs,
   native_float_runtime.rs helper export/tests; all root/protocol/value-function/
   value-VM/literal/JSON/structured-JSON/mutation integration; docs/examples/STE.
   Root tests may use tests/floating_list_pipeline.rs to avoid worker ownership.

Shared interfaces checked: list Core roots require Float checker list_value
parent support; root local operand schema must use FloatExpression for Float
append/set; native fields and dependency collection must include those Float
leaves; JSON parser must use strict finite mode rather than runtime canonical
parsing; function returns must use runtime grammar rather than JSON finite mode.
Existing 64-byte scalar layout and numeric owned-buffer flag are unchanged.

## Tasks

- [x] Red tests for strict numeric helper, list runtime/source/Core and native.
- [x] Core-only strict JSON number API/export and direct native proof.
- [x] Float list source/Core/STE/protocol/VM and function/JSON integration.
- [x] Native lists, functions, JSON and conditional numeric runtime dependencies.
- [x] Independent public oracle, numeric list file example and docs/exported STE.
- [x] Independent reviews and fixes; full serial suite, strict lint/fmt, prior
  regression oracles and frozen benchmark.
- [x] Record evidence; keep Python-replacement goal active.

## Implementation and review evidence

The strict numeric API rejects permissive Float tokens at JSON boundaries and
preserves correctly rounded finite values, signed zero and subnormals. A direct
standalone ELF test checks the new parse_json ABI and unchanged output storage
on error. Meaningful red/green logs are
`/tmp/ail-floating-list-number-{red,green}.log` and
`/tmp/ail-floating-list-abi-green.log`.

The list worker observed missing Float support and a forged global-field
ownership gap before implementing and tightening validation. Its 18 module
tests, 10 prior list integration tests and 5 Float Core tests pass. The native
worker's 130 emitter tests pass, including nine new Float-list cases for
grammar, IEEE values, aliases, limits, functions, recursion and error priority.

Three additional findings were reproduced and fixed:

- Native finite formatting initially checked NaN before the full list count;
  mode 4 now completes parsing/count checks before scanning for nonfinite
  descriptors. Strict JSON mode 3 still validates each number when parsed.
- Direct STE LIST instructions lacked the program-profile guard already
  enforced by Core. The bytecode verifier now applies it to all five list
  opcodes and all three element types. The integration suite checks 60
  unsupported-profile cases plus Application/C interop acceptance.
- Legacy descriptive functions/compiler passes did not classify floating
  append/set as executable syntax. The shared descriptive-body guard now
  rejects those statements with both system/function subjects. Red logs:
  `/tmp/ail-floating-list-protocol-red.log` and
  `/tmp/ail-floating-list-descriptive-red.log`.

The independent integration suite has 14 passing tests, covering source, saved
Core, STE, imports, secrets, ownership, malformed protocol, functions, JSON,
files and full flat-item limits (`/tmp/ail-floating-list-integration-final.log`).
The independent Python oracle passes 650 operation pipelines, 672 grammar
mutations, raw/item/canonical bounds, exact 64 MiB shared allocation probes,
65,536-item strict arrays and 36 real file reports
(`/tmp/ail-float-list-oracle.log`).

The new `examples/floating-lists` package and exported STE calculate mean,
centering and population variance. All six source/Core/STE × VM/native routes
produce the exact same report and scalar outputs
(`/tmp/ail-floating-list-example-six-routes.log`). Empty datasets, overflowing
input numbers and nonfinite computed means fail on both backends before
changing an existing report file.

Independent numeric/native and frontend/root reviewers approve with no
outstanding findings. The native reviewer also ran 20 focused executions,
including old Int/Text programs without Float runtime dependencies. The
frontend reviewer independently reran both descriptive/profile regressions.

## Final verification

`cargo test --all-targets -- --test-threads=1` exits 0: **957 tests across 18
targets**, including 331 library, 434 toolchain and 14 floating-list integration
tests (`/tmp/ail-floating-list-final-tests.log`). The local-socket fixtures run
with the previously authorized sandbox escalation. Strict all-target Clippy,
formatting, diff whitespace checks, the build and Python syntax checks pass.
The main documentation/example pages' 62 local links resolve.

All prior independent list, scalar Float, typed JSON, recursive JSON, JSON
mutation, typed-literal and value-function oracles exit 0. Logs are
`/tmp/ail-floating-list-regression-{list,float,json,structured-json,json-mutation,literal,value-function}.log`.
The frozen capability ladder stays **16/16**; the broader differential reports
**33 agree, 30 unsupported natively, 0 diverged**
(`/tmp/ail-floating-list-regression-{bench,broad}.log`). No benchmark rungs or
expected results were changed for this milestone.

This is verified progress on the active goal, not Python replacement completion.
AIL now has numerical lists and reusable numerical transformations through the
checked Specs→Core→STE→VM/native pipeline. General collections, broader library
and runtime facilities, and full Python replacement remain unfinished.
