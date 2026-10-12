# Directory Resources Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Track checked steps and evidence here.

**Goal:** Let AIL discover files and produce computed reports under explicit caller directory grants.

**Architecture:** Typed ordered directory operations carry relative Text paths.
A shared no_std Linux engine owns secure resolution, bounded collection and text
transfer; VM/native adapters own immutable authority and atomic arena publication.

**Tech Stack:** Rust2024 without dependencies, direct Linux x86-64 ELF, embedded
no_std runtime, independently calculated Python filesystem oracle.

**Spec:** [Directory resources design](../specs/2026-10-11-directory-resources-design.md).

## Global constraints

- Preserve the existing worktree; .git is read-only. No commits/worktrees.
- Reuse current workers with disjoint file ownership; no worker spawns agents.
- New effect Linux x86-64 with openat2 (Linux5.6+), BENEATH|NO_SYMLINKS, no fallback.
- Caller roots absolute, no NUL, <=4,095 encoded bytes; names <=256 qualified ASCII bytes.
- Per-operation root resolution/pinning. Root symlinks allowed; relative symlinks rejected.
- Relative paths <=4,095 UTF-8 bytes; strict components; empty only for root listing.
- Input/read Text <=1,048,576 bytes. Listing <=65,536 names, <=1,048,576 packed raw
  name bytes excluding separators, and <=1,048,576 canonical List<Text> bytes.
- Separate shared Text and List/Json budgets:67,108,864 bytes each. List/read atomic
  charge and destination commit; writes may retain partial external effects.
- PATH appended at index12; any8191; existing error indices/fatal boundaries preserved.
- Exact engine ABI80/16 and status mapping from spec; fixed0x311000 scratch and
 1,048,577-byte output, no engine heap/runtime compiler/external tool.

## Review focus

1. Symlink/ancestor substitution: kernel-relative resolution never reads or
   truncates an outside sentinel; distinguish hard links and selected-root changes.
2. Destination aliases input path: validate/operate before replacing it; old copies
   and shared arena budgets survive read/list failures and later results.
3. Control/Unicode names and raw versus canonical size: exact escapes/order/bounds,
   including invalid UTF-8 discovered only after bounded collection.
4. Invalid/unused/overwritten grants: fail before earlier effects in both native
   emitters, never expose authority as mutable/default-output state.
5. Nonregular files and partial failures: no FIFO wait or data transfer to devices;
   close owned fds/scratch before handlers without claiming rollback/durability.

## Task1: Typed frontend and VM adapter

**Owner:** function_core. **Create:** src/directory_operations.rs.
**Root provides:** AST container/order, source/Core hooks, module wiring and host
resource dispatch from Task2. Task3 supplies engine ABI.

**Interfaces:** AilDirectoryOperation::{ListNames{resource,path,key},
ReadText{resource,path,key},WriteText{resource,path,source},Invalid(String)} with
source(). AilDirectoryResources{reads,writes}. Functions parse_binding,
validate_resources, recognized_bullet, parse_bullet, elaborate, check, from_core,
namespace, instruction, verify, accesses. Signatures mirror process_operations.
execute(instruction:&AilBytecodeInstruction,state:&mut BTreeMap<String,String>,
text_allocated:&mut usize,list_allocated:&mut usize,
resources:&AilDirectoryResources)->Result<(),String>.

- [x] Add failing tests for exact source/three opcodes, all roles/type/ownership,
  ambiguous IDs, saved duplicates, import qualification, profile/function guards,
  resource/path/field authority and atomic result accounting. Record meaningful RED.
- [x] Implement frontend and strict typed Core decoding. Listing writes List<Text>;
  read writes Text; every op semantically reads path, write also reads source.
- [x] Marshal nullable field slices to the shared validator before fallible scratch
  allocation; execute the engine, map errors, reserve exactly one correct arena
  result, and publish immutable output. Old values/counters survive failure.
- [x] Focused GREEN: cargo test --lib directory_operations -- --test-threads=1.
  Real files cover bounds, relative read/write, aliases, PATH/TEXT recovery and
  arena exhaustion. Report /tmp/ail-directory-task-1-report.md with final hashes.

## Task2: Root source, protocol, host context, CLI and recovery

**Owner:** root. **Modify:** src/ail.rs, lib.rs, action_order.rs, ste.rs, main.rs,
recovery.rs. **Create:** tests/directory_protocol.rs, tests/directory_cli.rs.

**Interfaces:** Public AilHostResources{files,processes,directories}, Default/Clone/
Debug. Add run_ail_bytecode_action_with_host_resources(program,action,state,
&resources,&mut streams)->Result<AilRunResult,String>. Preserve every older API
signature, delegate through a private borrowed context with default-denied dirs.
Root registers cfg Linuxx86_64 crate::directory_host; exports directory types,
parser and the native legacy-directory initialization call from Task3.

