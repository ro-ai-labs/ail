# Pure AIL word frequency implementation plan

> For agentic workers: use superpowers:subagent-driven-development for sequential
> implementation, independent task reviews and a fresh whole-milestone review.
> Steps use checkboxes; standing autonomous execution authority remains in force.

Goal: implement reusable exact token frequencies and a bounded raw-stdin tool,
measured against independently frozen R021 cases on VM and native.

Architecture: pure imported AIL functions over Text, List<Text> and Json; the
application alone owns stdin/stdout. No new opcode or host text-analysis service.
Stack: existing dependency-free Rust compiler, AIL source and Python stdlib oracle.
Spec: [word-frequency design](../specs/2026-10-12-word-frequency-design.md).

## Global constraints

- No git mutations, worktrees, dependencies, publication or unrelated refactors.
- Preserve completed CSV milestone and all previous rung inputs/programs/scripts.
- No compiler/runtime changes without reproduced defects and separate review.
- Use exactly the pinned 29 whitespace scalars, case-sensitive exact words,
  numeric count-descending/text-ascending ties and five-entry cutoff from spec.
- Freeze independent new oracle/cases after missing-program RED and task review;
  do not weaken them to match AIL. No --bless-ladder or runtime-budget padding.
- Actual outputs and independent expectations decide passing, not self-agreement.
- Keep existing resource limits and report measured scope honestly. Passing R021
  does not mean Python replacement or completion of the alpha-publication gate.
- Controller owns baseline, integration, final score/JOURNAL/LADDER and checkpoints.
  Use /tmp/ail-words-* for task artifacts, briefs and reports; preserve evidence.

## Review focus

1. All 29 separators, including adjacent mixed whitespace, must discard only empty
   words; NUL, BOM, zero-width space, punctuation and normalization variants stay.
2. Counts compare numerically, and Unicode lexical ties at position five remain
   deterministic under input permutations, including BMP/supplementary codepoints.
3. Slash/tilde/control word keys cannot alter JSON paths; selected request keys
   reject duplicates even with escaped spelling, while unrelated duplicates do not.
4. Long words and useful token/unique counts must work without a per-scalar root
   loop or arbitrary tiny cap; fatal bounds cannot become successful partial counts.
5. Count treats stdin as one raw text, produces one JSON+LF even when empty, and
   emits no result on grant/UTF-8/bound errors. Analyze selects only the result.

## Task 1: Independent oracle and frozen R021 inputs

Files under `/tmp/ail-words-stage/bench/R021-word-frequency/` only:
`intent.md`, `reference.py`, `cases/*.args`.
Consumes: spec contract and documented AIL Json encoding, no AIL implementation.
Produces: standalone Python oracle reading `data.request` from key=value argv,
printing `out.result=<exact compact JSON>`, reviewed cases and frozen fingerprint.

- [x] Implement explicit-whitespace regex tokenization, Counter/sort aggregation,
  lossless selected-key validation and independent transport encoder.
- [x] Pin manual assertions: `" a\tb\r\na "` has words 3, unique 2 and top a2/b1;
  all-whitespace succeeds empty; `"z y x w v u"` yields u,v,w,x,y each 1; `"A a a,"`
  keeps three distinct words. Numeric count 10 must rank before count 9.
- [x] Author named adversarial/seeded cases covering all five review concerns
  and required 32,768-scalar words, 1,024 repeated words and 128 distinct words.
  Include request shape/duplicates/Unicode/number-token distinctions and all
  delimiter/non-delimiter boundaries. Expected results must never come from AIL.
- [x] Validate oracle with hand-calculated and generated tokenizer/count invariants;
  use unchanged `scripts/ail_bench.py` evaluator read-only to record actual
  missing-program RED. Hash exact inputs and write report/evidence under /tmp.
- [x] Independent review; fix material oracle findings before freeze. Root copies
  approved inputs byte-identically into repo; no source implementation precedes it.

## Task 2: Pure library and benchmark application

Create:
- `examples/ail_std_words.ail/{ail-package.md,spec.ail-spec.md,README.md}`
- `examples/word-frequency/{ail-package.md,spec.ail-spec.md,README.md}`
- `bench/R021-word-frequency/program.ail/{ail-package.md,spec.ail-spec.md}`
Consumes: frozen Task1 oracle/inputs and existing compiler/runtime.
Produces public `Words.tokens(Text)->List<Text>`, `Words.summarize(Text)->Json`,
`Words.summarizeRequest(Text)->Json` in `ail.std.words` 0.1.0, and Analyze action
using `data.request:Text` with selected `out.result:Json` in app and benchmark.

- [x] Record direct-library missing-import/function RED for whitespace/Unicode/tie
  scenarios before writing the library, using temporary probe packages.
- [x] Implement exact tokenization with bulk ordinary text operations; preserve
  case/punctuation/non-separators and skip empty words. No per-scalar root loop.
