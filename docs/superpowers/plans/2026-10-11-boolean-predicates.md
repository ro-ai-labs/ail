# Boolean predicate implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development with
> disjoint AST, frontend/Core and native work; root owns integration and final
> verification. Independent reviews must use different workers.

**Goal:** Reusable short-circuit Boolean predicates through source/Core/STE/binaries.
**Architecture:** One typed bounded AST, explicit Core tree, EVAL_BOOL/BRANCH_BOOL
transport and lazy evaluation on both backends. Existing numeric ASTs are reused.
**Tech stack:** Dependency-free Rust 2024, direct x86-64 ELF, Python test oracle.
**Spec:** ../specs/2026-10-11-boolean-predicates-design.md

## Global constraints

65,536 expression bytes; 1,024 nodes; depth128 across the whole AST including
numeric subtrees. Text runtime operands at most1MiB. Exact Bool transport.
Short-circuit runtime reads; all operands statically checked. No arena charge.
Preserve current source syntax, numeric semantics, ownership, profiles, field
layout, isolated functions, shared depth/fuel and immutable data behavior.
No dependencies, external interpreter/runtime files or compiler subprocesses.

Ruling: continue authorized work in the current shared worktree, preserve its
uncommitted state and read-only .git, and do not commit/reset/clean. The continuing
user goal and developer autonomy authorize implementation without repeating
skill approval pauses. Existing agents are reused because fresh spawning is
exhausted. Repository spec/plan/review records make decisions inspectable.

## Interface and ownership preflight

| Producer / consumer | Interface | Ownership check |
| --- | --- | --- |
| AST / all | BoolExpr/TextOperand variants/methods and typed postfix as spec | AST worker owns bool_expression.rs; root only registers module |
| Core / root | bool_operations APIs parallel float_operations | Core worker owns bool_operations.rs/control_flow.rs/float_operations.rs and their tests |
| Root / Core | scoped Int decoding/building available from existing modules | Root exposes value_function::scoped_expression if needed; Core validates incoming tree ownership and genuine fields |
| Root / native | EVAL_BOOL and BRANCH_BOOL exact operands | Root opcode/protocol/STE; native worker owns emission and dependency discovery |
| Oracle / implementation | public semantic spec | Oracle runs after AST slot frees; no production edits |

All tasks consume the same AST names, operand kinds, trace strings and limits.
Root owns shared action/function registration, so workers must not edit ail.rs,
lib.rs, value_function.rs, value_protocol.rs, value_vm.rs or integer_function.rs.
Root must classify Boolean controls as value functions even with Int-only signatures.

## Tasks

- [x] 1. AST worker: create src/bool_expression.rs and tests; consume existing
  Int/Float ASTs. Observe parser/evaluator RED, implement source/postfix/render,
  typed fields, validation, mapping, lazy evaluation and combined limits. Test
  quotes/keywords, source nesting, numeric subtrees, short-circuit vs reached
  errors, conflicting field types, full Unicode comparisons and all operators.
- [x] 2. Frontend/Core worker: create src/bool_operations.rs; modify
  src/control_flow.rs and focused parent support src/float_operations.rs. Observe
  RED; implement action operations and IfBoolean/WhileBoolean, explicit scoped
  graph trees, raw aggregate bounds and semantic accesses. Coordinate shared
  helper requirements with root. Tests cover malformed graphs, types, ownership,
  static skipped accesses, profile and source round trips. No root-owned edits.
- [x] 3. Native worker: create src/native_bool.rs; modify src/native_expr.rs,
  src/native_value_function.rs, src/native_float.rs dependency discovery and
  src/native_text.rs helper visibility only if required. Observe RED then compile
  lazy AST jumps, scalar result storage, field discovery and Text comparisons.
  Test skipped/malformed operands, left/right priority, no arena consumption,
  aliases, all types, function body-only dependencies and no unnecessary Float.
- [x] 4. Root: src/lib.rs/src/ail.rs, value_protocol.rs/value_function.rs/
  value_vm.rs/integer_function.rs and ste.rs. Register modules/Core/catalogs,
  action ordering/import/render/check/secret/compile/run paths, typed local
  operands/control/functions, exact opcodes/profiles/labels, native entry routing,
  legacy descriptive guards and STE. Observe initial AST/Core/oracle RED evidence
  before hooks. Expose scoped Int helper narrowly if Core worker requests it.
- [x] 5. Independent oracle worker once a slot is free: tests/boolean_predicates.rs
  and scripts/ail_bool_differential.py, no production. Exercise source/saved/
  rendered/STE/VM/native, functions/imports/secrets, all quoted/Unicode/numeric
  comparisons, short-circuit and limits; independent Python expected values.
- [x] 6. Root: docs/ail/48-boolean-predicates.md, indices/schema/reference updates,
  examples/boolean-filter and exported STE. Replace text-search classification in
  examples/json-inspection with direct Boolean text predicates and re-export STE.
  Verify real file datasets and all six public routes.
- [x] 7. Independent frontend/root and AST/native reviews; fix findings, run full
  serial Rust suite, strict lint/fmt, prior oracles and frozen 16/16 benchmark.
  Record evidence and keep the original goal active.

## Review focus

1. Dormant right side must remain statically checked but have no runtime errors
   or allocation. Add tests for missing globals/uninitialized locals, invalid
   native UTF-8, division-by-zero and skipped secret access (AST/Core/native/oracle).
2. Embedded numeric trees share every predicate budget; Core raw oversized
   literals cannot normalize below bounds before counting (AST/Core tests).