- [x] Write failing end-to-end source/Core/STE/CLI tests, recovery PATH/any masks,
  invalid grants before stdout and default denial through older APIs. Record RED.
- [x] Wire AST/storage/order/render/import/Core/semantic accesses, parser priority
  before broad file writes, reserved malformed function/pass guards, exact opcode
  verification over all actions and native selection. Preserve Core edge multiplicity.
- [x] Add aggregate+borrowed contexts through recursive/repeated dispatch and all
  binding validation before effects. CLI supports both directory flags only on
  execution commands, validates every occurrence and retains stdio reporting.
- [x] Append PATH category/code at index12 and update exact-mask assertions while
  proving all prior indices unchanged. Focused GREEN includes prior process and
  stream protocol suites; log evidence in this ledger.

## Task3: Shared directory engine and standalone native adapter

**Owner:** native_control. **Create:** src/directory_host.rs, src/native_directory.rs.
**Modify:** src/native_expr.rs, native_recovery.rs, build.rs,
runtime/float_runtime.rs; native_float_runtime.rs only if necessary.

**Interfaces:** Exact DirectoryRequest80, DirectoryResult16,
ail_directory_validate and ail_directory_run exports, statuses0..5 from spec.
Native grants/fields/initialize/emit_operation/helpers mirror native_process.
Export native_expr::legacy_directory_initialize(&mut X64Code)->Result<(),String>
for root to call after existing legacy process initialization.

- [x] Record RED for absent native opcodes and engine behavior. Real filesystem
  fixtures verify directory listing, nested reads/writes and sentinel protection.
- [x] Implement shared typed/path validation, fd ownership/openat2, strict regular
  fstat checks, bounded raw listing, UTF-8, stable sorting/canonical escapes, bounded
  transfer, exact errno map and close-without-retry cleanup. No d_type assumptions.
- [x] Embed exports and build dependencies; native startup checks every flag for
  scalar/legacy, immutable grant slots and default-output exclusion. Marshal slices,
  validate before tracked scratch allocation, preserve ABI/registers, allocate and
  commit in the correct persistent arena after success, unwind on recovery.
- [x] Focused GREEN: exact/root/nested paths, all operation limits and aliases,
  partial descriptor failures/repeated calls, special files, invalid outputs,
  overwrite/privacy flags, PATH recovery and existing embedded-runtime compatibility.
  Report /tmp/ail-directory-task-3-report.md with ABI, logs and six-file hashes.

## Task4: Independent public acceptance and filesystem oracle

**Owner:** condition_ast. **Create:** tests/directory_resources.rs,
scripts/ail_directory_differential.py.

- [x] Record RED for unsupported source/opcode; public Rust tests require all
  representations, hidden/duplicate/ambiguous graph rejection, imports, functions,
  profiles, unreachable malformed bytecode, old API default denial and all bindings.
- [x] Implement independent Python expectations across source/saved Core/rendered
  source/direct STE × VM/native. Cover exact names/order/escapes, nested file bytes,
  read/write permission separation, unused/duplicate flags, errors and aliases.
- [x] Add bounded adversarial filesystem fixtures: outside sentinels, symlinks,
  traversal, hard-link boundary, root replacement, nonregular files, invalid UTF-8,
  limits/budgets, cleanup, partial writes and prior effects. Keep race evidence
  bounded and separate from static exact assertions. Preserve fixed-file behavior.
- [x] Independently calculate Task5 workload outputs/manifest, including successful
  and rejected entries and continuation. Run full oracle after implementation and
  fixes; report seed/counts/logs/hashes at /tmp/ail-directory-task-4-report.md.

## Task5: Discovered-file AIL workload and references

**Owner:** root. **Create:** examples/directory-report (package/source/input fixtures/
README), examples/ste/directory-report.ail-ste, docs/ail/54-directory-resources.md.
**Modify:** README, docs/example indices, source/Core/STE/execution/recovery/
diagnostics references and agent authoring guide.

- [x] Build a pure-AIL batch reporter: discover sorted basenames, process each JSONL
  file, recover bad paths/text/data per entry, compute output names from sorted
  numeric indices, and write an ordered manifest under a separate output grant.
  Source contains no preselected filename list or implicit external tool.
- [x] Verify independently computed outputs through source/STE VM/native and exact
  STE export. Document grants, bounds, kernel/path/IO limits, thirteen categories,
  partial effects and remaining Python-replacement scope; update linked references.

## Task6: Independent review and final gates

**Owner:** root coordinates reciprocal reviews; authors never approve own scope.

- [x] Review separate baseline diffs for frontend, shared engine/native, root and
  oracle; resolve findings with scoped RED/GREEN and independent rechecks.
