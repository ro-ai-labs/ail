# Float computation implementation plan

Spec: ../specs/2026-10-11-float-computation-design.md
Active objective: continue the STE-to-binary, agent-usable Python replacement.
Baseline: value-function milestone, 868 tests, frozen16/16, all changes retained.

## Global constraints and decisions

Preserve old integer source/Core/bytecode/native performance and all prior work.
No dependency downloads, shared-library/interpreter requirement, external AIL
compile subprocess, or ambient file capability. Float is binary64 with explicit
Int conversions, canonical scientific transport, signed-zero/subnormal support,
canonical NaN and zero-divisor errors. Same source/Core/STE/VM/native semantics.
Continue in the existing authorized worktree; .git is read-only, so no commits,
cleanup, worktree replacement or approval pauses. Specs/plan are reviewable
artifacts in that worktree, not a request to reconfirm ongoing authorization.

## Ownership and interfaces

1. Numeric/runtime worker owns `src/float_number.rs`, `runtime/float_runtime.rs`,
   `build.rs`, `src/native_float_runtime.rs`. Shared Float bits API:
   parse(&str)->Result<u64, Error>, format(bits,&mut[u8])->Result<usize,Error>,
   binary(op:u8,left:u64,right:u64)->Result<u64,Error>, compare(op:u8,left,right)->bool,
   from_integer(i64)->u64, to_integer(u64)->Result<i64,Error>.
   Error variants Invalid, Limit, DivisionByZero, Range, Buffer; fixed op codes
   Add0/Sub1/Mul2/Div3/Rem4; Eq0/Ne1/Lt2/Le3/Gt4/Ge5.
   Canonical NaN constant; public normalize(bits). Build-generated runtime
   metadata exposes absolute helper symbols plus segment bytes/flags/addresses.
   native_float_runtime exposes wrap(code:&[u8])->Result<Vec<u8>,String> and
   symbol(name:&str)->Result<u64,String>. Native code uses fixed exported ABI:
   parse(ptr,len,out_bits)->status; format(bits,ptr,capacity)->length or usize::MAX;
   binary(op,left_bits,right_bits,out_bits)->status; compare(op,left,right)->0/1;
   from_integer(i64)->bits; to_integer(bits,out_i64)->status. Status0 success,
   Invalid1/Limit2/DivisionByZero3/Range4/Buffer5. Function names `ail_float_*`.
2. Frontend worker owns `src/float_expression.rs`, `src/float_operations.rs`,
   `src/control_flow.rs`, `src/condition.rs` if needed. FloatExpr parse/from_postfix,
   to_source/to_postfix/fields/eval/map_fields analogous IntExpr, Eq via literal
   bits. FloatCondition analogous IntCondition. FloatOperation parse/source/
   elaborate/check/from_core/instruction/namespace/accesses plus local typed
   operation instruction schema. Extend AilControl IfFloat/WhileFloat and its
   Core/emitter checks, coordinate with root on new node/edge names and scopes.
3. Native worker owns `src/native_float.rs`, `src/native_expr.rs`,
   `src/native_value_function.rs`, `src/native_json.rs`, and focused
   `src/native_structured_json.rs`/mutation helper changes. Emit float expressions,
   controls/conversions, scalar Float call/return and JSON bridge; link embedded
   runtime only when needed. Wait on numeric worker's ABI but use stated API.
4. Root owns `ail.rs`, `ste.rs`, `main.rs`, `lib.rs`, `action_order.rs`,
   `value_protocol.rs`, `value_function.rs`, `value_vm.rs`, `integer_function.rs`,
   `literal_assignment.rs`, `json_codec.rs`, `structured_json.rs`, `json_mutation.rs`;
   coordinates action vectors/order/source rendering/imports and local operation
   schema, Float literal/JSON transports, example/docs. Root prepares integration
   tests until independent test/review slot is available.
5. Independent reviewer/oracle worker later owns `tests/float_operations.rs` and
   `scripts/ail_float_differential.py`, reviews runtime/native or frontend and
   verifies cross-route Python comparisons and previous regressions.

## Tasks and ledger

- [x] Observe focused red tests for numeric semantics, expression/Core/source and
  real native ELF before the respective implementation.
- [x] Numeric module and embedded no_std runtime, validated multi-segment wrapper;
  prove exact parse/format bits, arithmetic, conversions and standalone output.
- [x] Float AST, typed operations/control Core, source roundtrips and strict
  operand ownership/type/profile/secret checks.
- [x] Root source/Core/STE/VM/scoped-function/literal/JSON integration.
- [x] Native Float instructions, stable scalar frames and numeric/JSON helpers.
- [x] Public integration and independent Python oracle; numeric file example,
  exported STE and guide with current scope and primary semantic references.
- [x] Independent reviews, concrete findings fixed with red→green regressions;
  full serial suite, strict lint/format, existing oracles and frozen16 benchmark.
- [x] Record final evidence and keep the broader goal active.