3. Quoted source/postfix Text can contain `and`, `or`, parentheses, field names,
   escapes and NUL without becoming syntax (AST/oracle roundtrip tests).
4. Function entry validates even unused Bool/Text args before predicate execution,
   and body-only Float predicates still link runtime (root/native/oracle tests).
5. Negation/ordering and mixed nested controls preserve continue/break/fuel;
   repeated Bool computation never consumes result arenas (Core/native/integration).

## Evidence ledger

Previous goal turn is verified progress based on current completed JSON inspection
plan and 981-test log. Full Python replacement is still unproved. Spec reviewed
for ambiguous grammar/transport/ownership; tasks share fixed interfaces above.

### Implementation and focused evidence

- AST: eleven tests pass after seven initial parser/evaluator failures and separate
  borrowed-frame/numeric-nesting failures. Bool/Text operands borrow function
  values; reached numeric comparisons use compatible `eval_borrowed` helpers.
- Frontend/Core: seven Boolean tests, two mixed-control tests, five prior Float
  Core tests and seventeen prior control tests pass. Module/model, catalogs,
  ordered action operations, import/secret paths, profiles, function classification,
  exact protocol, native routing and both STE templates are wired.
- Native: ten dedicated Boolean tests and all 146 native emitter tests pass.
  Independent native review approved 208 additional checks, including Unicode/NUL,
  invalid UTF-8, helper-call depth128 and dormant function-only Float dependencies.
- Both examples pass source/saved Core/rendered source × direct/STE × VM/native.
  BooleanFilter produces names `["ship parser","Unicode λ fix"]`, accepted 2,
  rejected 3; JsonInspection retains kind object and 8 scalar leaves.
  Evidence: `/tmp/ail-bool-examples-six-routes.log`.
- Independent oracle first reproduced punctuation/escaped-string corruption in
  saved Core. TextOperand.value now uses strict JSON quoting in text transport
  while the graph retains decoded strings; its six-route regression is green.
- Independent reviews found escaped raw-text aggregate limits and two source
  rendering boundaries (deep Float negation and a near-limit Bool field), plus
  legacy `boolean.count` prefix capture. Three focused regressions observed RED
  then GREEN in `tests/boolean_boundaries.rs`. Saved-Core raw string spelling
  sizes are retained internally until shared predicate preflight; no metadata
  is added to semantic graphs. Renderers avoid redundant unary/atom parentheses.
- Thirteen prior regression runners passed, including JSON inspection, recursive
  JSON/mutation, value functions, floating/list/text/file/literal operations,
  frozen 16/16 ladder and broad 33 agree / 30 unsupported / 0 divergence. Logs:
  `/tmp/ail-bool-regression-*.log`. Final full suite/review/oracle evidence follows.

### Independent oracle and review follow-up

- Independent integration target: 13/13 tests pass, including dotted fields named
  after every new keyword, quoted opaque values, imports, secrets, profiles,
  malformed protocol and isolated functions.
- Independent Python Boolean oracle:350 datasets × 84 predicates = 29,400 expected
  results per backend; 94 generated source predicates over all public routes;
  48 independent JSON-record filter datasets; reached/skipped invalid UTF-8 and
  typed errors; 128 predicates plus 128 Bool returns after both 64 MiB arenas fill.
  Final rerun passes: `/tmp/ail-bool-final-oracle-bool.log`.
- Frontend/root review approved after 3 focused regressions and 20 independent
  public-API probes, including exact 65,536 raw mixed Text/Int transport bytes
  and one byte over, across action and function computations/controls.
- AST review also caught Float literal-negation folding and complex near-limit
  source spacing. Explicit Negate literals retain one necessary pair of
  parentheses; other unary chains do not add redundant groups. Pretty Boolean
  source uses lexical whitespace compaction only when it exceeds the byte bound,
  preserving all quoted data and identifier separators. Expanded regressions
  observed RED then GREEN. Final independent AST review approved: 5,000 near-limit roundtrips
  (2,715 exercised compaction), 5,000 typed AST property cases, 954 scalar
  comparisons, all boundary/negation repros and all 11 AST unit tests pass.
  Evidence: `/tmp/ail-bool-ast-compaction-review.log`.
- Initial complete suite passed 1,026 tests across 21 targets; final rerun follows
  the last rendering changes. Final strict all-target Clippy, fmt and diff checks
  pass; final Float and JSON-inspection oracles also pass. Both example reports
  pass all 12 source/Core/rendered×direct/STE×VM/native executions again in
  `/tmp/ail-bool-final-examples-six-routes.log`.

### Final verification and goal status

- Final `cargo test --all-targets -- --test-threads=1`: **1,026 passed**, zero
  failures, 21 targets, exit 0. Evidence: `/tmp/ail-bool-final-tests.log`.
- Final `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all --check`,
  `git diff --check`, build and Python oracle syntax checks pass. Evidence:
  `/tmp/ail-bool-final-{clippy,fmt,diff-check,build}.log`.
- Final Boolean, Float and JSON-inspection oracle reruns all exit 0:
  `/tmp/ail-bool-final-oracle-status.log` and individual matching logs.
  The other ten prior runners passed earlier with no affected changes afterward.
- Both examples and exported STE match expected reports on all public routes;
  native, frontend/root and AST independent reviews are approved with no pending
  findings. No dependency, interpreter, compiler subprocess or runtime file was
  added; no commit/reset/clean was performed.

This milestone is verified progress. The active objective remains an agent-usable
Python replacement through the checked STE pipeline; full replacement has not
been demonstrated and the original goal is not complete.
