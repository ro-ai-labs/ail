# Text transformations implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development for disjoint
> frontend/runtime, native and independent oracle tasks. Root integrates and
> verifies; final reviews use workers other than each scope's implementer.

**Goal:** Split/join-list/replace/trim through source/Core/STE/VM/native and file reports.
**Architecture:** Extend existing typed TextOperation and reuse List<Text> codec,
owned result arenas and scoped function protocol. Direct native helpers share the
same validation order, limits and immutable results as the VM.
**Tech Stack:** Dependency-free Rust 2024, direct x86-64 ELF, Python oracle.
**Spec:** ../specs/2026-10-11-text-transformations-design.md

## Global constraints

Text/list raw and produced byte bounds 1,048,576; list items65,536; independent
shared Text and List/Json arenas67,108,864 bytes. Preserve old operation semantics,
argument validation, local ownership, scalar UTF-8 and zero operand-data leakage.
No deps/interpreter/compiler subprocess/runtime files. Linear split/replace
search under repetitive needles. New algorithms must be reviewed and measured.

Ruling: user continuation explicitly authorizes progress on the original goal.
Continue the existing shared worktree without resetting/committing/cleaning its
uncommitted work or writing read-only .git. Developer autonomy takes precedence
over repeated skill approval gates. Spec/plan/review ledger is reviewable here.
Existing agents are reused when fresh spawning is unavailable. Baseline snapshots
are /tmp/ail-text-transform-baseline. Previous turn is verified progress from
current Boolean plan and1,026-test final log, not an assumption of goal completion.

## Locked interfaces and ownership

| Producer/consumer | Interface | Owner |
| --- | --- | --- |
| Text/frontend/root | four AilTextOperation variants, fields/opcodes/source per spec | frontend worker text_operations.rs |
| Codec/Text VM | parse_text_items and make_text_list signatures per spec | frontend worker narrow list_operations.rs helpers |
| Text VM/root | execute(i,state,&mut text,&mut list) | frontend defines; root ail/value_vm call sites |
| Native/root | exact four opcodes, roles and helpers, existing field layout | native worker native_expr/native_text/native_value_function plus new native_text_transform.rs as needed |
| Root/function Core | operand_schema and ValueOperation::Text reconstruction | root value_protocol/value_function |
| Oracle/all | independent semantic spec and public CLI/API only | oracle tests/text_transformations.rs + scripts/ail_text_transform_differential.py |

## Tasks

- [x] 1. Frontend/runtime worker: meaningful RED then enum/source/Core/check/
  protocol/VM/namespace/access implementation in text_operations.rs and narrow
  List<Text> codec helpers in list_operations.rs. Strengthen touched Text Core
  decoding against wrong metadata, forged global fields, duplicate/foreign edges
  and extra parents. Preserve old source/Core syntax and old tests. Own focused
  unit tests; never edit root/native/oracle files. Release once checks pass.
- [x] 2. Native worker: meaningful raw opcode RED then native fields/action and
  function dispatch and helpers, optionally new native_text_transform.rs. Reuse
  UTF-8/list parsing/owned allocation; bounded linear search for split/replace,
  full decoded List<Text> join, explicit29-scalar trim table. Test error priority,
  empty-needle scalar boundaries, count/byte/arena limits, aliases, recursive
  frame/fuel registers and absence of Float runtime for Text-only programs.
- [x] 3. Independent oracle worker: tests/text_transformations.rs and
  scripts/ail_text_transform_differential.py. Observe missing-opcode/source RED,
  write generated Python split/join/replace/strip expectations plus boundary,
  malformed/protocol/Core/secret/import/functions/file tests. Report production
  findings to owner; no production edits, no full-suite/spawn.
- [x] 4. Root: ail.rs catalogs/native-entry/opcode/profiles/trace/VM routing and
  malformed descriptive guards; value_protocol/value_function/value_vm typed
  local operands/decode/execute; ste.rs exact templates and generic fixtures.
  Coordinate function/native field discovery and scratch/arena contracts.
- [x] 5. Root: docs/ail/49-text-transformations.md, source/schema/STE/functions/
  README indices, examples/tabular-report and exported STE. Demonstrate all four
  operations, numeric/Boolean predicates, reusable local functions, bound file
  read/write and actual report bytes across all six artifact routes/backends.
- [x] 6. Independent frontend/root and native reviews; fix findings with focused
  RED→GREEN and scoped re-review. Run fullserial suite, strictClippy/fmt/diff,
  independent new/prior oracles and frozen benchmark. Record evidence and leave
  original full Python-replacement goal active.

## Review focus

All operand types/static secret reads even on failing/skipped branches; aliases
source=destination and needle/replacement/delimiter=destination; nested function
scratch and immutable outputs. Empty separator failure priority; empty needle
Unicode scalar insertion; zero-width/NUL/combining data retention in trim; complete
JoinList source checking before delimiter and no canonical-input expansion limit.
Split canonical escaping/count bounds and exact shared arenas, including zero
length Text charging1 vs empty List output charges. No normalization or scanning
of inserted replacement. Search must not repeatedly rebuild a prefix table.

## Evidence ledger

Design reviewed against current Text/list/function/native code and Python/Unicode
primary references. Chosen direct operations expose semantics to Core and avoid
opaque calls. No user approval is missing under the continuing authorized goal.

### Implementation progress

- Frontend meaningful RED: `/tmp/ail-text-transform-frontend-red.log` recorded
  unrecognized source forms and ignored extra Core metadata before implementation.
