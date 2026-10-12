# Trustworthy AIL Edits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Admit AI edits only when they preserve existing checked structure or exactly match a separately supplied checked patch target.

**Architecture:** Canonicalize checked Core projection, then compare complete semantic structure. Share immutable edit admission across generation, repair, and publication. Keep isolated compilation and explicit patching available.

**Tech Stack:** Rust, existing AIL Core/STE/VM/native compiler, Python standard-library differential driver; no new dependencies.

**Spec:** [Trustworthy edits design](../specs/2026-10-11-trustworthy-edits-design.md).

## Global Constraints

- Keep the AIL-Spec → checked Core → STE → verified bytecode → VM/native pipeline.
- No new dependencies. Preserve all previous capability and runtime contracts.
- Do not modify frozen benchmark/oracle inputs or weaken their assertions.
- The capability ladder remains 19/19; this is an authoring correctness milestone.
- Work in the existing checkout; no git mutations, worktrees, resets, or commits.
- Preserve unrelated uncommitted work. Record changed files against the snapshot.
- Standing authorization supersedes additional skill approval pauses. Sequential
  implementation ownership avoids shared-file conflicts; independent read-only
  reviews/oracle preparation may overlap. Evidence lives under `/tmp/ail-edit-*`.

## Review Focus

1. Legacy edges without step_index: storage permutations must preserve scalar results.
2. Duplicate labels/ordinals and malformed saved baselines must fail closed.
3. Imported nodes and metadata must use the same immutable source snapshot.
4. Retries and saved-Spec builds must not bypass admission or mutate the baseline.
5. Intended changes must remain useful, and explicit compiler passes must retain a separate, accurately bounded proof.

## Task 1: Canonical checked Core projection

**Files:** `src/ail.rs`, narrow append-order change in `src/core_model.rs`;
focused `src/core_order.rs` if useful; new `tests/core_order.rs`.
**Interfaces:** retain `ail_document_from_core(&AilCore) -> AilDocument` and
`check_ail_core_diagnostics(&AilCore) -> Vec<AilDiagnostic>`.

- [x] Write tests reversing edge storage for legacy Set/Compute, mixed operations,
  duplicate calls, returns/traces, and executable functions; assert identical
  document, STE, bytecode, VM results. Add duplicate-ordinal rejection.
- [x] Run `cargo test --test core_order`; capture meaningful RED against baseline.
- [x] Canonicalize one Core view by ordinal at projection entry, avoiding a clone
  when already ordered. Feed that view to all generic/specialized decoders.
  Reject duplicate edge ordinals, including provenance, with `AIL-CORE-ORDER-001`.
  Graph append after removals allocates above the maximum retained ordinal;
  add a remove/add patch regression to prevent newly rejected valid edits.
- [x] Run the new test target and relevant existing order/function/round-trip tests.
  Record test output and changed-file hashes for independent task review.

## Task 2: Complete structural comparison and safe addition policy

**Files:** new `src/edit_semantics.rs`, new `tests/edit_semantics.rs`;
`src/ail.rs` replaces old comparator body with module/reexports; `src/main.rs`
fixes only equivalence dispatch/flags and saved-baseline validation.
**Interfaces:** preserve
`compare_ail_core_semantics(&AilCore, &AilCore) -> AilSemanticDiff` and
`render_ail_semantic_diff(&AilSemanticDiff) -> String`. Extend public diff fields
with `changed: Vec<String>`, `invalid: Vec<String>`,
`preservation_violations: Vec<String>`, and
`lowering_validation: AilSemanticLoweringValidation`. The public enum reports
`NotRequired`, `BaselineUnavailable`, `CandidateFailed`,
`ExistingRecordsPreserved`, or `ExistingRecordsChanged`; admission still depends
on the complete comparison facts and violations. Existing predicates retain names with
the stronger contract in the spec. No consumer constructs the struct without
Default; verify before making this source-compatible extension.

- [x] Write executable-change, order/multiplicity, case/metadata and invalid-input
  tests, plus ID/provenance/permutation invariance and independent new action/type
  acceptance. Assert appended old-action steps fail preservation. Capture RED.
