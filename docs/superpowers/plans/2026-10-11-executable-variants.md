# Executable Variant Values Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for
> each owned task. Independent compiler, runtime and oracle ownership follows
> superpowers:dispatching-parallel-agents; root integrates and reviews released work.

**Goal:** Execute user-defined recursive variant values through Specs, Core, STE,
VM and native ELF, demonstrated by a pure AIL binary search tree.

**Architecture:** Instantiate closed type schemas from existing declarations.
Three typed operations and schema-aware function boundaries use a shared no_std
transport engine; the native adapter embeds the same engine in standalone ELF.

**Tech Stack:** Existing Rust compiler/VM, direct x86-64 emitter, embedded no_std
Rust runtime, Python independent oracles. No added dependencies.

**Spec:** docs/superpowers/specs/2026-10-11-executable-variants-design.md

## Global Constraints

- Preserve prior work and all previous benchmark/oracle inputs. No git resets,
  commits or clean operations; authoritative baseline is /tmp/ail-adt-baseline.json
  with 1,741 copied files under /tmp/ail-adt-baseline/.
- Spec bounds: 8 MiB raw/produced ADT transport; 64 custom nesting; 65,536 typed
  values; shared 64 MiB composite arena; existing builtin internal limits.
- Schemas: depth32/type-name4096bytes/4096types/65536variants/65536fields,
  256variants per type/256fields per variant/4MiB binary table.
- VALUE category index14, any32767; VALUE-LIMIT fatal; AIL-VALUE-001/AILBC024.
- Native programs remain standalone and require no compiler/linker at AIL compile
  time. No type-specific tree opcode, sorting shortcut or host tree implementation.
- Keep full Python-replacement/AI-agent language goal active after this milestone.

## Review Focus

- A malformed unused recursive payload must fail a tag test or unused argument;
  owner Task1/2 tests full validation and parameter-before-depth ordering.
- Same-shaped nominal types must not cross typed source/Core/function boundaries;
  Task1 tests import-qualified and generic type distinctions.
- Nested generic commas, duplicate payloads and hostile Core ownership must reject
  rather than disappear during projection; Task1 negative roundtrip tests.
- Large Text/Bytes/Float/List/Json leaves must retain their existing semantics;
  Task2 compatibility tests and Task3 independent scalar/limit oracles.
- Repeated recovery from wrong variants or arena exhaustion must preserve callers,
  earlier effects, aliases and temporary cleanup; Task2 native/Task3 public tests.

## Shared interfaces

Frontend exports `AilValueSchemas`, a type alias for
`BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>`, as
`AilBytecodeProgram.type_schemas`. Empty metadata is omitted canonically.

`value_types` supplies `encode(&AilValueSchemas) -> Result<Vec<u8>, String>`,
`type_code(&AilValueSchemas, &str) -> Result<u32, String>` and
`indices(&AilValueSchemas, type_name, variant, field: Option<&str>) ->
Result<(usize, usize, Option<usize>), String>`. Use lexical schema/type/variant/
field order. Unknown names reject. Schema-aware value_protocol APIs add schemas
to signature, operand_schema and validate_body_instruction; signatures(program)
already has context. Frontend and native owners coordinate exact call signatures
before publishing changes. Keep the builtin predicate for leaf handling.

Native emitter `emit_with_functions(action, failures, functions, schemas)` receives
schemas for both entry paths, including function-free actions. All signature and
operation verification receives the same registry.

Shared engine structs use repr(C), eight-byte fields in this exact order:

```text
ValueSpan { data:*const u8, len:u64 }
ValueRequest {
  kind:u64, schema:*const u8, schema_len:u64, type_index:u64,
  variant_index:u64, field_index:u64, input:*const u8, input_len:u64,
  args:*const ValueSpan, arg_count:u64, output:*mut u8, output_capacity:u64,
  scratch:*mut u8, scratch_capacity:u64
}
ValueResult { output_len:u64, scalar:u64, result_kind:u64 }
```

