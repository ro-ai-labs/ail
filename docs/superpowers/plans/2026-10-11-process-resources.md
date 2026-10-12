# Process Resources Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Track checked steps and evidence here.

**Goal:** Run caller-authorized external tools from AIL with bounded communication,
recoverable launch failures/timeouts, and equivalent standalone native behavior.

**Architecture:** A checked action operation carries six typed field operands and
one named resource. A shared no_std Linux engine owns process/descriptor lifetime;
VM and native adapters validate inputs and atomically publish typed results.

**Tech Stack:** Rust 2024 without dependencies, direct Linux x86-64 ELF, embedded
no_std runtime, independent Python oracle and external test helpers.

**Spec:** [Process design](../specs/2026-10-11-process-resources-design.md).

## Global constraints

- Preserve unrelated worktree changes; .git is read-only. No commits/worktrees.
- Linux x86-64, close_range (Linux 5.9+) for the new effect; no implicit shell/PATH.
- Empty child environment, inherited cwd; private streams and no inherited fd>2.
- Resource names <=256 bytes; absolute executable path <=4,095 bytes, no NUL.
- <=256 argument strings; <=65,536 bytes including argv0 and all terminators.
- Timeout 1..60,000 milliseconds; input/captures <=1,048,576 bytes each.
- Atomic three-field commit and combined charge to shared 67,108,864 Text budget.
- Exact PROCESS_RUN resource,arguments,input,timeout_ms,stdout,stderr,status.
- PROCESS/TIMEOUT are new catchable categories, appended after existing ten.
- Engine errors return only after owned-child cleanup; no parent-global signal change.
- Reuse existing workers with disjoint ownership, then reciprocal independent
  reviews. Root owns integration. No worker creates agents or runs broad suites.

## Review focus

1. Child closes input early: avoid SIGPIPE/deadlock and preserve actual status.
2. Both outputs exceed pipe capacity before input consumption: concurrent fairness.
3. Exited leader leaves descendant holding stdout: deadline still works, cleanup
   cannot target a reused PID, and no child survives in the owned group.
4. Second output invalid/exhausts arena: no partial destination or charge commit.
5. Closed standard fds and inherited high fds: normalized plumbing leaks nothing.

## Task 1: Frontend operation and VM adapter

**Owner:** function_core. **Create:** src/process_operations.rs.
**Root provides:** module/re-export wiring, AST container fields, ordered hooks,
VM with_resources context and recovery categories (Task 2).

**Interfaces:** AilProcessOperation::{Run{resource,arguments,input,timeout_ms,
stdout,stderr,status},Invalid(String)}; AilProcessResources{executables};
parse_binding, validate_resources, parse_bullet, recognized_bullet, elaborate,
check, from_core, render, namespace, instruction, verify, accesses, execute.
Follow standard_streams/file_operations signatures; execute adds
&AilProcessResources to instruction,state,text_allocated. VM uses Task3 ABI.

- [x] Write module tests for source/Core exactness, ownership, typed operands,
  output alias rejection, profile/function guards and binding validation. Record
  RED from missing module/API or rejected source before implementation.
- [x] Implement the exact frontend contract and VM validation order, marshal
  C strings/argv and capture buffers into the shared host engine on supported
  hosts. Validate both outputs/combined arena before three-field commit.
- [x] Focused GREEN: cargo test --lib process_operations -- --test-threads=1;
  authority-before-operand tests, invalid second output/atomic arena, aliases,
  catchable PROCESS/TIMEOUT cleanup. Report /tmp/ail-process-task-1-report.md.

## Task 2: Root source, protocol, API, CLI and recovery integration

**Owner:** root. **Modify:** src/lib.rs, src/ail.rs, action_order.rs, ste.rs, main.rs,
recovery.rs. **Create:** tests/process_protocol.rs, tests/process_cli.rs.

**Interfaces:** run_ail_bytecode_action_with_resources(program,action,state,
files,streams,processes)->Result<AilRunResult,String>. Prior APIs delegate with
default-denied processes. Root registers cfg Linux x86-64 process_host module.

- [x] Write meaningful failing end-to-end protocol/CLI tests: accepted source
  lowers to PROCESS_RUN, STE roundtrip exactness, missing capability, malformed
  flag rejection before prior output/effects, two new recovery categories.
- [x] Wire source parse/render/Core serialization/elaboration/check/imports,
  action order/accesses, exact bytecode/profile/function validation, native
  route selection, STE template, public APIs and shared context through calls.