- [x] Implement checked exact labels/attributes/metadata and occurrence-preserving
  owner/relation order. Reject ambiguity; enumerate allowed new declaration roots
  and schema ownership closure. Preserve source-facing reports deterministically.
  Supplement graph facts with canonical projected Action reads/writes, Tool
  reads/writes/calls, CompilerPass reads/writes, and Failure handling; compare
  exact effective payload vectors so executable provenance fallback is preserved.
  Permit only ordinary Action/Function/Type roots; reject cross-runtime-name and
  type-head collisions. Test that PermissionDenied Failure addition is rejected.
  For additions when baseline lowers, compare existing lowered action/function,
  failure, and concrete schema entries and require candidate lowering success.
- [x] Make `ail-equivalence` strict by default, add `--preserve-baseline`, implement
  saved-Core candidate dispatch, validate saved baselines, reject selector conflicts.
- [x] Run `cargo test --test edit_semantics` and existing equivalence/roundtrip cases;
  capture tests and changed-file hashes for task review. Do not weaken old oracles.

## Task 3: Immutable library authoring admission

**Files:** new `src/edit_admission.rs`, new `tests/edit_admission.rs`, `src/ail.rs`;
narrow shared compiled-record validation extraction in `src/edit_semantics.rs`.
**Interfaces:** exported `AilEditContext` with private state; constructors
`preserve_package(&AilPackage) -> Result<Self, String>`,
`expected_patch(&AilPackage, &str) -> Result<Self, String>`,
`greenfield(&AilPackage) -> Result<Self, String>` (empty source only),
`preserve_core(&AilCore) -> Result<Self, String>`, and
`expected_core_patch(&AilCore, &str) -> Result<Self, String>`.
Expose `check_candidate(&self, &AilCore) -> Vec<AilDiagnostic>` and
`render_admission(&self, &AilCore) -> String` for deterministic evidence.
Expose `render_prompt_context(&self) -> String` so CLI requirements generation
can use the same selected baseline or target as Spec generation and repair.
Add `_with_edit_context` variants of existing draft/requirements/recorded/max-token
and repair functions, taking `context: &AilEditContext` as the final argument.
Export `check_ail_draft_spec_with_edit_context(package, spec_text: String,
requirements_text: Option<&str>, context: &AilEditContext) -> AilDraftResult`.

- [x] Add direct library admission tests covering destructive stub, unchanged
  formatting, safe additions, exact intended arithmetic/trace changes, unrelated
  extra edits, stale/wrong-package patch, invalid baseline, imports and empty source.
  Capture RED before implementation.
- [x] Implement private immutable contexts with checked construction and exact
  expected-target matching. Reuse Task 2 comparator and existing patch engine.
  Construction validates comparator preconditions as well as the ordinary Core
  checker (self-comparison invalid facts), so ambiguous baselines/targets fail
  before network. Greenfield retains exact package metadata in its context.
- [x] Preserve existing APIs as baseline-preserving wrappers; prepare context
  before network, share context through retries, enrich prompts with the checked
  source/target, and append `AIL-EDIT-*` diagnostics after isolated checking.
  Include expected Core attributes as well as source: Core→Spec rendering alone
  can alter retained writes.provenance text by inserting expression parentheses.
- [x] Run `cargo test --test edit_admission` and focused existing draft/repair tests.
  Local socket tests require approved host execution. Record task-review evidence.

## Task 4: CLI acceptance and publication

**Files:** `src/main.rs`, new `tests/edit_authoring.rs`, limited compatibility
fixtures in `tests/ail_toolchain.rs` whose intended change must now be explicit.
**Interfaces:** consume Task 3 context variants; add `--expected-patch <path>` to
draft/spec/story/build; allow `--baseline` on standalone Core build only when
meaningful. Reject ignored or conflicting flags. Pass a checked admission report
through build artifact publication and story acceptance.
Add `--greenfield` on draft/spec/story/package-build only for empty local source;
reject it with expected-patch/baseline or a nonempty source. Default empty source
fails with explicit guidance; malformed nonempty source never becomes greenfield.

- [x] Add mock-model tests for destructive direct draft, requirements generation,
  repair, story and saved-Spec build; assert failure before AcceptSpec/native or
  accepted artifacts and unchanged source. Test stale patch before network.
  Test explicit empty-source greenfield success and all invalid flag/baseline
  combinations failing before model invocation.
- [x] Construct one context before LLM calls from the already loaded package;
  preserve it across helpers/retries. Gate saved-Spec and optional baseline Core
  builds at the same pre-execution boundary.
