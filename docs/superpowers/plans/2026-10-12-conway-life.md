# Conway's Life Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a reusable pure Life API, independently measured R025 20×20×50,
and a bounded JSON stdin tool through the public VM/native routes.

**Architecture:** A flat integer list represents a finite rectangular grid.
Shared pure helpers validate grids and compute simultaneous generations;
step, evolve, population and the request adapter expose useful composition.
An independent dense oracle is checked against a sparse coordinate model.

**Tech Stack:** Existing AIL source, checked Core, STE, VM/native compiler;
Python standard-library oracle/harness; existing bounded process runner.

**Spec:** [Conway's Life design](../specs/2026-10-12-conway-life-design.md).

## Global Constraints

- Package `ail.std.life` 0.1.0, Application profile, alias `Life`, directory
  `examples/ail_std_life.ail`; standalone app `examples/conway-life`.
- Signatures: `step(width:Int,height:Int,cells:List<Int>)->List<Int>`,
  `evolve(width:Int,height:Int,cells:List<Int>,generations:Int)->List<Int>`,
  `population(cells:List<Int>)->Int`, `runRequest(text:Text)->Json`.
- Grid is row-major, positive width/height, exact product length, only 0/1.
  Dead exterior, eight neighbors excluding self, simultaneous B3/S23 updates.
  Avoid dimension-product overflow; normalize -0. Returned lists are immutable.
- Direct domain errors use VALUE; request strings are REQUEST, GRID, GENERATIONS,
  in that priority. Exact response shapes/order and narrow catches are in spec.
- Analyze accepts data.request:Text, selects only out.result:Bytes, trace Simulated.
  Run consumes one raw request to EOF and writes one compact response plus LF.
  All fallible work precedes output; fatal failures propagate without private leaks.
- Ordinary pure AIL only: no host Life, compiler/runtime/old script/test/rung edits,
  dependencies, budget changes, instruction padding, cycle shortcut or arbitrary cap.
- No git mutation/commit/worktree/cleanup/installation/network/publication.
  Preserve prior work; root owns design, plan, ledger, metadata and score.
- Independently freeze the oracle and observe actual missing-program RED before
  AIL source. Never derive expected behavior from AIL. Root integrates exact bytes.
- Finite leaf and outer deadlines, descendant cleanup, immutable evidence directories,
  exact commands/streams/statuses and complete tested dependency/artifact hashes.
  Preserve failed attempts; do not rerun unchanged passing gates.

## Review Focus

1. Small/narrow grids must not wrap, count self or duplicate neighbors; non-square
   dimensions and edge patterns must distinguish row-major indexing mistakes.
2. Simultaneous evolution must retain old grids and compose across repeated calls;
   zero generations must still validate and odd/even oscillators must be correct.
3. Maximum-i64 dimensions must fail as domain errors without overflow; selected
   integer grammar differs from opaque numbers in ignored fields.
4. Complete syntax/Unicode and selected duplicate-member validation precedes domain
   checks; GRID wins over negative generations and errors return no partial cells.
5. Useful-size evolution, raw stdin and fatal bounds must use actual measured paths;
   mutation detection must reject successful wrong output, not compilation errors.

## Task 1: Independent oracle and frozen rung

Stage only under `/tmp/ail-life-stage/bench/R025-conway-life`:
`intent.md`, `reference.py`, `cases/*.args`. No program.ail or repository products.
Consumes the spec and unchanged `scripts/ail_bench.py` observer. Produces standalone
`step(width,height,cells)`, `evolve(width,height,cells,generations)`,
`population(cells)`, `evaluate_request(text)` and `encode(result)` functions with
VALUE domain exceptions. Python direct functions reject bool-as-int. CLI emits
exactly `out.result=hex:<lowercase UTF-8 hex of encoded result>` plus LF for one
data.request argument. Model result dictionaries preserve specified field order.

- [x] Implement a dense independent model, duplicate-preserving request parser and
  complete Unicode validation; handle strict selected i64 and opaque ignored number
  tokens without Python's long-integer conversion limit. No other rung oracle import.
- [x] Check at least 20 hand-calculated direct/request assertions, including every
  spec anchor, single-row/column patterns, zero steps, nonbit/mismatch/oversized
  dimensions, population empty and error priority.
- [x] Cross-check 250 deterministic seeded grids of dimensions 1..12 and 0..15 steps
  against a separate set/neighbor-count model. Check every generation, population,
  composition and retained input. Record seed and counts.
- [x] Create named frozen cases for every Review Focus class, every selected field's
  missing/duplicate/escaped duplicate/type errors, grid domain causes, request
  syntax/Unicode failures and priority combinations. Include every required-size
  case in the spec, signed-i64 boundaries, negative-zero, ignored duplicate names,
  valid 5000-digit ignored numbers and fake output-line/Unicode separator text.
- [x] Verify every actual oracle CLI result through unchanged load_cases/state_lines;
  each .args request occupies one physical loader-safe line. Escape separators/NUL
  where required by the loader, keeping all semantic content.
- [x] Capture actual evaluate_rung missing-program RED before AIL source exists.
  Freeze exact staged membership, input hashes, fingerprint, commands and evidence.
- [x] Independent spec/quality review; root verifies baseline and integrates approved
  exact oracle bytes. Resolve findings through the same owner.

## Task 2: Pure API and Analyze application

Create exactly eight files:
`examples/ail_std_life.ail/{ail-package.md,spec.ail-spec.md,README.md}`,
`examples/conway-life/{ail-package.md,spec.ail-spec.md,README.md}`,
`bench/R025-conway-life/program.ail/{ail-package.md,spec.ail-spec.md}`.
Consumes Task1 oracle/cases and existing compiler. Produces all four public
signatures, meaningful pure helpers and matching app/bench Analyze bodies.

- [x] Close the missed syntactically valid pre-source direct RED as a documented
  process deviation. Preserve the initial attempts, corrected full-report GREEN
  and import-only missing-library control; see Task 2 completion below.
- [x] Implement complete grid/bit validation without product overflow; shared finite
  neighbor/generation logic; step, evolve and population; narrow request extraction/
  validation catches and exact result construction. No duplicated simulation algorithm.
- [x] Front-load complete VM/native report comparisons for all four APIs, composition,
  immutable input, zero-step validation and caught VALUE. Then run every frozen
  request on source VM/native and every public API on direct VM/native callers.
- [x] Measure all required useful-size cases early before broad route acceptance;
  preserve actual runtime, compiler identity, status and complete output. If a
  requirement fails, investigate before expanding tests or changing constraints.
- [x] Check direct malformed typed transport separately from well-typed domain
  errors; prove no overflow with max-i64 dimensions and canonical -0 returns.
  Test all Review Focus classes and repeated independent/composed calls.
- [x] Inspect generated IR for ordinary list/branch/function operations and no I/O
  in the library. Execute representative saved-bytecode VM and compiled-bytecode
  native callers and successful wrong-output mutation controls.
- [x] Reproduce realistic fatal fuel/list allocation limits with smaller success
  controls and private markers. Do not catch them or mislabel a reached limit.
- [x] Package exact owned/source/dependency/evidence/artifact hashes and full diff;
  independent task review. Freeze accepted library/benchmark for Task3.

## Task 3: Stdin tool, reproducible acceptance and documentation

Modify the three Task2 app files; create `examples/conway-life/{input.json,expected.json}`,
`examples/ste/conway-life.ail-ste`, `scripts/ail_life_differential.py` and
`docs/ail/65-conway-life.md`. Update `README.md`, `docs/README.md`,
`docs/ail/README.md`, `docs/ail/25-example-inventory.md`, `examples/README.md`,
`examples/support-packages.md`. Those are exactly 14 owned files. Consume frozen
library/benchmark/oracle, retaining the accepted Analyze body and existing runner.

- [ ] Demonstrate actual missing Run RED; implement raw stdin Run with existing
  grants, one final write and exact response plus LF. Add a runnable sample and guide.
- [ ] Implement independent acceptance with source/Core/rendered-Spec VM/native,
  their three directly executed STE forms, and source saved-bytecode VM/native:
  seven representations times two backends, 14 app and 14 direct API routes.
  The source STE route executes the exact exported STE bytes. Compare every frozen
  request Analyze and raw stdin on all 14 app routes and direct API vectors on all
  14 direct routes, with exact oracle bytes/canonical results.
- [ ] Before the broad matrix, front-load exact complete reports for all four APIs
  plus composition, zero-step invalid grid and VALUE recovery on source VM/native.
  Cover stdin success/domain errors, malformed UTF-8/missing grants, complete
  input/response boundaries and real reached fuel/list-allocation failures with
  positive controls. Preserve private-marker and no-result-stdout checks.
- [ ] Permanently prove two successful wrong-output mutants: sequential updates and
  wrapped edges, using hand/oracle-distinguishing cases. Inspect actual IR and
  guard full oracle membership, imports, compiler/runtime/build source and old runner.
- [ ] Reserve artifact directories before any child or writes; reject occupied and
  concurrent ownership without changing their bytes. Test fresh/empty acceptance,
  occupied/concurrent rejection and retained failure evidence. Every leaf and outer
  driver is bounded with cleanup; save complete streams and identities.
- [ ] Export final example STE and verify checked example bytecode preservation,
  inventory/learning-guide tests, production corpus loader support-package admission
  and negative missing-package control, applicable links and Python syntax. Use
  existing test binaries where compiler/tests are unchanged; record exact commands.
- [ ] Package final 14-file diff and current hashes plus immutable historical
  validation records. Independent task review; resolve findings through owner.

## Root completion gates

- [ ] Audit task ownership, all accepted evidence and exact unchanged baseline.
- [ ] Run one unchanged full 24-rung benchmark under a bounded outer process with
  all dependencies frozen. Preserve results and status; do not record score yet.
- [ ] Update README score and LADDER/JOURNAL with measured scope/limits; check links,
  whitespace, Python syntax and all old fingerprints/pass entries.
- [ ] Fresh whole-milestone review over the complete diff and frozen evidence.
- [ ] Record approved saved 24/24 results with unchanged write_lock, close this plan,
  save a completed snapshot and continue the broader active language goal.

## Preflight and execution ledger

Ruling: Continue design and SDD under standing continuous reversible-work authority;
use exact baseline/preimages and local evidence instead of git/worktree mutations
or cleanup — preserves all prior work — if scope is wrong, new local artifacts
and documentation can be revised without altering prior milestones.

Ruling: Finite dead-edge rectangular grids, with typed row-major0/1 lists, are the
R025 contract — the ladder names Life 20×20×50 without boundary/transport rules —
if another boundary convention is later wanted, it requires an explicit new API.

| Tasks | Produced/consumed boundary | Preflight finding |
| --- | --- | --- |
| 1 → 2 | Frozen four-function model and request cases → pure implementation | Same types, priority and exact response shape; source follows reviewed oracle. |
| 1 → 3 | Frozen oracle/cases → full route and stdin expectations | Task3 cannot change oracle or expected behavior. |
| 2 → 3 | Four public APIs and Analyze; three shared app files | Sequential ownership; Task3 adds Run and preserves Analyze/library/benchmark. |
| 3 → root | README and docs → metadata/final review | Root score edit occurs after Task3 and is included in whole-milestone diff. |
| 1 | Stage/model/CLI validation and missing-program RED | No repository product writes; actual observer verifies loader safety. |
| 2 | Eight files, API implementation and direct/full-report tests | Static feasibility remains unproven until required-size executions pass. |
| 3 | Fourteen files, permanent acceptance and integration | Existing code reused read-only; source identities frozen before long runs. |

Baseline: `/tmp/ail-life-baseline.json`, 2333 files, SHA-256
`1be85c5d65b892f099fdf86c9c7dbe309df966b37ad3a06fe4264ef832d3d6bb`.
Root checkpoint: `/tmp/ail-life-root-checkpoint.json`. R024 remains complete 23/23;
its gates and evidence must not be replayed or modified.


### Task 1 completion

- Task 1 complete after independent review and one scoped correction. Current
  frozen rung has 174 files / 172 cases: 37 success, 111 REQUEST, 22 GRID and
  2 GENERATIONS. Fingerprint: sha256:ce9f086748bbdd3abd28961f1e4f53ae.
- Independent evidence: 59 hand/direct assertions, 250 seeded grids and 1871
  generation/evolve/population/composition checks, plus retention/repeated-call/
  immutability checks. Original 172 CLI leaves passed; two amended cases were
  subsequently rechecked, carrying 170 unchanged cases by exact byte identity.
  Both stages observed real missing-program RED before AIL source existed.
- Initial review failed B1 because two positive cases exceeded the retained
  JSON depth bound. Ruling: preserve original products/evidence in the v1 stage,
  create v2 with exactly 63 ignored containers plus root (depth64), and change
  only those two cases and intent. Oversized originals remain for later fatal
  checks; no runtime/observer/budget change. If the positive-boundary assumption
  is wrong, required VM/native execution will expose it before broad acceptance.
- Ruling: the fix uses a separate report instead of appending to the original
  hashed report, preserving original manifest identity. The review package links
  both reports; the cost is following composed provenance across two records.
- Root verified all original maps unchanged, the three-file amendment, exact
  corrected depth and command/output evidence. Scoped re-review closed B1 with
  composed Spec PASS / Quality APPROVE. See /tmp/ail-life-task1-review.md,
  /tmp/ail-life-task1-fix1-review.md and /tmp/ail-life-task1-approved.json.
- Loader limitation: .args processing strips trailing whitespace; neither the
  original CLI nor 59 hand assertions proved literal trailing JSON whitespace.
  Task2 direct calls and Task3 raw stdin explicitly cover it. Valid whitespace-json
  succeeded; empty/whitespace-only requests were REQUEST.
- Python executable/version and test/driver/repository dependencies are captured;
  individual transitive Python/system libraries were not separately hashed.
  No AIL source/VM/native result is claimed by Task1. No passing matrix replay.


### Task 2 completion with explicit process exceptions

- Current eight-file product accepted: independent functional-contract PASS and
  Quality APPROVE. The review retains literal Spec FAIL for the two historical
  process requirements below; this is not an unqualified compliance claim.
  Review: /tmp/ail-life-task2-review.md. Root disposition:
  /tmp/ail-life-task2-approved.json.
- Exception 1: valid pre-source direct RED was missed because unresolved imports
  masked the invalid caller header. Initial records remain unchanged. Corrected
  complete-report GREEN and an import-only missing-library negative control prove
  present behavior, not the required historical chronology. The independently
  frozen Task 1 oracle and missing-program RED still predate implementation.
- Exception 2: the owner edited the live harness path while long drivers ran.
  Some leaf hashes describe later disk contents, and eight historical revisions
  have no retained source snapshot. Exact historical driver provenance remains
  incomplete. Root accepts continuation under standing reversible-work authority
  because independent saved-output/source review establishes the current behavior;
  it does not reconstruct the missing bytes. No passing matrix was replayed and
  no historical record was rewritten. Task 3 must execute immutable, attempt-
  specific driver copies and hash the same bytes at every leaf.
- The independent reviewer checked all 344 request/backend reports, 182 direct
  results, 18 complete front reports, both saved-bytecode executions and four
  successful wrong-output mutants against the frozen oracle. All 590 inspected
  green-era leaf records contain the current eight product hashes. All nine
  required sizes pass VM/native; maximum useful-size VM time was 21.885405s.
- Concrete limits remain unchanged: 1x1x10000 reaches instruction fuel exhaustion;
  32x32x70 reaches LIST-LIMIT (native 9.115534s; VM 128.367999s, finite 300s leaf
  limit). Smaller controls pass. Actual public failures have no result stdout or
  private marker. A separate synthetic SET_FIELD native default-report fixture
  exposed its seeded operand; that privacy assertion remains a recorded failure,
  distinct from subsequent --stdio transport probes. No Life code uses SET_FIELD.
- Root audit: exactly 8 additions, 2509 unchanged baseline files, all 174 frozen
  oracle files and compiler unchanged. Manifest:
  /tmp/ail-life-task2-hashes.json. Root command census: 626 records/1878 streams;
  exact request union is 18 early size executions plus 326 remaining executions.
  Accepted library/benchmark files are frozen for Task 3. Score remains 23/23.


### User-authorized Git checkpoints (2026-10-12)

The user requested periodic commits and pushes to preserve progress. This new
instruction overrides the earlier no-git/no-push process restrictions for root
checkpointing. Root will commit and push the current working branch at meaningful
verified boundaries; workers continue to avoid git mutations. No merge, force
push, branch deletion or workspace cleanup is authorized by this checkpoint policy.

First checkpoint scope: the STE pipeline and accumulated language/runtime/docs/
examples/tests through the completed 23/23 ladder, plus the reviewed Life oracle
and pure API. Life Task 3 and the full 24-rung milestone remain unfinished. Its
missing Run RED has been captured with no Task 3 product edits. The two Task 2
process exceptions remain recorded above. Existing benchmark and review evidence
is retained locally; temporary evidence and build products are outside Git.