- [x] Add CLI resources and --run-process parser, unsupported-command rejection,
  usage and additive with_resources call; preserve stdio/default reporting.
- [x] Append PROCESS/TIMEOUT to category/code arrays, preserving old indices.
  Focused GREEN on root tests and relevant stdio/recovery suites; record logs.

## Task 3: Shared host engine and native embedding

**Owner:** native_control. **Create:** src/process_host.rs, src/native_process.rs.
**Modify:** runtime/float_runtime.rs, build.rs, src/native_expr.rs,
src/native_recovery.rs, src/native_float_runtime.rs only as necessary.

**Interfaces:** exact 64-byte ProcessRequest, 24-byte ProcessResult and
ail_process_run ABI/statuses from spec. Native grants/fields/initialize/
emit_operation/helpers follow existing native_file interfaces. Root calls
legacy process-option validator if a legacy emitter needs it; worker exports
the narrow wrapper. PROCESS_RUN always selects scalar emitter.

- [x] Record focused RED for unsupported PROCESS_RUN and absent exported host
  runtime. Add lifecycle tests with real helpers, not mocked syscalls.
- [x] Implement no_std syscall engine, finite poll deadline, nonblocking parent
  communication, SIGPIPE-safe stdin, child signal/descriptor setup, exec error
  channel, WNOWAIT ownership, kill/group/reap cleanup and exact result ABI.
- [x] Embed runtime export; implement native grants, field validation/marshaling,
  bounded capture scratch, UTF-8 validation, atomic charge/commit and diagnostic
  recovery labels. Validate every grant even without reachable process op;
  grants never become mutable fields or get embedded in artifacts.
- [x] Focused native GREEN covers review-focus cases, exit127/exec failure,
  nonzero/signal status, launch/timeout recovery, exact bounds, aliases, repeated
  calls, inherited fd/signal isolation, no parent signal changes or leaks.
  Use escalation for actual local IPC if sandbox send is denied. Record exact
  commands/logs and final ABI in /tmp/ail-process-task-3-report.md.

## Task 4: Independent acceptance and differential oracle

**Owner:** condition_ast (reused available worker). **Create:** tests/process_resources.rs,
scripts/ail_process_differential.py (fixture helpers may be temp compiled).

- [x] Record RED acceptance against absent operation/flags. Cover public source,
  saved Core, rendered source and STE paths, malformed graphs/instructions,
  resource scopes/imports, functions/profiles and whole-program validation.
- [x] Compute independent exact observations across twelve routes using fixed
  seed: argument boundaries/metacharacters/Unicode, stdin NUL/newlines, separate
  outputs and status; child env/fds/signals/cwd; early input close and large
  simultaneous transfers. Do not use VM outputs as native expectations.
- [x] Add bounded process-group-aware harness timeout cleanup. Cover exact and
  over limits, invalid output UTF-8/atomic results, execution failure vs127,
  timeout with descendant-held pipes, recovery/earlier effects, fd/scratch/child
  cleanup and usable Task5 coordinator datasets. Full oracle GREEN after final
  implementation; report seed/counts in /tmp/ail-process-task-4-report.md.

## Task 5: AIL tool coordinator and documentation

**Owner:** root. **Create:** examples/tool-coordinator (package/spec/input/
README), examples/ste/tool-coordinator.ail-ste, docs/ail/53-process-resources.md.
**Modify:** README, docs/example indices, source/Core/STE/execution/recovery/
diagnostics references and agent authoring guide.

- [x] Build an actual coordinator that consumes JSON tasks via stdin, prepares
  argv/input/timeouts, invokes a separately compiled AIL worker resource and
  emits exact JSON records including nonzero and recovered tool errors.
- [x] Verify source/STE VM/native runs and exported STE; document invocation,
  authority/OS boundaries, limits, atomic results, cleanup and twelve recovery
  categories. Add reproducible evidence without claiming full Python readiness.

## Task 6: Independent reviews and final verification

**Owner:** root coordinates; no reviewer approves their own scope.

- [x] Review baseline diffs for frontend/VM, shared engine/native, root integration
  and independent oracle separately. Resolve findings and re-review scoped fixes.
- [x] Fresh build; serial all-target Rust suite (with needed local IPC authority),
  strict all-target Clippy, fmt, diff checks; new and all sixteen prior focused
  oracles, frozen benchmark16/16 and broad differential. Verify reviewed hashes,
  exact STE exports and documentation links. Keep full goal active.

