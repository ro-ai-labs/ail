# Pure AIL CSV parsing and aggregation implementation plan

> For agentic workers: use superpowers:subagent-driven-development, with
> sequential AIL implementation and independent task review. Preparation may
> occur under /tmp while the preceding milestone's frozen gates run.

Goal: add reusable CSV parsing and a file/JSON-lines aggregation application,
measured against independent frozen R022 cases on VM and native.

Architecture: pure imported AIL functions; structured Json results; existing
text, control, error and file/stream mechanisms. No new host parser/opcode.
Stack: existing dependency-free Rust toolchain, AIL source and Python stdlib
oracle. Spec: [CSV aggregation design](../specs/2026-10-12-csv-aggregation-design.md).

## Global constraints

- No git mutations, worktrees, dependencies, publication or existing rung edits.
- Current trustworthy-edit repository snapshot remains frozen until gates end;
  only /tmp staging is authorized before then. Preserve all prior work. After
  Task1's independent review and freeze, new pure AIL source may also be staged
  under /tmp using the frozen compiler; integration still waits for prior gates.
- Exact CSV dialect, result shape/key order, integer grammar and error priority
  from spec. No per-character root loop or runtime-budget padding.
- Freeze independently authored new rung inputs and prove unimplemented RED
  before AIL implementation. No --bless-ladder or oracle weakening after GREEN.
- Actual execution results and rejected output preservation determine success.
- Full objective remains active; a passing R022 does not mean Python replacement.

## Review focus

1. Quote pairs and CRLF crossing the chosen search window still parse exactly;
   quoted CR/LF, Unicode/NUL and empty/blank rows preserve their distinctions.
2. Validation priorities cannot expose a partial total or accept malformed tail
   input after earlier valid rows; all widths precede numeric processing.
3. Header/category keys containing slash/tilde/controls cannot change JSON paths,
   and duplicate selected request members reject rather than choosing a value.
4. i64 minimum, -0, invalid spellings and per-group overflow are independent of
   a later mathematically in-range total; exact result key order is stable.
5. Invalid file input preserves a preexisting report, and one malformed stream
   request does not terminate subsequent valid requests or emit input secrets.

## Task 1: Independent contract and R022 inputs

