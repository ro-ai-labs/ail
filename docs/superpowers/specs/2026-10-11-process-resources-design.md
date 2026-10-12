# Caller-bound process execution

## Intent and evidence

The active user goal is a Specs → ASD-STE100-based intermediate → binary language
that can replace Python and be used by an AI agent. The standard-stream milestone
is verified: 1,215 Rust tests, 16 focused differential oracles, the frozen 16/16
benchmark, and 33 supported VM/native agreements with zero divergences. Its 97
reviewed inputs remain unchanged. This next architectural increment enables AIL
to invoke real tools and other AIL executables, handle their failures, and compose
their outputs. Existing AgentTool descriptions do not execute arbitrary processes.

Alternatives considered: shell command strings make quoting and authority unclear;
VM-only std::process calls leave standalone binaries unable to perform the same
work; duplicating a large syscall state machine in assembly invites divergence.
Use explicit resource bindings, typed arguments, and a shared no_std Linux host
engine linked into the existing embedded runtime image. Independent external
helpers and Python expectations must test actual behavior, not just backend parity.

The ongoing user instruction authorizes this implementation. Preserve the shared
worktree and read-only .git; no commit, worktree, reset or publication. The
implementation plan and /tmp/ail-process-baseline snapshots record review state.
Skill approval gates must not repeat authorization for this continuing goal.

## Language surface

Application and C interop actions support the exact source form:

```text
- the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status
```

The six dynamic operands are genuine global fields: arguments List<Text>, input
Text, timeout Int, output/error Text, and status Int. All three destinations are
distinct. Input may alias a Text destination; timeout may alias status. Read all
inputs before publishing any output. Ordered effects work in branches, loops,
recovery scopes and called/repeated actions. Functions and CompilerPass bodies
reject recognized operations, including malformed variants. Imports prefix both
the resource and fields. No automatic descriptive write edges are synthesized.

Public AST AilProcessOperation::Run { resource, arguments, input, timeout_ms,
stdout, stderr, status } and Invalid(String); AilAction.process_operations;
AilActionStep::Process(index). Module src/process_operations.rs owns parsing,
Core validation, namespace, rendering, protocol, access derivation and VM adapter.

Core ProcessOperation has no type annotation and exactly process_kind=run and
resource attributes. One process_step edge from an executable Action owns it,
with required numeric step_index and optional numeric operation_index only.
Exactly one attribute-free edge of each kind process_arguments, process_input,
process_timeout, process_stdout, process_stderr, process_status supplies operands.
Reject extra attributes/edges, duplicate/foreign/orphan ownership, hidden operands
and local field forgeries; retain normal provenance. Derive three semantic reads
and three writes independently of descriptive edges, including secret checks.

Opcode PROCESS_RUN has exactly resource,arguments,input,timeout_ms,stdout,stderr,
status operands. STE template:

```text
Run process {resource} with arguments {arguments}, input {input}, limit {timeout_ms} milliseconds, output {stdout}, errors {stderr}, and status {status}.
```

Every placeholder resolves through technical data. Source/Core/protocol errors
use AIL-PROCESS-001 (bytecode wrapper AILBC021). Both function IRs and unsupported
profiles reject process effects before either backend executes.

## Authority and API

AilProcessResources has public executables: BTreeMap<String, PathBuf> and defaults
to no grants. Resource names use the existing bounded qualified ASCII identifier
grammar (256 bytes maximum). Each executable path must be absolute, nonempty,
at most 4,095 encoded bytes, and contain no NUL. Every binding, including unused
ones, is validated before any program effect; validation does not open the path.
The public parser is parse_ail_process_binding(resource=path).

An additive run_ail_bytecode_action_with_resources(program, action_name,
runtime_state, files, streams, processes) API preserves existing signatures.
The parameter types match with_io plus &AilProcessResources. Earlier entry points
delegate with no process grants. All bytecode, file bindings and process bindings
are validated first. The same immutable process context passes through calls.

ail-run, ail-vm and emitted ELF accept --run-process=name=/absolute/executable.
Duplicate valid bindings use the last path; all occurrences must be valid.
Malformed --run-process-prefixed tokens and use on other CLI commands fail with
CAPABILITY before effects. A runtime state value, package metadata, file grant
or --stdio cannot grant process execution. Compilation never embeds a grant.
The new effect requires Linux x86-64 and close_range (Linux 5.9+); unsupported
hosts reject process execution explicitly. Existing unrelated execution remains.

The grant authorizes a host executable with the program's argument strings. It
is not confinement of that executable: normal UID, cwd, filesystem, network,
umask and other OS limits apply. A caller may explicitly bind a shell, but AIL
does not invoke one implicitly. No PATH search or ENOEXEC shell fallback occurs.
argv[0] is the bound executable path. Child environment is empty. Child fd0/1/2
are private captured streams; no other inherited descriptor survives exec.

## Validation, results and recovery

Check the operation's resource grant first. Fully parse its List<Text> arguments
using existing borrowed list rules (LIST/LIST-LIMIT), then enforce at most 256
arguments and at most 65,536 decoded bytes including argv[0] and all terminating
NULs (fatal PROCESS-LIMIT). Reject embedded argument NUL with catchable PROCESS.
Read bounded input Text next, then parse timeout with existing integer rules
(INT); timeout must be 1 through 60,000 milliseconds inclusive (PROCESS otherwise).
Only after complete validation may child creation occur.

Input is exact UTF-8 bytes including NUL. Feed it concurrently with capturing
stdout and stderr, without newline conversion. Close input after all bytes; an
early child input close discards the unsent suffix and still yields output/status.
Each capture is bounded to 1,048,576 bytes, with at most one sentinel byte consumed
to detect overflow promptly. Keep the streams separate; no cross-stream ordering
is promised. EOF on both captures and observed direct-child exit complete a call.
Exit status is 0..255 normally or the negative terminating signal number. A
nonzero status is a normal result, including an actual exit 127.