- [x] Freeze reviewed inputs; fresh build, serial all-target Rust suite with
  existing local IPC test permissions, strict Clippy, fmt/diff; all18 focused
  oracles, frozen16/16 benchmark and broad differential. Check final hashes,
  documentation links and exact STE export. Mark only this milestone complete.

## Evidence and decisions

Previous goal turn classified as progress, proven by the process milestone's
completed ledger and unchanged105-file implementation/29-file documentation
manifests. Baseline snapshot: /tmp/ail-directory-baseline, with manifest.json.
Read-only preflight reports: /tmp/ail-directory-{frontend,native,oracle}-preflight.md.
Their recommendations were reconciled in the binding spec; listing includes all
names and PATH permits per-entry recovery. Existing fixed-file symlink semantics
remain unchanged. The user already authorized continued language development;
this architectural design/implementation proceeds within that active objective.
No new publication, external communication, deployment or destructive workspace
operation is part of the task.

Plan self-review and interface check:

| Tasks | Shared contract | Result |
| --- | --- | --- |
| 1/2 | AST ListNames/ReadText/WriteText, DIRECTORY_LIST_NAMES, directory roles | Exact spec names override early preflight alternatives. |
| 1/3 | DirectoryRequest80/Result16, status0..5, nullable slices, validate/run | Fixed; packed names exclude separators, descriptors store lengths. |
| 2/3 | native selection, legacy_directory_initialize, PATH index12 | Root owns public recovery; native worker owns native dispatch. |
| 4/5 | discovered JSONL files, sorted numeric output names and manifest | Root supplies source; oracle computes outcomes independently. |
| 1 | Typed Core/adapter tests versus one-module ownership | Consistent; root and host interfaces explicit. |
| 2 | API/CLI compatibility and source/order coverage | Consistent; no public old struct/signature changes. |
| 3 | ABI/host/native tests and file ownership | Consistent; no root-file edits. |
| 4 | Independent outcomes and adversarial bounds | Consistent; new tests/oracle only. |
| 5 | Workload and documentation | Consistent; no new host operations beyond the three specified. |
| 6 | Reciprocal reviews and final gates | Consistent; existing17 plus new1 focused oracles. |

Ruling: reuse existing available agents with separate briefs and reciprocal
review scopes; do not create additional agents. Continue in the authorized shared
workspace and retain the previous worktree. Baseline manifests substitute for a
new git commit/worktree because .git is read-only and unrelated edits must survive.
Ruling: relative-path grammar/4,095-byte overflow is PATH; Text transport overflow
and result/count/arena limits remain fatal. The spec now names that exception
explicitly, resolving the early preflight's proposed generic fatal size family.

Task2 implementation evidence: source/protocol RED at
/tmp/ail-directory-protocol-red.log; CLI RED at /tmp/ail-directory-root-red.log.
The unused-root quiet test was already green and is compatibility evidence.
All six root production files are captured in /tmp/ail-directory-root.diff
relative to the unchanged process baseline. Focused integration is 8/8 green
(new directory protocol/CLI plus existing process and stream protocol) in
/tmp/ail-directory-root-focused.log. Recovery exact-mask tests are 2/2 in
/tmp/ail-directory-integration.log (that command's filter selected no public
directory integration tests; these run separately).

Task5 first execution: source VM and source native produce byte-identical three
reports and manifest, with batch.count=5/written=3/failed=2. Logs/artifacts:
/tmp/ail-directory-demo-{vm,native}, /tmp/ail-directory-demo-{vm,native}.log,
/tmp/ail-directory-demo-build.log. Exact compiler STE output is published at
examples/ste/directory-report.ail-ste. Independent twelve-route workload
expectations and reciprocal reviews are still pending at this checkpoint.

Errno clarification: unsupported syscall ENOSYS maps to IO; denied EPERM/EACCES
map to PATH even if a syscall filter returned them. There is no portable way to
identify that origin from errno. Spec/docs now state the exact-map priority;
no engine behavior changed for this wording correction.

Task1 author evidence: /tmp/ail-directory-task-1-report.md; original16 focused
checks grew to17 with provenance regressions. Independent public acceptance
found ambiguous Provenance IDs attached to participating Action/Field/Thing
nodes, beyond operation provenance. Narrow correction validates every relevant
provenance target and edge metadata; RED /tmp/ail-directory-provenance-red.log,
GREEN /tmp/ail-directory-provenance-green.log (17 module,8 public).

Root review additionally reproduced a malformed reserved `writes text to
directory` form being accepted in descriptive function bodies. Recognition now
covers the absent-source `to directory` prefix; RED and GREEN:
/tmp/ail-directory-malformed-{red,green}.log. The root protocol target now has
three passing tests. Native review found resource-only trace wording differed;
Task3 author added a RED assertion and aligned native read/write verbs with VM.