Interface scan: numeric worker exports the fixed bits/status ABI consumed by
native worker; frontend emits the four declared opcodes consumed by root/native;
root owns global registrations and typed-local/JSON hooks. Native owns its JSON
assembly counterparts while root owns VM JSON. No simultaneous write ownership.

Review focus: exact halfway decimal rounding; native call-stack alignment and
fixed runtime segments; NaN/unordered and signed-zero behavior; Float-to-Int
2^63 boundary; global/local typed ownership and secret operands; nonfinite JSON
rejection without changing lossless Json number handling; no numeric arena charge.

## Implementation and review evidence

The numeric module shares allocation-free binary64 parsing/formatting/arithmetic
between the VM and a checked-in no_std runtime. Seven numeric tests include
20,000 seeded fmod comparisons and 20,000 format/parse bit round trips. Eight
runtime tests execute real standalone ELF calls, check statuses and SysV callee
registers, and reject malformed or overlapping ELF segments. Independent Python
numeric comparison passed 79,207 cases with seed `0xA11F10A7`; two separate build
directories produced identical embedded ELF and metadata bytes.

Native fields retain their existing 64-byte layout and owned-buffer flag. Float
function packets carry scalar bits; returned fields format into their own
32-byte buffers. No scalar operation or return charges the text/list arenas.
All 121 native emitter tests passed, including 13 new Float tests. The complete
focused Float library selection passed 41 tests; legacy control tests passed
17/17. Independent numeric/runtime/native review approved spec compliance and
code quality without outstanding findings.

`tests/float_pipeline.rs` checks source/saved Core/rendered source through STE,
typed functions and conversions, wrong types/hidden globals, and imported
Float operations/conditions/calls with repeated action invocation. Its three
tests pass. `tests/float_operations.rs` independently covers public source,
Core, STE, VM/native, binary64 special values and comparisons, casts, function
depth/unused-parameter validation, typed JSON, exact Core operands and secret
accesses, profile restrictions, and a file report. All eleven tests pass.

`scripts/ail_float_differential.py` passed with seed `252749584`:

- 1,195 input pairs, each comparing six arithmetic/function results and six
  IEEE comparisons against Python, including binary64 bits and signed zeros.
- 169 decimal transports, 23 malformed inputs, 214 conversion cases, and exact
  and exceeded 1 MiB Float parameter boundaries.
- 53 literals and aliasing function returns through source/saved Core/rendered
  source/STE in both the VM and native executable.
- 60 Unicode/NUL file transformations, retaining lossless neighboring Json
  numbers and checking nonfinite typed JSON rejection.

The Float measurement example and exported STE produce identical selected
outputs and JSON reports over source/Core/STE × VM/native. The report contains
four readings, one below freezing, mean `5.68625e1`, truncated mean `56`.

Concrete findings resolved with regressions:

1. The initial integrated source test exposed missing Float registration in
   the general known-type checker. Registering Float made the source/Core/STE
   pipeline pass without relaxing operand type checks.
2. Float JSON parsing rejected a canonical-expanding string as the wrong type
   before its size limit, unlike native parsing. A red VM test confirmed the
   mismatch. `encoded_size` now runs before scalar-type selection; VM and native
   regressions pass. General Json number tokens remain untouched.
3. Core literal normalization could hide an oversized raw Float expression.
   Raw byte accounting now precedes normalization and is shared between both
   comparison operands. The frontend also rejects orphan floating controls.
4. Compiler-pass Float operations incorrectly reported JSON diagnostics, while
   descriptive function bodies could silently accept `the function` Float
   operations and floating controls. Independent red regressions cover all
   three operations, both subjects and if/while in descriptive bodies. Dedicated
   rejection now reports `AIL-FLOAT-001`; all three focused tests and the full
   eleven-test integration target pass. Independent frontend/root review
   approved the fix and remaining spec/code-quality scope.

Per-run evidence is in `/tmp/ail-float-{oracle,restrictions-green,import-test,
example-six-routes,json-green}.log`. Existing literal, typed JSON, recursive
JSON, JSON mutation and value-function Python regression scripts all pass.
The frozen ladder is still 16/16; the broader corpus has 33 VM/native agreements,
30 unsupported native cases and zero divergences.

## Final verification

`cargo test --all-targets -- --test-threads=1` passes **923 tests across 17
targets**: 311 library, 0 main, 434 toolchain, 17 control, 16 files, 11 Float
operations, 3 Float pipeline, 17 integer expressions, 14 integer functions,
12 typed JSON, 13 JSON mutation, 10 lists, 13 literals, 12 STE, 14 structured
JSON, 11 text, 15 value functions. Local socket fixtures use the existing
authorized test escalation. Log: `/tmp/ail-float-final-tests.log`.

`cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`,
`git diff --check` and final `cargo build` pass. Logs:
`/tmp/ail-float-final-clippy.log` and `/tmp/ail-float-final-build.log`.
The public Float oracle and prior regression oracles above passed; the final
production change only tightens descriptive-source rejection and its diagnostic.

This completes the scalar Float milestone. The broader goal remains active:
typed floating collections, more reusable library operations and general
Python replacement are still beyond the verified scope of this milestone.