- [x] Implement counting and deterministic top-five selection, safe word keys,
  exact success/failure objects and narrow request recovery per spec.
- [x] Run unchanged R021 evaluator on VM/native and direct token/result checks.
  Include all delimiters, pointer/control words, ties/permutations, numeric counts
  and required useful sizes. Check representative Core/STE execution and pure IR.
- [x] Independently review source and evidence, fix findings, freeze owned hashes.
  Do not edit oracle, compiler, existing rungs or old scripts to get GREEN.

## Task 3: Stdin application, independent acceptance and documentation

Extend app `ail-package.md`, `spec.ail-spec.md`, `README.md`; add app
`input.txt` and `expected.json`; create `scripts/ail_word_frequency_differential.py`,
`examples/ste/word-frequency.ail-ste`, `docs/ail/62-word-frequency.md`.
Narrow navigation/inventory edits: `README.md`, `docs/README.md`,
`docs/ail/README.md`, `docs/ail/25-example-inventory.md`, `examples/README.md`,
`examples/support-packages.md`. Keep existing 128-entry catalog unchanged.
Consumes: frozen library/benchmark/Analyze and Task1 expectations.
Produces Count raw-stdin action and executable independent acceptance/docs.

- [x] Record missing Count action RED, then implement bounded raw stdin → summarize
  → exact compact JSON+LF under --stdio. Preserve Analyze and pure library source.
- [x] Build all 14 source/Core/rendered/direct/STE/saved-bytecode VM/native routes.
  Execute every frozen request and independent direct tokens vectors on them;
  inspect semantic IR for prohibited host/process operations and library effects.
- [x] Check actual Count stdout/status/stderr on Unicode/NUL, all whitespace,
  multiline text, no trailing LF, JSON-looking raw text and useful sizes. Test
  missing stdio, invalid UTF-8, oversized stdin and representative fatal bounds;
  errors must have no result stdout or unexpected input-value leakage.
- [x] Mutate one real ranking/count decision in an isolated copied library and
  prove independent expected output rejects it on VM/native at success status.
- [x] Document commands, token/ranking/request contracts, measured limits and
  exact samples/export. Register the new support package and resulting actual
  package count without changing existing replay catalog/story/request counts.
- [x] Run affected inventory/guide/support/STE tests, actual support-loader closure,
  local links, exact STE regeneration and sample commands. Do not repeat unrelated
  completed compiler suites unless compiler inputs change.
- [x] Independent task review; root obtains fresh whole-milestone review after
  updating current-score/history prose and LADDER/JOURNAL evidence.
- [x] Root revalidates old inputs and records added passing rung through existing
  lock writer only after all required gates/reviews; closes plan, keeps goal active.

## Preflight and execution ledger

- Baseline: /tmp/ail-words-baseline.json contains 1,967 files, byte-identical to
  /tmp/ail-csv-completed-inputs.json. Relevant preimages /tmp/ail-words-baseline.
  Prior CSV milestone is complete at 20/20 (/tmp/ail-csv-final-summary.json);
  no need to rerun its completed acceptance or compiler gates during preparation.
- Ruling: proceed with the written design/plan under standing continuous reversible
  language-development authority. No redundant approval pause, git mutation or
  workspace cleanup. Scope mistakes cost local reviewable rework.
- Design self-review: tokenization, tie ordering, top-five cutoff, raw stdin versus
  JSON request, request ambiguity, fatal versus semantic failures and transport
  encoding are explicit. All public signatures/paths/result keys agree. No
  placeholders, new dependencies or unsupported unbounded-processing claims.

| Tasks/interface | Producer and consumer agreement | Ruling |
| --- | --- | --- |
| 1 ↔ 2 | Frozen data.request oracle emits out.result; Analyze uses those names and imports Words. | Review/freeze oracle before source. |
| 1 ↔ 3 | Acceptance reads frozen cases/reference and compares exact encoded results. | Read-only expectations; direct token vectors independent. |
| 2 ↔ 3 | Same app files; Task2 Analyze and library precede Count extension. | Sequential ownership; Task3 may not edit frozen library/benchmark. |
| 1 internal | Independent stdlib oracle, no AIL source; real missing-program RED. | Scope agrees. |
| 2 internal | Three public pure functions; app/benchmark only Analyze, ordinary functions/values. | Scope agrees; no compiler feature implied. |
| 3 internal | Actual raw stdin behavior plus all artifact routes and scoped docs. | Scope agrees; no unrequested file/JSON-lines interface. |
| 3 ↔ root | Agent owns app/script/docs; root owns scoring/status and final whole review. | Root status edits only after agent freeze/review. |