## Evidence and decisions

Previous turn classified as progress: standard-stream milestone completed and
verified. Process preflight uses authoritative current source, frontend/oracle
reports in /tmp/ail-process-*-preflight.md, and the native worker's read-only
architectural findings; the native implementation report records the final ABI. Baseline snapshot contains
98 source/test/build/runtime/oracle files; all 97 prior reviewed hashes match.

Preflight interface review: Tasks1/2 share module/AST/runtime context signatures;
Tasks1/3 share the fixed request/result ABI; Tasks2/3 share native selection and
new recovery indices; Tasks4/5 share the coordinator workload. Each pair's types,
field names and bounds are fixed in this spec/plan. Task ownership is disjoint;
native runtime and engine remain one implementation owner. Root performs shared
integration while workers remain within their files. Every task has its own
failure evidence and focused green check; only root runs final broad checks.

Design self-review resolved subprocess status semantics, early stdin close,
atomic double-output accounting, signal/descriptor inheritance, timeout while
draining descendants, PID retention and explicit unsupported-host behavior.
No placeholder requirements or open interface decisions remain.

Root RED: /tmp/ail-process-root-red.log rejects malformed flags with the old
state-argument diagnostic; /tmp/ail-process-protocol-red.log rejects the unknown
process statement inside a recovery block. Corrected the fixture recovery
literal from an unsupported `assigns literal 9` spelling to `assigns 9` before
subsequent runs. Module RED /tmp/ail-process-module-first.log records seven
stub failures. Root wiring is in progress. Task4 moved to the available existing
condition_ast worker after review_ste could not be resumed at the thread limit.

Root initial GREEN: /tmp/ail-process-root-first-green.log records 4 protocol/CLI
passes. /tmp/ail-process-recovery-mask-{red,green}.log documents the expanded any
mask and preserved old/new category bits (2 focused tests green). Source/Core/STE
coordinator checks pass; VM sample runs the actual compiled AIL filter worker and
all five independently expected outcomes pass, including continuation after a
PROCESS recovery. /tmp/ail-process-example-vm.{out,err} stores exact bytes. Native
coordinator verification is pending. Exported STE added; 261 relative reference
targets resolve. Root formatting and git diff --check pass at this point.

Native coordinator sample now matches the five independently checked VM records
byte for byte, with empty parent stderr: /tmp/ail-process-example-native.{out,err}.
A new adversarial declaration test exposed saved-Core has_field deduplication: the
in-memory checker rejected duplicate ownership but the parser erased evidence.
Root traced the fallback Graph.add_edge call and preserved has_field edges in the
same raw structural-edge path. RED/GREEN logs are /tmp/ail-process-core-owner-
{red,green}.log; the all-six-operand ownership test now passes. Valid generated
declaration edges are already unique, so their behavior is unchanged.

STE self-review caught an overlong process template, confirmed by the unchanged
20-word profile test in /tmp/ail-process-ste-length-red.log. Shortened it to the
18-word imperative now in the spec/reference; the exact seven operands and all
source/runtime semantics remain unchanged. This preserves the original STE
profile rather than weakening its existing test. Regenerate exports and review
hashes after the focused STE roundtrip check.

Root compatibility checks pass: /tmp/ail-process-root-regression-green.log has
17 tests across process protocol/CLI, prior stream protocol/CLI and recovery
protocol. The new 18-word STE template passes both unchanged template/roundtrip
tests (/tmp/ail-process-ste-green.log). Corrected a copied internal index-error
label from STDIO to PROCESS after independent root review; refreshed review
manifest /tmp/ail-process-root-review-hashes.json and exported coordinator STE.

Task1 released at a6914f6913ac with 16 focused tests, strict library Clippy and
formatting passing (/tmp/ail-process-task-1-report.md). Task3 released its six
files with 8 native, 7 shared-host and 9 embedded-runtime tests passing, plus
strict library Clippy, formatting and a rebuilt CLI
(/tmp/ail-process-task-3-report.md). The independent oracle found a native
default-output grant-path leak; its RED/GREEN regression is recorded in the
Task3 report and reserved process flags are now excluded from unknown fields.