Task3 author evidence: /tmp/ail-directory-task-3-report.md; shared host9/native7/
embedded-runtime9/process-compatibility1 focused tests, strict lib Clippy and
owned formatting pass. All process implementation and ABI inputs remain intact.
Independent reviews and final source hashes follow these corrections.

Task4 complete independent oracle: seed0xd1ec701,434 assertions/505 bounded
commands, /tmp/ail-directory-oracle-green.log. Actual4,095-byte relative paths,
all listing/count/raw/encoding and arena limits, mixed/called action budgets,
root replacement, cleanup, partial effects and twelve-route seeded+documented
workload outputs pass. The missing-final-write-target and JSON-format-overhead
harness expectations were corrected before the final run. Root read-only review
of the complete oracle: /tmp/ail-directory-review-oracle.md, no remaining findings.
Task5 independent outputs match;213 local Markdown links resolve and the checked
STE export exactly matches current compiler output. Frozen benchmark lock remains
ba0bd203c1a69da4a5ce29593a6a18884b56c89a1c00ef8294e762ca616e1adf.

Task3 independent review approved the final six hashes including trace correction:
/tmp/ail-directory-review-engine-native.md. Independent probes cover called-action
recovery/restored output selection/prior writes, both classes of relative symlink,
root symlink and hard-link semantics,68 grant cases in both native emitters, plus
constrained300-recovery execution. No remaining engine/native review findings.

Task1/2 independent approval: /tmp/ail-directory-review-root-frontend.md, with
final source hashes and3/3 protocol tests plus independent ordering/forgery/
authority/called-recovery/validation-priority probes. All review findings are
resolved. Implementation frozen at112 files in
/tmp/ail-directory-reviewed-inputs.json. The final gate runner is
/tmp/ail-directory-final-gates.py; authorized local IPC/localhost test access is
used for existing fixtures. All18 focused oracles, frozen benchmark and broad
VM/native comparison follow the serial all-target Rust suite. Documentation/
example review proceeds independently while the frozen implementation is tested.

First gate stopped at formatting before any broad semantic tests: the independent
public test's single format! invocation needed Rust2024 line wrapping. Corrected
only that layout and refreshed its frozen hash. Documentation review also found
and corrected the old fixed-file guide's statement that directory APIs were still
future work; it now links the implemented directory reference.

Documentation review confirms exact sample arithmetic, report inventory and
manifest bytes. Revised the shared-context wording to nested/repeated action
calls, avoiding an implication of native recursive-action expansion. Existing
native action-depth/expansion limits remain unchanged. Final fmt/diff/build and
strict all-target Clippy are green; serial tests are running on the frozen inputs.

Final documentation review approved: /tmp/ail-directory-docs-independent-review.md.
Three wording findings corrected: implemented directory support in the fixed-file
guide; nested/repeated action context wording; explicit recoverable relative-path
length exception in the recovery guide. The reviewed example arithmetic/manifest
matches real VM/native output. Final local-link/export check:
/tmp/ail-directory-doc-checks.json,42 artifacts/224 links/no missing/exact STE.

Final serial Rust suite passed1,303 tests across37 targets, zero failures/ignored;
/tmp/ail-directory-final-test-counts.json and /tmp/ail-directory-final-tests.log.
All112 reviewed implementation hashes and18 independent documentation review
hashes were rechecked unchanged. Feature oracles/frozen benchmark/broad comparison
are the only remaining final gate at this checkpoint.

Directory milestone complete. All six final gates passed: formatting, diff check,
build, strict all-target Clippy, the 1,303-test serial Rust suite across 37 targets,
and the complete regression runner. All 18 focused differential oracles passed;
the directory oracle independently checked 434 outcomes across 505 bounded
commands. The frozen benchmark scored 16/16. The broad VM/native comparison
reported 33 agreements, 30 cases not natively compilable, and zero divergences.
Evidence: /tmp/ail-directory-final-gates.json,
/tmp/ail-directory-final-runners.json, /tmp/ail-directory-final-test-counts.json,
and the logs referenced by those manifests. Final implementation hashes are in
/tmp/ail-directory-reviewed-inputs.json; final documentation/example hashes are
in /tmp/ail-directory-reviewed-docs.json. The independent documentation review
hashes remain in /tmp/ail-directory-docs-reviewed-hashes.json. The benchmark lock
is unchanged. Independent reviews have no unresolved findings.

This completes caller-bound directory resources and the working directory-report
example. The broader Specs → STE → Binary Python-replacement and AI-agent goal
remains active; this milestone does not establish general Python compatibility.