`value_engine::schema_check(schema:*const u8,len:u64)->u64` and
`value_engine::run(request:*const ValueRequest,result:*mut ValueResult)->u64` are
unsafe shared functions. Embedded exports prefix these with `ail_value_`.
`value_engine::validate_argument(request:*const ValueRequest)->u64` validates a
single constructor argument without arena allocation; for this helper only,
type_index carries a type CODE and input/input_len carries its transport. This
permits ordered argument validation before evaluating the next Int expression.
Modes:0validate,1canonicalize,2construct,3test,4get. Status:0success,1VALUE,
2VALUE-LIMIT,3fatal internal ABI/schema/buffer error (VALUE-IO). Scratch8MiB.
Validate/test require no output; test returns Bool code2/scalar0or1/output_len0.
Other modes write canonical transport; scalar also contains signed Int bits,
Float bits or Boolean0/1 for scalar gets. Result kind is the schema type code.

Schema binary v1 header32: magic `AILADT01` at0; u32LE type_count@8,
variant_count@12,field_count@16,names_bytes@20; reservedu64@24=0. Then dense tables:
types8bytes(first_variant:u32,count:u32), variants16bytes(name_offset:u32,
name_len:u32,first_field:u32,count:u32), fields4bytes(type_code:u32), name pool.
Offsets are relative to the name pool. Validate exact lengths, dense ownership,
unique sorted variant names, ranges, limits and all type codes. Type codes:
0Int,1Float,2Bool,3Text,4Bytes,5List<Int>,6List<Float>,7List<Text>,8Json,
9+custom type index. Request variant/field indices are local to their parent.

## Task 1: Compiler, schema checking and VM integration

**Owner:** adt_frontend_design.
**Files:** new src/value_types.rs, src/value_operations.rs; src/ail.rs, lib.rs,
action_order.rs, integer_function.rs, value_function.rs, value_protocol.rs,
value_vm.rs, recovery.rs, ste.rs; new tests/value_types.rs and
tests/value_type_protocol.rs. Mechanical AilBytecodeProgram initializer updates
outside native files are included; coordinate any other needed file.

- [x] Write public source/Core/STE/VM tests for tree construction/test/get and
  functions; observe unsupported-operation/type RED before product changes.
- [x] Implement structural type parsing/instantiation and strict schema metadata.
  Add generics/duplicates/import/nominal/unknown-type tests named in Review Focus.
- [x] Implement source and function operations, strict Core operand ownership,
  semantic reads/writes, bytecode/STE templates and schema-aware protocol checking.
- [x] Wire engine modes into action VM and function borrowed/return paths, preserving
  aliases, validation order and result charging; add VALUE recovery/raise category.
- [x] Run focused tests and compatibility checks; release owned hashes/diff/report
  to /tmp/ail-adt-task-1-* and stop editing for independent review.

## Task 2: Shared transport engine and native execution

**Owner:** adt_native_design.
**Files:** new src/value_transport.rs, value_leaf.rs, value_engine.rs, native_value.rs;
src/native_expr.rs, native_value_function.rs, native_recovery.rs;
runtime/float_runtime.rs, build.rs; new tests/value_types_native.rs.
Frontend owns lib.rs module registration and all non-native compiler glue.

- [x] Write failing engine/unit and actual ELF tests before implementation, covering
  recursive trees, every builtin leaf and schema validation.
- [x] Implement the shared ABI with bounded no_std scanning/canonicalization;
  reuse Float/Bytes engines, independently verify remaining builtin compatibility.
- [x] Implement static schema emission, constructor operand collection, all three
  operation dispatches, custom argument/return support, correct scalar storage,
  tracked scratch cleanup, arena charging and VALUE/status error routing.
- [x] Test no-function ADT actions, nested/mutual calls, aliases, limits, repeated
  recovery and fatal resource exhaustion through actual ELF and public routes.
- [x] Run focused native tests, release /tmp/ail-adt-task-2-* hashes/diff/report,
  and stop editing for independent review.

## Task 3: Independent R019 reference and public differential