Independent root integration review approves its eight-file scope after seven
additional public probes, four protocol/CLI tests and two recovery tests. The
probe coverage includes mixed ordering through twelve process steps, default
denial in all older APIs, all-binding validation before effects, whole-program
validation, call/repeat context forwarding, legacy option validation and fatal
limit versus catchable process errors. Report and final hashes:
/tmp/ail-process-root-independent-review.md. Reciprocal frontend and host/native
reviews are in progress; author reports do not stand in for those reviews.

Task4 released with six Rust acceptance tests and a full independent oracle at
seed0xA11C011: 589 assertions, 645 bounded commands, all six sections passing.
The final log is /tmp/ail-process-oracle-green.log; report is
/tmp/ail-process-task-4-report.md. Root reviewed both acceptance/oracle files,
including independent expectations and session/subreaper timeout cleanup, with
no unresolved finding (/tmp/ail-process-oracle-independent-review.md).
Added the missing standard-stream/process entries to the top-level docs index.

The frontend reviewer reproduced an in-memory Core ambiguity: a second process
node with the same id could be ignored. Root added process-scoped unique identity
resolution for the operation, executable owner, six fields, declaring Things and
provenance targets. The new regression checks appended and prepended duplicates;
/tmp/ail-process-identity-red.log records thirteen previously accepted variants
and /tmp/ail-process-identity-green.log records the passing check. This concerns
in-memory identities; saved Core regenerates identities from declarations, so
existing saved-Core structural regressions remain separate. No generic Graph
semantics changed. Frontend review manifest has been refreshed for re-review.

Documentation/example review approves twenty files after clarifying that the
older general ExternalCall binding model differs from the implemented file,
stream and process operations. Report: /tmp/ail-process-docs-independent-review.md.
The exported coordinator STE matches a fresh export byte for byte. Root checked
193 relative links across the current documentation/example set, none missing.

All independent reviews are now approved with no unresolved findings. The final
frontend report (/tmp/ail-process-frontend-independent-review.md) rechecks the
original identity forgeries and eleven independent VM adapter probes; module hash
is 2d4da8761fb85aa2827b1736b282e27e5e15f07ce74e8f49f7dcd3434dab0a10.
The native report (/tmp/ail-process-native-independent-review.md) confirms the six
released hashes and adds independent descriptor-exhaustion, signal-interruption,
ABI, cleanup, exact binding-limit and legacy/scalar privacy probes.

The combined verification manifest /tmp/ail-process-reviewed-inputs.json freezes
105 source/test/build/runtime/oracle inputs, including every Rust file under
src/tests/runtime. Every changed baseline file belongs to a reviewed scope.
Root's serial final runner /tmp/ail-process-final-gates.py checks this manifest
before each gate. All seventeen focused oracles will run after the final frontend
fix, followed by frozen benchmark and broad differential checks.

## Final milestone result

The process-resource milestone is complete. The broader active goal of making
AIL a Python replacement for AI-agent work remains open; this increment proves
caller-authorized synchronous tool coordination, not complete Python readiness.

Final reviewed implementation verification (all exit0):

- Fresh build, strict all-target Clippy, cargo fmt check and git diff check.
- Serial all-target Rust suite: **1,257 passed across 34 targets, 0 failed,
  0 ignored**. Log: /tmp/ail-process-final-tests.log.
- All **17 focused differential oracles**, including a fresh process oracle after
  the identity fix: seed0xA11C011, **589 assertions / 645 bounded commands**.
  Final runner ledger: /tmp/ail-process-final-runners.json.
- Frozen benchmark: **16/16**. SCORE.lock remains
  ba0bd203c1a69da4a5ce29593a6a18884b56c89a1c00ef8294e762ca616e1adf.
- Broad VM/native comparison: **33 agree, 30 not natively compilable, 0 diverged**.
  These known unsupported examples are not counted as native successes.
- All **105** reviewed source/test/build/runtime/oracle hashes remained unchanged
  through the final gates. Gate commands, statuses and timings are recorded in
  /tmp/ail-process-final-gates.json; the manifest is
  /tmp/ail-process-reviewed-inputs.json.
- A final rebuilt-CLI coordinator STE export matches the checked-in artifact.
  All **196** checked relative documentation links resolve after the external-call
  clarification. Documentation/example hashes are recorded in
  /tmp/ail-process-reviewed-docs.json.

No commits or unrelated workspace changes were made. Independent reviews have no
unresolved findings; details and scoped hashes are in the five review reports
recorded above. No additional broad test reruns are needed without a new change.