- [x] Publish deterministic admission evidence bound to the manifest; story
  executable acceptance uses it. Detect source artifact snapshot mismatch.
  Describe explicit compiler pass transformations separately from admission.
- [x] Migrate deliberate trace/permission/native saved-Spec test changes to explicit
  expected patches or standalone source fixtures appropriate to each test's purpose.
  Do not weaken assertions or convert rejected scenarios into unconditional success.
- [x] Run new authoring tests plus existing draft/build/story/manifest/pass tests;
  verify no accepted output on rejection and legitimate changed VM/native behavior.
  Record evidence for task review.

## Task 5: Independent oracle, documentation, and final verification

**Files:** new `scripts/ail_edit_differential.py`, new
`docs/ail/60-trustworthy-edits.md`; README, docs/ail/05-agent-protocol.md,
and narrowly relevant current-status references.

- [x] Independently generate mutations for arithmetic, literals, functions,
  variants, call order/multiplicity, permissions, traces, types and metadata.
  Verify actual changed results for executable cases through STE/VM/native;
  assert strict equality and preservation reject, and exact approved edits pass.
  Include neutral ID/provenance/storage/formatting transformations and additions.
- [x] Independently review task diffs and whole milestone; fix material findings.
- [x] Document CLI/API policy, intentional edits, diagnostics, proof limitations,
  and standalone compilation. Keep ladder score 19/19 and larger goal active.
- [x] Run full Rust tests, all existing differential families plus new oracle,
  full 19-rung benchmark, and broad VM/native suite on frozen final inputs.
- [x] Rehash prior benchmark/oracle inputs, review changed-file scope, validate
  local documentation links, and record final commands/counts/artifact locations.

## Execution ledger

| Preflight scope | Producer/consumer or internal check | Ruling |
| --- | --- | --- |
| Task 1 | Checked ordinal view; tests exercise legacy and explicit decoder order | Consistent; unique ordinals and gapped append allocation required |
| Task 2 | Complete comparison; tests cover case, attributes, occurrences, payloads, ambiguity and roots | Consistent after audit refinements below |
| Task 3 | Private contexts and public wrappers; tests cover construction and admission | Validate comparability before network, not just isolated checking |
| Task 4 | CLI contexts to publication; tests cover all authoring entrances | Explicit greenfield and expected patch flags cannot be ignored |
| Task 5 | Independent behavior oracle and frozen full gates | Keep original score and inputs; no new rung claimed |
| Tasks 1 / 2 | Shared ail.rs and ordinal contract | Sequential ownership; comparator consumes canonical projection |
| Tasks 2 / 3 | AilSemanticDiff API and predicates | Task 3 uses exact target equality or permitted-addition predicate |
| Tasks 2 / 4 | Shared main.rs CLI argument handling | Sequential ownership; baseline build extension preserves strict equivalence flags |
| Tasks 3 / 4 | AilEditContext and context-aware drafting APIs | One captured context survives retries and reaches publication |
| Tasks 4 / 5 | CLI flags and admission reports | Freeze implementation before running complete final oracle |

- Design reviewed against the two read-only audits. All five review-focus cases
  are assigned tests above. Tasks 1→2 share `src/ail.rs` and ordered-Core contract;
  2→3 share comparison API; 3→4 share context API; 4→5 share CLI/report contract.
  Implementation is sequential. Each task's tests and file ownership match its
  interfaces; root owns this plan, spec, review scheduling, and final verification.
- Ruling: use snapshot-based review instead of git commits/worktree automation —
  the managed checkout has extensive existing work and read-only git metadata;
  all changes remain locally reviewable against `/tmp/ail-edit-baseline`.
- Ruling: execute without another design approval pause — the user authorized
  continuous language development, and higher-priority instructions require
  completing authorized reversible work. A mistaken design costs local rework.
- Ruling: require all edge ordinals to be unique, including provenance — legacy
  payload selection can depend on provenance order, and one graph-wide invariant
  avoids opaque-ID ties or a second context-sensitive ordering algorithm. Gaps
  remain legal. Custom graphs with tied ordinals must assign explicit order.
- Ruling: preserve effective executable payload recovered from provenance before
  excluding evidence — an independently checked reproduction changed a legacy
  scalar result from 2 to 5 by changing a Provenance node name alone. Attribute
  names are insufficient evidence that a value is nonsemantic.