- Native meaningful RED: a raw `TEXT_TRIM` program was rejected as unsupported
  before native dispatch/helpers were added.
- Independent integration RED: `/tmp/ail-text-transform-red.log` reached newly
  emitted instructions but failed at the VM's old unreachable opcode branch.
- Root wiring is implemented across opcode/profile/trace routing, local typed
  schemas and reconstruction, Core edge persistence, STE and descriptive-body
  guards. `cargo check --all-targets` passes in
  `/tmp/ail-text-transform-root-check2.log`.
- Initial independent integration target: 11 tests pass in
  `/tmp/ail-text-transform-integration-progress.log`; final checks remain pending.
- Tabular Report and its exported STE are written. Source, saved Core and rendered
  source, each directly and through STE, produced the expected JSON file in both
  VM/native (12 executions under `/tmp/ail-text-transform-example`). Expected
  values: names Apple/Unicode λ, 2 accepted, 2 rejected, 5 units and 850 cents.
- Documentation chapter49 and language/schema/STE/function indices are updated;
  relative links in the changed indices, chapter and example resolve.

The plan's evidence ledger remains the persistent review record because this
shared worktree is not committed and .git is read-only. Reviews compare the
milestone snapshots in `/tmp/ail-text-transform-baseline` rather than an unrelated
historical commit range. No task is marked complete before independent review.

### Review and fixes

1. The new List<Text> reader initially propagated the existing JSON parser's
   invalid-escape character. Frontend self-review and independent frontend review
   both found it. `/tmp/ail-text-transform-codec-diagnostic-red.log` captures RED;
   the first full-suite attempt also caught that snapshot (397 passed, 1 failed).
   The new helper now preserves LIST/LIST-LIMIT families with a content-free
   message. Its focused codec test passes independently; legacy parsers are unchanged.
2. Independent frontend/root review found terminal punctuation bypassed the new
   descriptive-body guard (`the system joins list.` and bare transformation verbs
   ending in a period). Reproduction `/tmp/ail-text-guard-review.rs` accepted all
   16 malformed combinations. The reviewer is assigned the narrow guard fix and
   RED/GREEN regression; the independent oracle worker will review that diff.

Task1: complete after independent frontend/runtime review. 20 focused Text tests,
19 list tests and strict library Clippy pass. The native worker independently
checked Core metadata/ownership/static secrets and atomic result/arena failures;
no remaining frontend/runtime finding. Root guard review is tracked separately.

Task3: complete. 12 independent integration tests pass, focused Clippy is clean,
and `/tmp/ail-text-transform-oracle.log` records 1,000 seeded datasets with four
operations/eight aliases each on both backends; 18 malformed/priority cases;
32 invalid UTF-8 probes; 27 exact/over-bound and pathological-search checks;
exact shared arena/function-return accounting; 240 source transformations through
12 public routes; and 54 independently calculated TSV report datasets through
those routes plus 7 malformed numeric inputs preserving prior reports.

Task5: complete. Docs and example were independently reviewed against the exact
contract; the oracle exercises the actual report, including Unicode/NUL inputs.
No example or documentation issue remains.

Task2: complete after independent native review. 23 focused native Text tests
pass (12 new); independent re-run of the 12 new tests plus 70 additional
Python-derived VM/native probes passes. Review verified balanced stack/preserved
RBX/RBP/R12–R15, full input validation, immutable copies before scratch release,
Unicode boundaries and exact allocation. Maximum match offsets stay below the
prefix table; the largest prefix table ends at the mapping boundary. Evidence:
`/tmp/ail-text-native-review-focused.log`, `/tmp/ail-text-native-review.log`.

Task4: complete after independent frontend/root review and scoped review of its
fix. The guard now calls existing `trim_sentence`; new
`tests/text_transform_guards.rs` reproduces the prior punctuation bypass and
checks unrelated identifiers remain accepted. RED1fail/1pass and GREEN2/2 are in
`/tmp/ail-text-transform-guard-review-{red,green}.log`. A different reviewer reran
all 12 transformation tests plus both guard tests, strict targeted Clippy and
formatting; all pass. No outstanding findings. Final integration/oracle review:
`/tmp/ail-text-transform-oracle-review.txt`.

Prior independent regressions all pass in
`/tmp/ail-text-transform-regression-{text,list,json,literal,file,structured_json,json_mutation,value_function,float,float_list,json_inspection,bool}.log`.
Frozen benchmark remains16/16; broad differential remains33agreements,
30 unsupported native cases, zero divergences. Neither benchmark files nor
SCORE.lock were changed. Full-project post-review results appear below.

### Final verification

Task6: complete. On the reviewed final code, the serial all-target Rust suite
passes **1,064 tests across 23 targets**, exit0:
`/tmp/ail-text-transform-post-review-tests.log`. Strict all-target Clippy,
`cargo fmt --all -- --check` and `git diff --check` pass in the adjacent
`post-review-{clippy,fmt,diff-check}.log` files. The reviewed source/test/script
hash manifest still matches after verification
(`/tmp/ail-text-transform-reviewed-check.log`). Build and example STE export
checks pass. Independent frontend, native and integration/oracle reviews have
no outstanding findings; both discovered defects have focused RED/GREEN and
independent verification.

The implementation and test deliverables for this increment are complete.
The original goal remains active: these operations and the frozen16/16 score
do not establish general Python replacement, full library coverage or full
ASD-STE100 conformity. Existing worktree changes remain preserved; no commit,
merge, reset, cleanup, benchmark mutation or publication was performed.