On success validate stdout UTF-8, then stderr UTF-8 (catchable TEXT). Reserve the
combined max(stdout_bytes,1)+max(stderr_bytes,1) in the existing shared 67,108,864
Text arena, then atomically commit both immutable Texts and status. No List arena
charge is incurred. All three destinations and the allocation counter stay
unchanged on failure. Prior aliases survive later calls and recovery.

Append PROCESS and TIMEOUT to the existing recovery category table, preserving
the ten old indices. PROCESS covers invalid invocation data and child setup/exec
failure; TIMEOUT covers deadline expiration. These are catchable separately or
through any, which now means twelve categories. PROCESS-LIMIT, TEXT-LIMIT,
CAPABILITY and host IO remain fatal. A failing operation cleans its resources
before entering a handler; executed child effects and earlier AIL effects cannot
be rolled back. Traces name the resource only, not paths, input, arguments or output.

## Linux host engine and native boundary

Use src/process_host.rs, included in the VM on Linux x86-64 and the no_std embedded
runtime. No allocator, libc, threads, compiler invocation or temporary file is
required by a generated executable. The caller owns already validated/marshaled
argument and capture buffers. The engine owns all its descriptors and child.

The shared repr(C) ABI is fixed:

```text
ProcessRequest (64 bytes): executable C-string pointer @0; argv pointer @8;
input pointer @16; input_len u64 @24; timeout_ms u64 @32;
stdout pointer @40; stderr pointer @48; capacity u64 @56 (1,048,577).
ProcessResult (24 bytes): stdout_len u64 @0; stderr_len u64 @8; status i64 @16.
unsafe extern C ail_process_run(*const ProcessRequest, *mut ProcessResult) -> u64
0 success; 1 PROCESS; 2 TIMEOUT; 3 IO; 4 TEXT-LIMIT.
```

The engine leaves ProcessResult untouched on error. Native marshaling uses tracked
scratch; validate both outputs and combined charge before allocating persistent
results or committing slots. The host engine returns before recovery can unwind,
so it must release descriptors and reap its direct child on every owned-child
return path. The embedding adds the export to the existing validated runtime ELF.

Use a blocking child stdin socketpair and parent send with MSG_NOSIGNAL and
MSG_DONTWAIT, avoiding parent-global signal changes. Use pipes for both captures
and a CLOEXEC error channel to distinguish exec failure from exit 127. Normalize
owned descriptors above 3; child duplicates streams to 0/1/2 and error channel to
3 with CLOEXEC, then closes 4..UINT_MAX with close_range. Parent endpoints are
nonblocking; child endpoints are blocking. Retry EINTR, handle short transfers,
and service all endpoints fairly so hot output cannot starve input or deadlines.

Block signals around fork. The child creates its own process group, restores
default signal dispositions and an empty signal mask before exec. The parent
restores its exact original mask promptly. Reject inherited SIGCHLD=SIG_IGN or
SA_NOCLDWAIT before spawning (IO). Library callers must not reap children owned
by an active call through an external handler/thread. The engine does not modify
the parent's global signal dispositions or unrelated descriptors.

A monotonic deadline covers process creation, feeding, capturing, child exit and
capture EOF. Observe exit using waitid WNOWAIT to retain the leader PID until
cleanup; descendants retaining pipe ends keep the call open until EOF/deadline.
Before successful return or any post-spawn error, close owned endpoints, kill
remaining members of the private process group, and reap the direct child. On
ECHILD from an interfering reaper, report IO without signaling a recycled PID.
Unexpected signal-mask/setup/pipe/poll/read/write failures are IO, while the
child's setup/exec channel maps to PROCESS. Cleanup must precede recovery.

This is synchronous bounded process communication, not a background-job or
interactive process API. Descendants that deliberately escape the process group
are outside the containment guarantee. Deadline expiry starts kill/reap cleanup;
uninterruptible kernel operations can delay cleanup beyond the requested time.
Actual local socket operations may require the test harness's sandbox escalation;
that does not change Linux runtime semantics.

Primary references: [execve](https://man7.org/linux/man-pages/man2/execve.2.html),
[fork](https://man7.org/linux/man-pages/man2/fork.2.html),
[wait](https://man7.org/linux/man-pages/man2/waitpid.2.html),
[close_range](https://www.man7.org/linux/man-pages/man2/close_range.2.html),
[socket signals](https://www.man7.org/linux/man-pages/man7/socket.7.html), and
[Python subprocess](https://docs.python.org/3/library/subprocess.html).

## Proof of usefulness and verification

Add an AIL tool coordinator that reads JSON tasks from stdin, prepares typed
arguments/input/timeouts, invokes a caller-bound worker, and emits JSON results.
Use another standalone AIL program as the worker so production execution has no
Python dependency. Include nonzero results and recovered failures; preserve a
source and exported STE example and reproducible VM/native commands.

Independent tests cover twelve source/Core/rendered-source, direct/STE, VM/native
routes; exact Unicode/NUL/newline transport; literal shell metacharacters; empty
arguments/environment; child descriptors/signals; output/input larger than pipe
capacity; early stdin close; timeout while draining descendants; no zombie/fd/
scratch leaks; 127 versus launch failure; signals; exact capture/argv/arena limits;
atomic destinations; aliases; recovery; malformed Core/bytecode and authority.
External helper programs supply observations and Python computes expectations.
Final gate includes serial all-target Rust tests, strict Clippy, formatting,
diff checks, the new and prior sixteen oracles, unchanged frozen benchmark and
broad VM/native differential. This milestone cannot prove full Python replacement.