- Task 1 implementation: 109 focused tests passed after recorded RED; independent
  review found ordinal exhaustion could panic through the checked in-memory patch
  API. Fix round 1 adds nonwrapping allocation and a checked rejection regression,
  plus a permanent executable-provenance attachment-order regression. Review
  artifacts: `/tmp/ail-edit-task-1-report.md` and `...-review.md`.
- Ruling: restrict implicit new roots and check existing lowered records when
  available — adding PermissionDenied changes old requirement failure selection,
  and a new Function can collide with an existing Action runtime name. Unsupported
  profiles retain an explicitly narrower structural claim. Other declaration
  families remain editable through a separately supplied expected patch.
- Ruling: preserve conservative writes.provenance equality even when typed
  expression parentheses alone change and bytecode matches — policy checkers
  still consume this attribute. Document the exact structural limitation; use
  checked Core attributes in target prompts. Do not weaken frozen roundtrip tests.

- Task 1: complete. Final review passed spec and code quality with no unresolved
  findings; root independently reran 15 new regressions, all passed. Full focused
  evidence totals 112 passing tests. See `/tmp/ail-edit-task-1-review.md`.
- Independent preimplementation oracle: 17 executable mutations (12 seeded
  arithmetic, plus literal/function/variant/order/multiplicity) all produce the
  Python-owned expected differing results, but the frozen original comparator
  accepts all 17 as equivalent. Evidence `/tmp/ail-edit-independent-red.json`;
  cases `/tmp/ail-edit-oracle-cases.json` SHA256
  `b67b17d9b5d0989e2fa4be3344f4ebfe9e40a8b4765045c3af0e513b9ccf7703`.
- Task 2 initial independent verification: all 17 frozen behavior mutations fail
  strict and directional comparison through package and saved-Core inputs
  (68 negative comparisons). The same original/candidate cases pass 204 expected
  execution checks across source, saved Core, STE, VM, and native with both the
  old compiler and the initial Task 2 compiler. Reports:
  `/tmp/ail-edit-independent-task2.json`, `/tmp/ail-edit-runtime-baseline.json`,
  and `/tmp/ail-edit-runtime-task2.json`. Root also ran 37 Task 1/2 regressions.
- Task 2: fix round 1 in progress. Independent review found incomplete nominal
  collision checks: omitted builtin type heads and existing Thing declarations
  were admissible as new Type roots. The checker recognizes both namespaces;
  fix the admission vocabulary without changing checker language behavior.
  Review: `/tmp/ail-edit-task-2-review.md`.
- Task 2: fix round 1/5 addressed the collision finding; independent scoped
  re-review passes spec and quality (`/tmp/ail-edit-task-2-rereview-1.md`).
  Focused tests passed 39; root replayed all eight reported collision cases.
  Root's required fmt/clippy gates found one reexport formatting issue and four
  comparator lint failures. Fix round 2 is limited to mechanical corrections;
  behavioral and quality gates must pass before task completion.

- Task 2: fix round 2/5 addressed the formatting and lint gates with mechanical
  rewrites only. Final scoped review passes spec and quality with no unresolved
  findings (`/tmp/ail-edit-task-2-rereview-2.md`). Final new comparator suite: 25
  passing tests; fmt and all-targets clippy clean. File hashes match the report.
- Task 2: complete. Comparison, safe-addition policy, strict/directional CLI,
  and saved-Core validation are reviewed. Authoring admission remains Task 3/4
  work; comparison success alone is not yet a guarded drafting workflow.
- Task 3: dispatched for immutable library contexts and preserve-by-default
  draft/repair wrappers. Requirements: `/tmp/ail-edit-task-3-brief.md`.
- Ruling: expose a read-only prompt-context renderer — requirements generation
  must see the selected patch target too, or its old-trace obligations can
  conflict with an explicitly authorized trace change. Reuse one context
  renderer across requirements, Spec generation, and repair. Cost is a small
  additive public API; it supplies guidance, never authorization.
- Task 2 package compatibility survey: all 69 checker-clean packages among 71
  example/benchmark packages compare equal to themselves under the stricter
  rules. No checked package was rejected as ambiguous. Evidence:
  `/tmp/ail-edit-package-compatibility.json`.
- Ruling: preserve standalone requirements-only API behavior — prose generation
  and repair do not accept executable code and can work with empty source.
  Context-aware requirements variants are useful for guarded build/story flows,
  but existing prose-only wrappers must not gain a checked-baseline prerequisite.
  Spec-producing wrappers remain preserve-by-default. This keeps initial
  requirements authoring available without adding an unrelated CLI flag.