- Ruling: after Task2 approval freezes library/benchmark, root may run full 21-rung
  scoring once concurrently with Task3, which cannot alter benchmark dependencies.
  Call unchanged bench main without lock flags and retain every result plus exact
  compiler/program/import/oracle dependency hashes. After all reviews/gates, recheck
  identities and publish through unchanged write_lock. Only non-executable
  LADDER/JOURNAL status edits are allowed dependency exceptions; original SCORE
  must match until writer executes. If any semantic dependency changes or coverage
  is incomplete, discard saved scoring and rerun. No --bless-ladder.
- Model/role selection: standard gpt-6.1-sol/high implementers for prose-defined
  integration tasks; gpt-6-astra/high independent task and whole-milestone reviewers.
  Fresh contexts, sequential product ownership, scoped fix loops under SDD.

- Task1 dispatched to /root/words_oracle with /tmp/ail-words-task1-brief.md;
  staging only. Root prepared Task2/3 briefs while independent oracle work runs.

- Task1: complete. Independent spec PASS and quality APPROVE with no findings,
  /tmp/ail-words-task1-review.md (ba739f32...); root verified all 77 input and
  14 evidence hashes and integrated all inputs byte-identically. Frozen 75 cases
  (39 success, 36 REQUEST), fingerprint sha256:468062b540458705e855e78c52757374.
  Manifest SHA25670c0d7fe206264ba3a1251f0398aed85314a4a2cefd255f58cec3da211b1187d.
  Oracle reports 13,587 checks; reviewer additionally executed every actual-loader
  CLI case and nine focused requests. Both confirm genuine missing-program RED.
  Unicode line separators are escaped for the existing argv loader. An initial
  expected-decoder integer-conversion issue was confined to the test harness and
  retained in evidence; oracle numeric tokens never passed through int or Float.
  No AIL source existed at freeze/review. Integration record /tmp/ail-words-integration.json.

- Task2: complete. Independent spec PASS and quality APPROVE; eight new files
  frozen by /tmp/ail-words-task2-hashes.json (afeb2bac...). Root and reviewer
  validated source/evidence/artifact identities. All 75 frozen cases pass on
  VM/native, plus 328 direct token/summary and 16 Core/STE comparisons: 494 total.
  Required useful sizes, pure IR, exact ordinary reporting and fatal JSON depth
  are covered. Original import RED proves missing resolution only; the invalid
  temporary probe verb was corrected later. Literal UTF-8 argv and IR inspection
  corrections were test-harness changes and are retained in the report.
- Task2: minor (deferred): report says every subprocess had a timeout, but three
  historical scratch scripts omit timeout=. Independent review qualifies this
  wording; completed outputs remain valid. Do not repeat this universal claim.
  Task3 permanent acceptance requires finite subprocess deadlines. The final
  whole-milestone review must see this report qualification.

- Task3 Ruling: extend ownership to the library README for the seven mandatory
  learning-guide sections exposed by actual guide-test RED. The spec requires
  package/docs integration. A documentation-only correction preserves API,
  semantics and measured limits; executable library/benchmark inputs stay frozen.
  Retain the preimage and defer this edit until current acceptance's identity
  checks finish. Subsequent guide/link checks cover the changed file; repeating
  behavioral comparisons for prose alone adds no evidence. A mistaken ruling
  costs local documentation/review rework, not a changed benchmark result.

- Task3: complete within implementation/task-review scope. Independent spec PASS
  and quality APPROVE, no findings (/tmp/ail-words-task3-review.md). Root verified
  all 15 owned, 2,039 unchanged, 125 evidence and 5,878 artifact hashes plus exact
  oracle membership/fingerprint and executable/compiler identities. The clean
  standalone acceptance passes 1,896 checks across 1,939 commands and 14 app/14
  token routes. Four affected tests, 157 links, samples/exact STE and real support
  closure probes pass. Initial missing-guide failure and the reviewed docs-only
  correction are retained; no behavioral input changed after acceptance.
  Full scoring, final metadata/whole review and lock closure remain root-owned.

- Whole-milestone review: spec PASS and quality APPROVE; see
  /tmp/ail-words-final-review.md and the frozen validation index. All required
  task, acceptance, documentation, package and scope gates passed. The review
  explicitly includes the Task2 scratch timeout wording qualification; successful
  historical outputs stand, without a claim that every scratch invocation had
  a timeout. Final permanent acceptance subprocesses use finite deadlines.
- Score publication: unchanged ail_bench.write_lock records the saved complete
  21/21 results only after required reviews/gates and fresh dependency checks.
  All 20 prior fingerprints/pass entries remain, and the R021 fingerprint is
  sha256:468062b540458705e855e78c52757374. No --bless-ladder, repeated full
  benchmark, git mutation, dependency change or external publication occurred.
- Word-frequency milestone complete. /tmp/ail-words-final-summary.json,
  /tmp/ail-words-score-update.json and /tmp/ail-words-validation-index.md retain
  the measured scope and evidence. Sources and documentation remain in the
  workspace. The broader Python-replacement and AI-agent language goal stays active.