**Owner:** adt_oracle.
**Files:** bench/R019-binary-tree/{intent.md,reference.py,cases/*};
scripts/ail_value_type_differential.py. Root owns program.ail.

- [x] Freeze valid representative fixed cases and an independent iterative Python
  tree. Run `python3 scripts/ail_bench.py --rung R019-binary-tree`; expect exit1
  unimplemented. Record hashes/fingerprint/RED before any AIL tree implementation.
- [x] Build 12-route frozen/generated tree oracle, builtin/import/generic and
  malformed/boundary/recovery probes, and sensitive value/validation mutations.
- [x] Run against released implementation; report failures without weakening frozen
  cases. Release /tmp/ail-adt-task-3-* hashes/diff/report for independent review.

## Task 4: Pure AIL tree and usable documentation

**Owner:** root.
**Files:** new examples/ail_std_tree.ail and examples/binary-tree; R019/program.ail;
examples/ste/binary-tree.ail-ste; docs/ail/59-executable-variants.md; applicable
indexes, package guides/support registration and benchmark ledger.

- [x] Wait for R019 frozen RED; implement generic Tree<T> declaration and concrete
  Int tree functions for insert, contains, height and in-order traversal. Preserve
  duplicates using multiplicity and construct all structure with variants.
- [x] Implement caller and benchmark wrapper, verify actual results and source/Core/
  STE/native routes without a host/sorting shortcut.
- [x] Document runnable examples, exact types/transports, limits, errors, imports
  and current remaining gaps; register the support package without altering the
  historical 128-example replay corpus.

## Task 5: Independent review and global verification

**Owner:** root with reciprocal reviews after file ownership is released.

- [x] Independently review frontend, runtime/native, oracle and library/docs;
  resolve real findings with failing/passing evidence and freeze reviewed files.
- [x] Run fmt/diff/build/strict all-target Clippy, all Rust tests, all prior and
  new differential suites, full benchmark, and broad VM/native comparison.
- [x] Promote only new R019 after independent pass; verify unchanged previous
  18 rung entries and old benchmark/oracle inputs. Refresh exact STE/docs links,
  manifest tested inputs and leave full goal active.

## Ledger

Previous turn: progress (completed and verified explicit-error/RLE capability).
Current-state audit rehashed313inputs unchanged. Baseline1741files captured.
Ruling: continuing user authorization and developer autonomy instructions override
repeated skill approval gates. Work stays in the authorized existing worktree
because it contains the full uncommitted language implementation; no git mutation.
Ruling: use parallel compiler/runtime/oracle ownership and reciprocal independent
review. Source metadata, engine ABI and binary schema above are the shared contract.
Ruling: raise custom transport bound to8MiB to accommodate maximum existing Text
and Bytes payloads. Keep borrowed validation distinct from producing a bounded
canonical result, as with existing List input/return semantics.
Spec self-review: coherent source/protocol/schema/native/limits/oracle contracts;
no placeholders or unresolved user choices. AILBC024 verified unused.

| Tasks | Interface check |
| --- | --- |
| 1 <-> 2 | AilValueSchemas + exact dense schema encoding, request/result ABI and status codes. |
| 1 <-> 4 | Source forms, generic import qualification and checked custom function boundaries. |
| 3 <-> 4 | Freeze sorted/present/height contract before AIL tree implementation. |
| 5 <-> 1/2/3/4 | Review released owned files; final checks against frozen hashes. |

R019 freeze:45cases, fingerprintsha256:2366607a2a2761edf25a49af0567ef69,
/tmp/ail-adt-rung-frozen.json; unimplementedRED exit1 before program.ail existed.
The initial metadata mistakenly labeled R019 Tier2; corrected to Tier1 before
any tree program existed and repeated RED/freeze. Original attempt preserved;
all cases/reference unchanged. Root library/app/benchmark now contain six pure
functions and explicit recursive Tree<T>; current compiler RED rejects the first
variant test (/tmp/ail-adt-library-red.log). Existing list-operation spellings
were checked against their references while authoring.
Ruling: add shared validate_argument helper to enforce sorted constructor argument
validation before the next integer expression can fail; the original run ABI is
unchanged. Native owner recorded initial native and engine RED tests.
Ruling: embedded builtin size/item/depth violations use fatal VALUE-LIMIT while
embedded grammar/type failures use catchable VALUE. Standalone builtin operations
and extracted result publication keep their own families. This clarifies the
shared engine status contract and agrees with the independent oracle boundaries.
Ruling: selected builtin canonical-bound failures are engine VALUE-LIMIT errors;
only actual extracted-result arena reservation retains the builtin arena family.
Independent initial GREEN: leaves2160checks/260commands and recovery518/557,
all12routes. Integration RED: imported custom types disappear in merge_ail_import,
and malformed bytecode schema uses source diagnostic AIL-VALUE-001 instead of
AILBC024. Frontend owner is fixing both with regression evidence. No oracle input
or expected behavior was weakened.

Root sample GREEN: documented `[5,2,8,2,7]`/query7 produces sorted[2,2,5,7,8],
presenttrue,height3 in source VM, standalone native ELF and STE VM. Five commands
pass in /tmp/ail-adt-doc-example.json; exact checked-in STE9473bytes. Library and
example guides/support registration complete, package directory/guide count29,
historical counted corpus128 unchanged. Root code review inputs are recorded in
/tmp/ail-adt-library-review-inputs.json. Source/Core/rendered routes and all frozen
cases remain under the independent suite's ongoing run.

Review clarification: builtin precedence is exact for the nine executable leaf
spellings; this milestone adds no broad reservation of descriptive type heads.
Existing source secret handling still applies. Expanded independent limit probes
found borrowed Json canonical expansion was incorrectly rejected; runtime owner
is fixing this without changing the expected behavior. Constructor IntegerExpression
provenance failures must use AIL-VALUE-001 as the Core contract specifies.

Task1 released14files with hashes/diff/report at /tmp/ail-adt-task-1-*.
Fourteen focused tests,41 existing function/STE tests, State Core compatibility,
and strict Clippy pass. Imported types/binder shadow, schema diagnostic, constructor
provenance and legacy State-format findings have RED/GREEN evidence. Declared
Secret<T> retains existing protection diagnostics.
Task2 released after the Json borrowed-bound fix with paired Json/List<Text>
regression:8engine and8native tests pass; strict Clippy and formatting pass.
/tmp/ail-adt-task-2-* contains hashes, baseline diff, snapshots and report.
The existing nonempty process-stdin tests reproduce sandbox EPERM; final global
verification must use the approved unrestricted local-process environment.
Independent root oracle review cross-checked5465small cases without findings.
Independent library review reproduced documentation and exact STE; skew heights62/63
succeed and64fails the existing DEPTH limit on both backends. Reciprocal compiler
and native reviews are now running; no global completion claim yet.

Native independent review found no additional defect after the Json correction:
/tmp/ail-adt-native-independent-review.md, fresh8engine+8native tests and an
independent64-frame invalid-argument/alias probe pass. Frontend review reproduced
two findings: unused descriptive overloaded heads must retain baseline acceptance,
and raw action bytecode must enforce nominal custom slot consistency. Frontend
owner reopened only these fixes; oracle adds independent protocol regressions.
Final verification will use all eight complete independent oracle section runs,
with targeted reruns for corrected protocol/runtime paths. Do not launch a second
full tree suite while its CPU-heavy generated cases are active. This preserves
all12routes and all117frozen/generated cases without duplicating unchanged checks.

Frontend final re-review approved after the nominal consistency, lazy descriptive
ambiguity and builtin self-alias fixes. All14 reviewed owner hashes match;
/tmp/ail-adt-task-1-review-by-native-{report.txt,hashes.json} records independent
RED/GREEN probes. Runtime/native review is also approved. Root independently reviewed the oracle; the oracle owner reviewed the library/docs; frozen tree inputs are unchanged. The37-file
implementation manifest is /tmp/ail-adt-reviewed-inputs.json. Global gates started
under approved host-enabled execution; fmt, diff, build and strict Clippy pass.

First final global run stopped at one synthetic STE fixture:586library tests pass,
all-opcode roundtrip fails because its VALUE_TEST/GET reused custom source storage
as Bool/Int output. The stricter checker correctly rejects it. Frontend owner will
separate the fixture's typed destinations and retain every opcode assertion; this
is a test fixture correction. Original logs are preserved in
/tmp/ail-adt-final-attempt1. All frozen tree45cases x12routes pass (540checks),
and generated72cases continue. Oracle follow-up protocol fixtures pass60checks;
its unused-descriptive fixture labels were corrected to avoid an accidental legacy
duplicate variant, without changing the expected acceptance or frozen rung inputs.

STE fixture repair is test-only, independently inspected by root; its exact
roundtrip test passes. The updated37-file manifest and two documented post-review
test/oracle adjustments are frozen at /tmp/ail-adt-reviewed-inputs.json and
/tmp/ail-adt-post-review-adjustments.json. Final Rust gate now passes1404tests
across45targets,0failed/ignored, in382.66s; fmt/build/strict Clippy also pass.
The21prior differential suites are running while generated tree routes finish.

Independent oracle final GREEN: eight separate complete section invocations,
4270checks/2502commands. All117tree cases run on12routes; generated alone864checks
in1389.83s. Section durations sum2310.34s and overlap, so this is not a single-run
wall time. /tmp/ail-adt-oracle-verification-final.json records every command/log.
All48 oracle-owned hashes, original runner and37 implementation hashes match.
No pending review finding or oracle process remains. Existing-language differential
suites and final benchmark/broad gates continue under root's live runner.

All21previous differential suites pass in the final run. RLE remains5064checks/
5113commands and finishes in766.46s; its frozen fingerprint is unchanged. Together
with the new eight-section suite, all22differential families are green. Root's
full benchmark --update-lock is now running, followed by the broad comparison.

Final benchmark exposed a checker performance regression on the unchanged R026
calculator: identical 1,632-node/2,052-edge graphs checked in1.06–1.13s at the
baseline and6.00–6.13s after schema integration. Builtin field checks repeatedly
resolved whole-program custom schemas. Root stopped only its owned verification
process tree before score promotion; all26 completed gates remain preserved in
/tmp/ail-adt-final-attempt2. The score is still18/18. Frontend will restore exact
builtin fast paths and reuse source schema resolution within compilation, while
retaining unconditional closed-schema checking. Independent review and fresh
verification will use the corrected final implementation.

Performance correction independently approved with no outstanding finding:
/tmp/ail-adt-performance-review-by-native.txt and its reviewed hash manifest.
R026 CLI6.18s→1.08s with identical output; paired Core checks0.99–1.01s against
baseline0.97–0.98s. Independent repeat1.09/1.13s. Production change is confined
to value_function.rs; one new regression guards malformed custom schemas used
only by unused locals in builtin-returning functions. Reviewer additionally
checks valid→invalid→valid mutable Core, saved Core, ownership/provenance and
exact builtin precedence. Fresh66 focused tests plus STE and strict Clippy pass;
independent32tests pass. Updated37-input freeze is recorded. Root restarted all
gates, including all eight new oracle sections, against this final code. Earlier
successful evidence is retained as history rather than substituted for this run.

Final corrected-code run GREEN: fmt/diff/build/strict all-target Clippy and
1,405 Rust tests across45targets (zero failed/ignored;281.85s). All21prior
differential families pass, including RLE5064checks/5113commands. All eight new
variant sections were rerun after the performance correction:4270checks/
2502commands, all117tree cases on12routes. Fixed896.35s and generated1474.88s
ran concurrently with other suites; these are section times, not total wall time.
The authoritative fresh records are /tmp/ail-adt-final-gates.json and
/tmp/ail-adt-final-oracle.json. All37reviewed inputs remain unchanged.

Full benchmark passes19/19 in566.94s; only R019 is added to the lock, with all18
prior rung entries unchanged. Broad comparison passes33agreements/55non-root
entries/0divergences in70.11s. The six added non-root entries are tree library
functions checked by the dedicated suite. README and benchmark journal now
record the actual result. Final audit rechecked426previous benchmark/script inputs,47frozen R019inputs,
independent review manifests and exact9473-byte STE. All288local links across25
changed documents resolve. The386-file final manifest and29verification groups
are recorded in /tmp/ail-adt-final-inputs.json and /tmp/ail-adt-final-summary.json. No dependency
or git-history mutation. This milestone is complete; the broader language goal
remains active.
