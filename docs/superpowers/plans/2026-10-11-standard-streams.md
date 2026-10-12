# Standard-stream execution implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development for the
> disjoint frontend/runtime, native and independent-oracle tasks. Root integrates
> source/VM/CLI/legacy output and docs. Reuse available workers, no nested agents,
> then independently review each released scope.

**Goal:** Real stdin → computation → exact stdout/stderr programs through
checked Core, STE, VM and standalone Linux x86-64 ELF.
**Architecture:** Action-only standard-stream operations, injected borrowed VM
authority, native inherited descriptors, explicit runtime --stdio mode.
**Tech Stack:** Dependency-free Rust2024, direct x86-64 ELF, Python oracle.
**Spec:** ../specs/2026-10-11-standard-streams-design.md

## Global constraints and rulings

Text bytes1,048,576; read at most1,048,577 bytes through EOF; shared Text arena
67,108,864 bytes, successful read charge max(bytes,1), writes uncharged. Retry
Interrupted; short IO progresses; other IO errors and nonempty zero writes are
fatal IO. Nonempty writes flush; empty writes validate and perform no host IO.
Grant first; read order IO/size/UTF-8/arena/commit. Invalid UTF-8 is catchable TEXT;
limits/IO/CAPABILITY remain fatal. Preserve consumed input and completed/partial
effects, destination atomicity and immutable aliases. No fd closure/reopening,
blocking-flag changes, implicit grants, new dependencies or new function effects.

Ruling: the continuing user goal and developer autonomy authorize this increment
without repeating skill approval gates. The existing shared worktree and read-only
.git prohibit commits/worktree replacement/reset/cleanup; preserve all work.
Use `/tmp/ail-stdio-baseline` for reviewed diffs and this persistent plan as ledger.
The cost of a wrong routine design choice is reversible rework; no publication or
external account action is included.

Ruling: explicit disjoint ownership permits parallel worker implementation after
the interface preflight; root owns shared integration. Reuse existing workers
because this long-running session exhausted fresh slots previously. Different
workers perform scoped reviews after release. Root alone runs the final serial
full suite, avoiding duplicate runs and localhost/resource races. Skill requests
for per-worker full suites/commits do not override these environment constraints.

Prior goal turn classified as progress: current89 hashes match and the authoritative
full log records1,163 passes/28targets; previous recovery plan is complete.

## Preflight ownership and interfaces

| Tasks | Interface or shared concern | Ruling |
| --- | --- | --- |
| 1/2 | AilStandardStreamOperation and AilStandardStreams | worker owns new standard_streams.rs; root owns registration/action fields/dispatch |
| 1/2 | parser order, Core edge names, derived accesses | stream parser before file parser; exact stream_step/source/destination, scoped_field |
| 2/3 | scalar opcode admission, legacy quiet mode | root owns ail.rs hooks; native worker exposes legacy initialization/skip helpers |
| 2/4 | exact --stdio flag and early diagnostics | root owns CLI; oracle tests all source/Core/artifact routes and no-op mode |
| 3/4 | partial IO/SIGPIPE/read scratch cleanup | independent bytes/resource-limited oracle; no shared production edits |
| 4/5 | filter record and summary contract | root authors example; oracle independently derives expectations |
| 1 | frontend/runtime tests agree with spec | default-deny borrowed endpoints, exact Core, flush nonempty only |
| 2 | root integration tests agree with spec | clean CLI streams and legacy output without skipping return validation |
| 3 | native tests agree with spec | shared +72 flag, no frame layout change, fatal diagnostics ungated |
| 4 | oracle tests agree with spec | independent bytes and concurrent pipe feeding/draining, bounded timeouts |
| 5/6 | docs/verification agree with spec | bounded EOF, not interactive streaming; preserve full goal and frozen benchmark |

Read-only frontend/native/oracle preflights confirmed feasibility. Load-bearing
decisions: file parser claims broad write prefixes; both CLI reporting funnels
and early diagnostics need mode handling; legacy native has RBP free; scalar root
has unused RBP+72; scalar trace() also emits fatal messages and needs separation;
guard all of trace_value; native write-all completion is flush, never fsync.

## Tasks

### Task 1: Frontend, strict Core and injected VM primitives

**Owner:** function_core. **Files:** new src/standard_streams.rs and embedded
unit tests only. No ail.rs/action_order/main/native edits.
**Produces:** exact public enum/stream-handle struct in spec; pub(super)
parse_bullet, recognized_bullet, elaborate, from_core, check, namespace,
instruction, verify, accesses and execute matching file_operations conventions.
Operation source() renders the three exact source forms. Runtime execute takes
instruction,&mut state,&mut usize Text allocation,&mut AilStandardStreams.