- Ruling: keep `bench/JOURNAL.md` frozen along with the existing benchmark tree —
  the plan's bare JOURNAL reference resolves only to that file. This milestone
  adds no capability rung; record its progress in the new authoring document and
  README instead. This preserves the full 479-file benchmark/script snapshot.
- Task 3 initial independent verification: 216 public-API assertions across
  twelve explicit arithmetic targets passed. Original destructive direct draft
  now rejects while isolated checking remains usable and source stays unchanged.
  Reports: `/tmp/ail-edit-task3-independent-context.json` and
  `/tmp/ail-edit-guard-green.json`. Two native tests that failed with OS26 in a
  parallel broad run both pass individually with one test thread; full serial
  verification remains pending after Task 4's three intentional-edit migrations.
- Task 3: fix round 1 in progress. Independent review plus root VM replay
  confirmed greenfield import projection can hide a local PermissionDenied
  addition that changes an imported action's selected failure and trace.
  Preserve protected imported compiled records against the full candidate when
  lowering is available, while retaining local greenfield declarations and the
  narrower documented nonlowerable-profile claim. Reuse the existing Task 2
  compiled-record validator through a narrow shared extraction. Evidence:
  `/tmp/ail-edit-task-3-review.md`, `/tmp/ail-edit-greenfield-import-red.json`.

- Task 3: fix round 1/5 addressed the imported-record P1. Scoped re-review
  passes spec and quality with no unresolved findings
  (`/tmp/ail-edit-task-3-rereview-1.md`). All 16 admission and 25 comparator
  tests pass; fmt and all-targets clippy pass. Root rebuilt the exact frozen
  inputs and independently confirmed harmless greenfield admission and rejection
  of the imported failure/trace change (`/tmp/ail-edit-greenfield-import-green.json`).
- Task 3: complete. Immutable context constructors, public drafting/requirements
  variants, synchronous candidate checks, and deterministic evidence are reviewed.
  CLI publication integration remains Task 4.
- Task 4: dispatched for CLI context propagation, pre-acceptance gates and
  publication proof. Root independently reproduced four model-time snapshot
  mutations (local/imported Spec and manifest); the frozen Task 3 CLI publishes
  despite each mutation. Evidence `/tmp/ail-edit-snapshot-red.json`; queued
  green replay will verify rejection with no accepted artifacts.
- Ruling: define publication protection at the admission/snapshot rejection
  boundary — every such check must complete before the first accepted write.
  A second snapshot rejection after artifact writes would violate that guarantee;
  use one final publication barrier. Atomic filesystem isolation and arbitrary
  I/O-failure rollback remain outside this authoring milestone. If broader
  transactional publication is needed, it requires separate implementation.
- Task 4 initial freeze: 88 focused/covering tests plus fmt/clippy pass. Root
  independently verified destructive saved-Spec rejection, preservation of prior
  accepted files/native results through two rejected edits, all four model-time
  snapshot mutations rejecting without publication, and nine exact-target model
  outcomes across draft/Spec/build. Native results are 9 → rejected9 → approved5
  → rejected5. Reports `/tmp/ail-edit-{saved-spec-green,publication-proof,
  snapshot-green,model-target-proof}.json`; independent review is pending.
- Task 4: fix round 1 in progress. Review reproduced a positional-package plus
  `--core-file` selector bypass: the package was silently ignored and changed
  native output published without admission. Reject that conflicting form while
  retaining genuinely pathless standalone Core builds and optional baselines.
  Evidence `/tmp/ail-edit-task4-selector-review.json`; scoped review
  `/tmp/ail-edit-task-4-review.md` found no other blocker.
- Task 4: fix round 1/5 closed the selector P1; scoped re-review passes spec
  and quality with no unresolved findings (`/tmp/ail-edit-task-4-rereview-1.md`).
  Fix evidence: 13 authoring tests and six standalone Core tests pass, fmt/clippy
  clean. Six old Core build invocations only removed their ignored positional
  package argument; all assertions remain. Root independently passed conflicting,
  pathless standalone, baseline-protected and exact-target Core build outcomes
  (`/tmp/ail-edit-selector-green.json`).
- Task 4: complete. CLI admission, source consistency, publication proof and
  explicit selector policy are reviewed. Task5 permanent oracle/docs and the
  full final milestone gates remain pending.
