# Named process resources

AIL actions can run a caller-bound executable, send Text to its stdin, capture
its stdout and stderr, and inspect its exit status. The same operation travels
through source, checked Core, STE, the VM and standalone Linux x86-64 ELF.

```text
- the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status
```

Declare `arguments` as List<Text>, `input`, `output` and `errors` as Text, and
`timeout` and `status` as Int. These are global action fields; functions can
prepare their values but cannot perform the effect. Application and C interop
actions support process calls in branches, loops, recovery scopes and called
actions. The three destinations must be distinct. Input may share a field with
a Text destination; timeout may share its field with status.

## Binding a tool

```bash
target/debug/ail ail-run examples/tool-coordinator --action Coordinate --stdio \
  --run-process=worker=/tmp/ail-coordinator-worker \
  < examples/tool-coordinator/input.jsonl
```

Compile the worker first using the [coordinator example](../../examples/tool-coordinator).
`ail-run`, `ail-vm` and emitted ELF accept the flag at execution time. The path
must be absolute, contain no NUL, and use at most 4,095 encoded bytes. A resource
name uses the [file-resource identifier grammar](40-file-resources.md), including
qualified imported names, with a 256-byte bound. Imports qualify resources and
field references. Each flag is one argument; paths may contain spaces or `=`.
All bindings are validated before effects, including unused and earlier duplicate
bindings. The last valid binding for a name wins. Unused paths are not opened.

The resource grants the selected executable with program-supplied arguments.
There is no implicit shell, PATH lookup, or shell fallback for a file that cannot
be executed. A script with a valid host shebang can execute normally. `argv[0]`
is the bound path. The child gets an empty environment, the inherited working
directory, and private fd0/1/2; no other inherited descriptor survives exec.
The new effect requires Linux x86-64 with `close_range` (Linux 5.9 or later).

The executable retains ordinary host permissions, including access to files and
networks. Binding a tool is not an OS sandbox for that tool or its descendants.
Package metadata, state fields, file bindings, and `--stdio` cannot grant process
access. Compiled artifacts contain resource names, never caller grants. Without
a process binding, execution fails before evaluating the operation's inputs.

## Communication and results

Arguments remain separate strings, including empty strings and literal shell
metacharacters. Their List<Text> representation must pass the existing list
validator. There may be at most 256 strings and 65,536 decoded bytes including
the executable path and all terminating NULs. Argument strings cannot contain
NUL; stdin and captured output can. Timeout is an integer from 1 through 60,000
milliseconds inclusive.

Input and each capture are bounded to 1,048,576 bytes. The runtime feeds input
while draining both outputs, so a child can write more than pipe capacity before
reading. If the child closes stdin early, unsent input is discarded and the call
still collects its output and status. Bytes preserve Unicode, NUL, CR and LF;
there is no newline conversion or automatic newline. Captures are separate and
do not imply a combined cross-stream ordering.

A call completes after both outputs reach EOF and the direct child exits. Status
is its normal exit code (0..255) or the negative signal number that terminated
it. Nonzero status is a normal result. In particular, a program exiting 127 is
distinct from a launch failure.

The runtime checks stdout UTF-8, then stderr UTF-8, and reserves their combined
`max(stdout_bytes, 1) + max(stderr_bytes, 1)` from the shared 67,108,864-byte Text
result budget. Only then does it publish both Text fields and status together.
Failed calls leave all three destinations and allocation charges unchanged.
Earlier copies of captured Text keep their values after another call.

## Failure handling and cleanup

```text
- try:
- the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status
- recover PROCESS or TIMEOUT into data.failure:
- the system writes text data.failure to standard error
- end try
```

| Diagnostic | Meaning |
| --- | --- |
| AIL-PROCESS-001 | Invalid source, Core, profile or process instruction |
| AIL-RUNTIME-CAPABILITY | Missing grant or malformed binding |
| AIL-RUNTIME-LIST / LIST-LIMIT | Existing argument-list validation errors |
| AIL-RUNTIME-PROCESS-LIMIT | Argument count or combined byte limit exceeded |
| AIL-RUNTIME-INT | Missing or malformed timeout integer |
| AIL-RUNTIME-PROCESS | NUL argument, timeout outside range, or child setup/exec failure |
| AIL-RUNTIME-TIMEOUT | Process communication deadline expired |
| AIL-RUNTIME-TEXT | Missing/invalid Text or invalid UTF-8 capture |
| AIL-RUNTIME-TEXT-LIMIT | Input/capture size or combined Text budget exceeded |
| AIL-RUNTIME-IO | Host descriptor, allocation, signal, clock or communication failure |

PROCESS and TIMEOUT extend [runtime recovery](51-runtime-recovery.md); `any` now
matches fifteen categories, including PATH from [directory resources](54-directory-resources.md), BYTES from [byte values](56-bytes.md) and VALUE from [executable variants](59-executable-variants.md). Capability, resource-limit and host IO failures remain
fatal. A handler runs only after owned descriptors are closed, remaining members
of the private child process group are killed, and the direct child is reaped.
The timeout includes communication after the leader exits while descendants hold
pipe ends. It uses a monotonic deadline. Kill/reap cleanup can take longer when
the kernel delays an uninterruptible operation. A descendant that deliberately
escapes its group is outside the containment guarantee.

Cleanup also runs on successful completion, ending remaining group members.
This API is for synchronous tools, not persistent background services. Child
effects and previously completed AIL effects remain; recovery does not undo them.
Trace messages identify the resource without disclosing its path, arguments,
input or output. Explicitly writing captured data remains the program's decision.

## Library API

```rust
let mut processes = AilProcessResources::default();
processes.executables.insert("worker".into(), executable_path);
let result = run_ail_bytecode_action_with_resources(
    &program, "Coordinate", state, &files, &mut streams, &processes,
)?;
```

`parse_ail_process_binding` parses the `resource=path` part of a CLI grant.
Earlier execution APIs remain available and grant no processes. Verification and
all file/process binding checks precede effects; calls share the same context.
The Linux engine preserves the parent's signal mask and dispositions. It rejects
SIGCHLD ignored or SA_NOCLDWAIT before creating a child. Embedding callers must
not reap an active call's child from another handler/thread. Child signal
dispositions reset to defaults, including SIGPIPE, and its mask is empty.

## Core and STE

`ProcessOperation` has exactly `process_kind=run` and `resource`, no type annotation,
and one ordered `process_step` owner from an executable Action. Operand edges are
`process_arguments`, `process_input`, `process_timeout`, `process_stdout`,
`process_stderr`, and `process_status`; all are unique and attribute-free. They
target genuine global fields of the required types. Extra metadata, hidden edges,
foreign/local ownership, unsupported profiles and aliased destinations fail.
Semantic access checks derive all three reads and writes from these operands.

`PROCESS_RUN` takes exactly `resource,arguments,input,timeout_ms,stdout,stderr,status`.
Its controlled-English template is:

```text
Run process {resource} with arguments {arguments}, input {input}, limit {timeout_ms} milliseconds, output {stdout}, errors {stderr}, and status {status}.
```

Placeholders refer to declared technical data. The
[implementation record](../superpowers/plans/2026-10-11-process-resources.md)
tracks verification. The new operation expands practical tool automation; it
does not establish complete Python library compatibility.