- [x] Observe missing support through a meaningful public parser/protocol or
  module test before runtime implementation; coordinate temporary registration
  with root, not shared-file edits. Record RED command/output.
- [x] Implement exact parser/guard, AST/Core/render/import, genuine global Text
  storage, ownership/order/metadata/duplicate/provenance checks, semantic accesses
  and exact opcode validation using AIL-STDIO-001. No descriptive write edges.
- [x] Implement authority-first borrowed IO with EOF/size/UTF-8/arena/commit
  ordering, write-all/flush policy, empty writes and stable sanitized error codes.
- [x] Focused tests: each endpoint grant, short/interrupted IO, zero writes,
  flush errors, oversized/invalid/empty input, exact consumption/arena, failed
  destination preservation, Core forgeries/secrets/imports and parser coexistence.
  Report focused GREEN, strict library Clippy, owned fmt and release file.

### Task 2: Shared source/VM/STE, CLI and legacy native integration

**Owner:** root. **Files:** src/ail.rs, src/action_order.rs, src/main.rs,
src/ste.rs, targeted function rejection hooks if needed;
new tests/standard_stream_protocol.rs and tests/standard_stream_cli.rs.
**Consumes:** task1 APIs; task3 native_expr::legacy_stdio_initialize(code)
->Result<(),String> and legacy_stdio_skip(code,target).

- [x] Add failing protocol/quiet-CLI tests before hooks. Register module/exports,
  source-model fields/constructors/steps, action_order mapping, elaborate/check/
  from_core/render/import/lower, edge catalog and saved duplicate preservation.
- [x] Wire parser before file operations and recognized-bullet guards; exact
  required opcode operands/profile gates; reject both function IR kinds. Chain
  semantic reads/writes into existing secrecy checks. Add three exact STE templates.
- [x] Add with_io verified root API; existing APIs delegate with empty streams.
  Reborrow stream context through action/repeat execution; route operations at
  existing recovery boundaries and retain traces. Force scalar native admission
  and inlining/fuel wrappers for the new opcodes.
- [x] Add exact CLI --stdio bool for run/vm only, malformed-prefix rejection and
  runtime-only handle locks. Preserve old reporting without flag; quiet both run
  funnels and put all early/fatal/named-rule diagnostics on stderr in mode.
- [x] Hook legacy native initialization after argv capture. Guard whole ordinary
  trace/state/copy paths; suppress failure trace details while retaining failure
  name. Do not skip computation or return validation. Test quiet scalar/legacy
  programs with no stream opcodes, early-invalid artifacts, unchanged old output.
- [x] Run focused protocol/CLI/old file/recovery regressions; record evidence.

### Task 3: Direct native standard streams and runtime mode

**Owner:** native_control. **Files:** new src/native_stdio.rs; src/native_expr.rs
and necessary native_* files only. No ail.rs or main edits.
**Produces:** two legacy helper wrappers specified above and internal scalar
helpers/opcode handlers; shared mode at RBP+72, ROOT_BYTES80 unchanged.

- [x] Observe RED raw opcodes and --stdio behavior before implementing helpers.
- [x] Parse/validate --stdio before program effects in scalar and legacy startup,
  preserve argv registers and initialize shared context. Ignore SIGPIPE in mode,
  use self-contained legacy error labels and best-effort diagnostics.
- [x] Implement fd0 bounded EOF reads with tracked scratch, UTF-8/arena/atomic
  commit and no descriptor ownership. Implement fd1/fd2 full writes, EINTR retry,
  nonempty zero/other errors fatal, no fsync. Preserve register/fuel contracts.
- [x] Separate diagnostics from ordinary trace output, guard all of trace_value
  and final default/selected state rendering, retain return checks and function/
  recovery scope semantics. Automatic failure trace details are quiet in mode.
- [x] Focused native tests include aliases, called cursor/EOF, exact limits,
  malformed flags before effects, short pipes, invalid UTF-8 recovery, closed/
  broken fds, full arenas/write freedom and repeated scratch cleanup under limits.
  Run owned compatibility/Clippy/fmt; release/report integration hooks.

### Task 4: Independent public acceptance and Python oracle

**Owner:** review_ste. **Files:** new tests/standard_streams.rs and
scripts/ail_stdio_differential.py. No production/example edits.

- [x] Observe current public CLI RED before root hooks. Add exact-byte tests
  across source/saved Core/rendered source, directly and through STE, VM/native.
- [x] Cover authority/default denial/spoofing, exact flags, early diagnostics,
  pure-function/profile/Core rejection, normal legacy/scalar quiet mode, semantic
  secret protection, import identity, cursor/recovery/aliases and fatal outcomes.
- [x] Use concurrent input feeding and both-output draining with hard child
  deadlines. Test full-capacity bidirectional pipes, limit+1 with producer still
  open, split Unicode, merged stdout/stderr order, broken/closed descriptors and
  cleanup under resource constraints. Never assert impossible stderr delivery.