Files: new bench/R022-csv-aggregate/{intent.md,reference.py,cases/*.args}; during
the current freeze, prepare exact future contents under
`/tmp/ail-csv-stage/bench/R022-csv-aggregate/` only.
Consumes: spec contract, no AIL implementation. Produces: a deterministic Python
oracle reading key=value argv and selecting `out.result`, frozen inputs/hashes,
and an unimplemented-rung RED using the existing bench evaluator read-only.

- [x] Author Python csv.reader-based independent rows plus a declarative strict
  grammar check; preserve selected duplicate request keys. Own encoder matches
  established Json control escaping. Do not derive expected values from AIL.
- [x] Author named adversarial and seeded cases for all five review concerns,
  accepted long fields and representative row counts, request/CSV/header/width/
  amount/overflow priority. Use explicit seeds and independent expectations.
- [x] Test oracle against hand-calculated small outputs and Python csv.reader
  for generated valid dialect input. Show genuine missing-program RED through
  `scripts/ail_bench.py`'s unchanged evaluator. Write hashes and report in /tmp.
- [x] Independent review of oracle/contract/cases. Freeze approved inputs;
  copy byte-identically into repo only after prior milestone gates finish.

## Task 2: Pure library and benchmark application

Files: examples/ail_std_csv.ail/{ail-package.md,spec.ail-spec.md,README.md},
examples/csv-aggregate/{ail-package.md,spec.ail-spec.md,README.md},
bench/R022-csv-aggregate/program.ail/{ail-package.md,spec.ail-spec.md}.
If the preceding milestone is still verifying, use exactly these relative paths
under `/tmp/ail-csv-stage/`; do not copy into the repository yet.
Consumes: frozen R022 contract and existing runtime. Produces public functions
parse(Text)->Json, summarize(Text)->Json, summarizeRequest(Text)->Json;
Analyze action with data.request:Text and out.result:Json.

- [x] Record direct-library RED on quoted/Unicode/malformed input before source.
- [x] Implement scanners and parser with bulk/bounded-window text operations;
  imported helper/type names stay qualified, no external process or host parser.
- [x] Implement checked aggregation and structured request/errors exactly.
- [x] Run frozen R022 and direct parser checks on VM/native, including bounds,
  quote/window crossings, order and extreme integers. Never mutate oracle inputs.
- [x] Independent task review; resolve material findings and freeze source.

## Task 3: Real I/O, independent differential acceptance and documentation

Files: extend examples/csv-aggregate actions and samples; new
scripts/ail_csv_differential.py; generated examples/ste/csv-aggregate.ail-ste;
new docs/ail/61-csv-aggregation.md; README/manual/example inventory/support lists
only as necessary for the new library and app. Controller owns score/ledger.

- [x] Add Stream and WriteReport with the exact spec behavior; write pending
  report only after successful analysis, retain original nonblank JSON lines.
- [x] Independent script drives frozen expectations across source, saved Core,
  rendered Spec, STE, bytecode and VM/native; includes direct parser vectors,
  useful large inputs, actual files, streams, wrong grants and input rejection.
- [x] Deliberately mutate one AIL operation in an isolated copied package and
  show oracle failure. Do not mutate the working library or old scripts.
- [x] Document commands, contracts, API, useful sizes and runtime limitations;
  update required package guide/inventory links and exact checked-in STE.
- [x] Independent task and final whole-milestone review. If compiler inputs are
  unchanged, run new acceptance, old full benchmark and affected docs/package
  tests, links and exact STE check; do not repeat unrelated compiler suites.
- [x] Compare old rung hashes, record added passing rung with the existing lock writer only
  after all gates pass, append actual evidence to journal/plan, keep goal active.

## Preparation ledger

- Ruling: preparation is isolated under /tmp while the preceding milestone is
  verified. Its frozen repository/hash authority is unchanged. Cost if the
  current milestone needs another fix is only later integration work.
- Ruling: staged pure AIL implementation may follow the independently reviewed
  and frozen oracle before old gates finish. The frozen compiler can execute
  standalone packages without repository writes. Acceptance and repository
  integration still wait for both reviews and completed old gates. If the old
  compiler changes, rerun the new targeted checks against that final binary.
- Ruling: proceed with reversible planning and independent fixture preparation
  under the standing autonomous language-development objective; no additional
  approval pause. Scope mistakes cost local rework and remain reviewable.
- Design self-review: field syntax and empty-record distinction are explicit;
  header/width/numeric priorities agree with output rules; direct parser versus
  structured wrapper failures are separate; effect claims exclude write rollback.
  Each review concern has a Task1 case and Task2/3 executable acceptance owner.
- Task1 staging only; no new production/library/benchmark repo files yet.

- Task1: fix round 1/5 (1 addressed, 0 open): changed only the lone-CR
  regex branch to reject a following LF, eliminating exponential CRLF
  backtracking on malformed tails; added the 384-row permanent regression.
  Original snapshot remains at /tmp/ail-csv-task1-initial-freeze.
- Task1: complete (staged, independent spec PASS and quality APPROVE in
  /tmp/ail-csv-task1-rereview.md). No repository integration claimed. The
  reviewer declined AIL/runtime/file/stream correctness and integration
  readiness; Tasks2/3 and preceding final gates explicitly own those checks.
  Frozen 131 input files / 129 cases (50 successful, 79 structured failures),
  fingerprint sha256:421457ed6f6825d12c9d9bb1c268ff25. Replacement manifest
  /tmp/ail-csv-task1-hashes.json SHA256
  d24709fa28f62006a6c7bd23e301dc2758e73d42e319022c48fd8fbfad251332;
  report /tmp/ail-csv-task1-report.md SHA256
  2ec972b4cdbe0b2f5eb3e5e8ca80869598e6fccbe5125b489829f9537dd9f1.
- Task2: staged implementation dispatched after Task1 approval, using
  /tmp/ail-csv-task2-brief.md and the frozen trustworthy-edit compiler.

- Task2 initial freeze: eight staged files, unchanged approved compiler and
  all131 frozen oracle inputs. Unchanged evaluator PASS129 cases on VM/native
  (389 commands, all status0), direct parser76 checks and Core/STE16 checks
  pass. Root verified owned and oracle hashes. Report
  /tmp/ail-csv-task2-report.md; manifest SHA256
  1776fc5962cdf458ae7b0c294c7cd9c448143f1f009cd134c61809c87585b132.
  Independent task review pending; source remains under /tmp only.

- Integration boundary: preceding trustworthy-edit final gates and full frozen
  inventory check passed (/tmp/ail-edit-final-summary.json). The completed
  prior milestone remains unchanged. Captured /tmp/ail-csv-baseline.json before
  this milestone's first repository additions; relevant documentation preimages
  are under /tmp/ail-csv-baseline. Copied all131 approved oracle input files
  byte-identically. Only spec/plan status and links changed from staged drafts;
  their behavioral contract remains identical. Task2 source is still staged and
  waits for independent approval before integration. The repository can now
  accept scoped CSV work; no compiler or old rung edit is authorized.

- Task2: complete (independent spec PASS and quality APPROVE, no findings;
  /tmp/ail-csv-task2-review.md). Root integrated all8 owned files byte-identically
  from approved manifest1776fc59..., retaining oracle fingerprint and compiler.
  Review's four focused probes confirm fatal JSON depth/fuel propagation. A
  roughly16KiB field of8192 doubled quote pairs exhausts ordinary function fuel;
  plain32KiB fields pass. Record this input-dependent bound in Task3 docs;
  no universal byte threshold is claimed. Immutable structured values may copy
  and scalar-indexed operations may rescan prefixes; no linear-time guarantee.
  Task3 owns its declined I/O/export/mutation/docs checks; root owns integration,
  final benchmark/frozen-input checks and continued broader goal.
- Task3: dispatched after Task2 approval and byte-identical source integration.
  Brief /tmp/ail-csv-task3-brief.md. Root captured pretask hash map and allowed
  existing-file preimages at /tmp/ail-csv-task3-baseline.json and
  /tmp/ail-csv-task3-baseline for a narrowly scoped review diff.

- Ruling: run the full20-rung benchmark once, concurrently with Task3 work that
  cannot modify any benchmark program or imported library. Capture results by
  wrapping unchanged evaluate_rung, then call unchanged main with no lock flag;
  all original fingerprint and ratchet checks remain active. Freeze benchmark
  inputs/programs, imported example manifests/specs, evaluator and compiler
  before/after. After all Task3 and final reviews/gates pass, recheck that map
  and record results through unchanged ail_bench.write_lock. This separates
  expensive scoring from delayed publication of the local score without a
  redundant second run. The spec requires the new score only after gates, not
  a particular CLI flag. Cost if the dependency map proves incomplete or any
  bound file changes: discard those results and run the full benchmark again.

- Final metadata scope: root will update bench/LADDER.md alongside SCORE.lock
  and JOURNAL.md after acceptance, to remove the stale claim that all Tier2
  rungs except R026/R027 remain planned. Proposed content is isolated at
  /tmp/ail-csv-LADDER.proposed.md; no benchmark file changes during scoring.
  These three administrative benchmark files are explicit finalization changes;
  every old rung input/program and existing script remains frozen.

- Task3 implementation and independent task review: complete, spec PASS and
  quality APPROVE with no findings (/tmp/ail-csv-task3-review.md). Frozen manifest
  d7f2b0e900c3cbba2623180a1604c731b77b49c01ab58ec423726e309750c42d binds
  15 owned files, 50 evidence files and 48 production artifacts, plus unchanged
  compiler and five library/benchmark files. Root verified all 119 hashes;
  reviewer also recomputed complete native output hashes from independent
  expectations. Script cb34b42f... has actual coverage for every final assertion.
- Acceptance is explicitly composed: 1,806 frozen-case comparisons, 602 direct
  parser checks and 280 I/O/bound checks, totaling 2,688, with 154 preservation
  assertions within I/O and two mutation detections. Its 2,737 primary commands
  include two retained failed native-report harness assertions and 45 builds.
  The planned parser-directory FileExistsError occurs after all 14 app routes
  and is only an orchestration boundary. Separate workers provide the remaining
  checks; no successful full invocation is claimed. Supplemental 14 original-line
  and 80 focused I/O checks, four docs/package/STE tests, 139 local links and
  actual support-loader closure probes pass. No product fix was required.
- Root final metadata staged then applied after Task3 approval: current score
  prose reflects the actual 20/20 benchmark, trustworthy-edit score wording is
  historical, LADDER describes R022, and JOURNAL records measured composed
  evidence. Task3's original reviewed doc hashes remain preserved; root changes
  are separately bound by /tmp/ail-csv-final-metadata.json. SCORE.lock remains19
  until whole-milestone review and final gates pass. Full benchmark results bind
  899 dependencies; only documented non-executable LADDER/JOURNAL changes are
  exempted from final dependency revalidation. Whole-milestone review is pending.

- Whole-milestone review: spec PASS and quality APPROVE, no findings
  (/tmp/ail-csv-final-review.md). The reviewer revalidated all 1,967 snapshot
  hashes, all 155 changed files, 131 frozen oracle inputs, Task3 evidence/artifacts,
  897 unchanged benchmark dependency files plus the two documented status-only
  exceptions, all 20 rung fingerprints and original 19 lock entries. No final
  product fix was needed.
- Final root gates: 1,967 reviewed files unchanged before score publication,
  152 local links with none missing, clean product whitespace and git diff check,
  both Python sources compile, and benchmark dependencies/fingerprints remain
  bound. A root scratch check initially expected the wrong lock-line spelling;
  it was corrected to the actual best-score record and rerun, with no product
  change. The original scratch failure remains recorded under /tmp.
- Score publication: the unchanged existing ail_bench.write_lock recorded the
  saved full 20/20 results only after required reviews/gates and fresh checks.
  All 19 prior fingerprints/pass entries remain; the new R022 fingerprint is
  sha256:421457ed6f6825d12c9d9bb1c268ff25. No --bless-ladder, repeated full
  benchmark, git mutation, dependency change or publication occurred.
- CSV milestone complete. Final summary /tmp/ail-csv-final-summary.json and
  score record /tmp/ail-csv-score-update.json retain evidence and identities.
  Source, library, application, oracle and documentation remain in this workspace.
  The broader Python-replacement and AI-agent language goal remains active.