- Task 5 implementation frozen: new self-contained oracle passed 280 recorded
  VM/native checks across 731 commands, 60 CLI comparisons, 15 exact targets,
  four metadata rejections, graph/provenance/invalid cases and five publication
  checks. All 104 local doc links and five executable documentation commands
  pass. Evidence `/tmp/ail-edit-task5-final-evidence/report.json`; independent
  task review and full final gates remain pending. Cumulative input audit
  confirms 479 old benchmark/script files plus both Cargo files unchanged.
- Task 5 delegated implementation review: PASS for spec and quality, no material
  findings (`/tmp/ail-edit-task-5-review.md`). Whole-milestone review and root
  final frozen gates now begin; Task5 overall verification remains in progress.

- Final integration review: requirements PASS, quality APPROVE, no material
  findings (`/tmp/ail-edit-final-review.md`). Its declined broad-gate completion
  remains root-owned; general equivalence, atomic publication, and full Python
  replacement are explicitly outside this milestone's claims.
- Final full Rust gate: 432 toolchain tests passed, two older tests failed.
  One intentionally edits a saved Spec but lacks explicit edit authority, so
  the new guard correctly rejects before its native-lowering assertion. The
  other receives the exact missing-capability diagnostic during baseline
  validation on stderr, while its assertion reads stdout only. No executable
  implementation defect is established. Initial gate evidence is preserved in
  `/tmp/ail-edit-final-fix/`; the final run will include --no-fail-fast.
- Ruling: migrate these two test setups/assertions to the established admission
  contract, retaining their original native rejection and capability facts.
  The separately authored native test source will be its own baseline; the
  package test will inspect the baseline diagnostic. Cost if wrong is loss of
  a legacy output-channel expectation, not weaker capability validation.
  Correct the optional v0 package-field wording in the same reviewed fix wave.

- Final fix wave: both exact failures reproduced RED then GREEN, 22 nearby
  covering tests passed, fmt and whitespace clean. Independent scoped re-review
  PASS/APPROVE addresses both test migrations and package-field wording with no
  new breakage (`/tmp/ail-edit-final-rereview.md`). Only the declared test/doc
  hashes changed; production executable remains SHA256
  `22bf54b8ea97847d52e6802d3f167162747137d1d4c92b8651cca303c9ad295d`.
  Revised freeze is explicit; original freeze/failure evidence is retained.

- Final revised gates: PASS on the independently reviewed frozen implementation.
  Full serial Rust: 1474 tests across 49 targets,
  0 failures and 0 ignored. All 22 differential
  families pass, plus all8 ADT sections (4270 checks,
  2502 commands). New edit oracle:280 runtime checks,731 commands,
  60 CLI comparisons,15 exact targets; final report /tmp/ail-edit-oracle-478dyk4w/report.json.
  fmt, diff, build and all-targets clippy pass. Benchmark remains19/19 without
  lock update. Broad result: vm/native differential: 33 agree, 55 not natively compilable, 0 diverged.
- Final freeze audit: all1819 reviewed repository inputs retained,
  all481 frozen inputs retained (479 old benchmark/script
  files plus Cargo.toml/Cargo.lock). The final17 product-file scope is reviewed;
  this plan is administrative. All104 local documentation links and5 example
  commands checked. Evidence: /tmp/ail-edit-final-summary.json,
  /tmp/ail-edit-final-gates.json, /tmp/ail-edit-final-oracle.json and associated
  logs. Final binary SHA256 22bf54b8ea97847d52e6802d3f167162747137d1d4c92b8651cca303c9ad295d.
- Task5: complete (task review PASS, whole-milestone review PASS/APPROVE,
  scoped final test/doc fix PASS/APPROVE, no open material findings). The final
  broad gates resolve review items explicitly left to root. General behavioral
  equivalence, arbitrary filesystem write rollback and full Python replacement
  remain outside this bounded milestone; the documented structural guarantee
  and pre-publication rejection boundary are the accepted contract.
- Trustworthy-edit milestone: complete. Preserve this checkout and all evidence
  in place under the standing no-git-mutation constraint; no commit, merge,
  publication or cleanup is requested. The overall language-development goal
  remains active. Next milestone: pure AIL CSV parsing and aggregation, with
  independently reviewed oracle preparation already isolated under /tmp.