- [x] Independently calculate randomized JSON-lines stdout records and stderr
  summaries for task5 across all12 routes; exercise a real two-process AIL pipe.
  Record fixed seed/counts, full oracle and focused test GREEN, then release.

### Task 5: Real filter, reference and agent guidance

**Owner:** root. **Files:** examples/json-lines-filter (package/spec/input/README),
examples/ste/json-lines-filter.ail-ste, docs/ail/52-standard-streams.md, relevant
indices/source/schema/STE/runtime/file/failure references and agent guide.

- [x] Implement the spec's per-accepted-row stdout JSON filter and ordered stderr
  summary using existing parse/divide/recovery functions, real standard streams.
- [x] Exercise sample on VM/native and export STE; explain invocation/limits/
  partial effects/grants/quiet mode. Update stale file chapter claim about richer
  function types while linking the now-available standard streams.

### Task 6: Independent review and final verification

**Owner:** root coordinates reciprocal reviews; no reviewer approves own scope.

- [x] Generate baseline diffs, review frontend/runtime, native, root CLI/legacy
  and oracle separately; resolve concrete findings and re-review scoped fixes.
- [x] Fresh build; final serial all-target Rust suite (including localhost tests),
  strict all-target Clippy, fmt, diff, new and all15 prior focused differential
  runners, frozen16/16 benchmark and broad differential. Verify reviewed hashes,
  documentation links and unchanged SCORE.lock. Complete this milestone ledger
  only with evidence; keep the full Python/agent goal active.

## Review focus

1. Quiet rendering must never skip return validation, named failures or real effects.
2. A pipe producer may keep stdin open after LIMIT+1 bytes; fail promptly, do not
   wait for EOF or consume beyond the sentinel byte.
3. A program can write more than pipe capacity before reading; the test harness
   must not deadlock and mistake its own behavior for a runtime defect.
4. Invalid UTF-8 read consumes input but cannot replace destination/refund prior
   allocations; catches must free scratch without closing inherited descriptors.
5. Fatal and ordinary trace paths currently share helpers: mode must keep fatal
   diagnostics and explicit stderr, while suppressing all generated trace bytes.

## Evidence ledger

Preflight reports: `/tmp/ail-stdio-frontend-preflight.md`,
`/tmp/ail-stdio-native-preflight.md`, `/tmp/ail-stdio-oracle-preflight.md`.
Primary Linux read/write and Rust SIGPIPE documentation informed IO decisions;
no quotations are incorporated. Spec/plan self-review found consistent exact
API, source/Core/opcode names, bounds, flush/empty policy and ownership. No
implementation was present at preflight; implementation evidence follows.


Root Task2 RED: `/tmp/ail-stdio-root-protocol-red.log` fails because stream writes
are misclassified as file operations; `/tmp/ail-stdio-root-red.log` rejects the
mode and does not classify malformed variants as capabilities. Initial quiet
fixture was corrected from nonexistent Compute to the existing AddNumbers before
recording the meaningful quiet-mode failure. Independent CLI/native/module RED
also recorded by workers before implementation.

Root Task2 GREEN: `/tmp/ail-stdio-root-first-green.log` records4 protocol/CLI
passes. `/tmp/ail-stdio-root-regressions.log` records focused file/recovery/protocol/
CLI passes. Task5 sample VM/native exact stdout and stderr compare equal:
`/tmp/ail-stdio-example-{vm,native}.{out,err}`. Exported STE generated by current
compiler. Source, Core, STE, execution, failure, diagnostic, file, agent guide and
README references updated. Worker acceptance/oracle and independent reviews are
still pending; these are milestone progress, not the final verification gate.


Oracle discovery: Rust startup normalizes already-closed fd0/1/2 to /dev/null
before CLI main. Root confirmed the primary std Unix startup implementation; a
strace attempt was unavailable under ptrace sandbox restrictions. The spec and
chapter52 now distinguish CLI host normalization from direct-ELF closed-fd IO.
Actual broken pipes and injected VM IO errors retain parity. Oracle updates test
this distinction explicitly; no production workaround or silent test omission.

Review fixes: the oracle reproduced stderr diagnostic recursion through eprintln
(exit101). Root regression `/tmp/ail-stdio-diagnostic-red.log` reproduced it using
/dev/full; ignored diagnostic writeln now covers main, early-check and named-fail
paths. `/tmp/ail-stdio-diagnostic-green.log`:3 tests pass. Root then detected
StdinLock prefetch past the sentinel: `/tmp/ail-stdio-prefetch-red.log` measured
1,056,768 bytes consumed instead of1,048,577. A borrowed Unix fd Read adapter now
retains the lock and avoids buffering/open/dup/seek/close. Non-Unix CLI mode is
explicitly unsupported pending an equivalent adapter; injected library IO remains
portable. The independent oracle adds a nonzero-start shared cursor check.
Chapter52/spec document host boundaries. Redirected example commands now use
cargo --quiet to prevent Cargo progress text contaminating application stderr.


Tasks1/3/4 released. Task1 module15 focusedtests; Task3 native compatibility202
including17newtests; Task4 integration13tests and independentoracle100transport
cases×12routes plus60filterreports×12routes and4realpipelines allpass.
Reports: `/tmp/ail-stdio-task-{1,3,4}-report.md`.
Root revised protocol/CLI6tests pass (`/tmp/ail-stdio-root-revised-green.log`).
Independent root review approved (`/tmp/ail-stdio-root-independent-review.md`),
with6regressiontests and extra publicAPI repeat/verification/purefunction probes.
Independent frontend review approved (`/tmp/ail-stdio-frontend-independent-review.md`),
with15module+2protocol+8adversarial checks. Root reviewed the independent oracle's
expectations/route construction/pipe harness and requested timeout kill/reap cleanup;
worker applied it and reran the oracle green. Native independent review pending.
142relative documentation links resolve; frozen bench/SCORE.lock hash still matches
baseline ba0bd203c1a69da4a5ce29593a6a18884b56c89a1c00ef8294e762ca616e1adf.

Native review found one startup-only diagnostic edge: malformed --stdio-prefixed
flags branch to a stderr diagnostic before SIGPIPE ignore is installed. With a
broken fd2, both native emitters could terminate by SIGPIPE (including a valid
flag before/after the malformed one). Evidence `/tmp/ail-stdio-native-startup-review.log`.
Root accepted the finding and requested terminal diagnostic signal protection,
without executing program effects or granting malformed mode. Native worker owns
focused RED/GREEN; independent reviewer will recheck the released fix before
final suite/hash snapshot. No other native review findings.

Native startup regression: `/tmp/ail-stdio-native-startup-red.log` reproduces six
SIGPIPE failures across both emitters and invalid flag order/alone. The narrow
terminal error path now protects diagnostic writes independently of stream mode;
if protection setup fails it exits1 immediately without attempting stderr.
Normal valid/no-flag initialization remains unchanged. Focused validation and
independent recheck are running before final verification.

All implementation reviews approved with no open findings. The final native
startup fix passes18focusedtests and independent6-case startup replay; refreshed
acceptance13tests and the complete new oracle also pass on its exact source.
Native review report `/tmp/ail-stdio-native-independent-review.md` records final
native_stdio hash140f4b2c552f462db8d03d34235ac477c82c6470d2cbd20ec61d9417facee6bd.
Final97source/test/toolchain/oracle inputs snapshotted in
`/tmp/ail-stdio-reviewed-inputs.json`. Full serial all-target suite is running with
localhost permission; cargo fmt and git diff --check pass. The new oracle's fresh
independent replay is reused for this final gate; root runs all15 prior focused
oracles, frozen benchmark and broad differential after the suite, avoiding an
unnecessary duplicate of the unchanged new-oracle replay.


Final Rust verification: `/tmp/ail-stdio-final-tests.log` completes with exit0,
1,215passed across31targets,0failed,0ignored (summary JSON alongside). Strict
all-target Clippy and fresh cargo build pass (`/tmp/ail-stdio-final-clippy.log`,
`/tmp/ail-stdio-final-build.log`). Cargo fmt --all -- --check and git diff --check
pass. After tests/build, all97 reviewed inputs and18 reviewed documentation/
example hashes still match their manifests. Exported example STE matches fresh
compiler output. Prior differential replay is running serially; its per-runner
exit/timing/log ledger is `/tmp/ail-stdio-final-runners.json`.

Final verification gate complete. All 15 prior focused differential runners pass,
alongside the independently replayed standard-stream oracle (seed 359696): 100
transport cases across 12 routes, 60 JSON-lines datasets across 12 routes, and
four real two-process pipelines. The frozen benchmark remains 16/16. Broad
VM/native differential reports 33 agreements, 30 cases not natively compilable,
and zero divergences. All 17 regression-runner processes exited successfully;
their logs and timings are recorded in `/tmp/ail-stdio-final-runners.json`.

After all runners finished, all 97 reviewed source/test/toolchain/oracle hashes
and all 18 documentation/example hashes still matched. The frozen SCORE.lock
hash remained unchanged and git diff --check passed. Together with the final
1,215 Rust tests, strict Clippy, fresh build, formatting, exact example STE
comparison, 142 resolved relative documentation links, and approved independent
reviews, this completes the standard-stream milestone. The broader Python
replacement and AI-agent language goal remains active.
