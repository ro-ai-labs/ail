# Standard-stream execution design

## Intent and starting evidence

The user wants the complete Specs → STE → binary language to become useful in
place of Python and for AI agents. The preceding recovery milestone is verified
progress: 1,163 tests pass, the 89 reviewed source/test/toolchain/oracle hashes
still match, and its implementation plan is complete. The broader goal remains
active. Current executable AIL can process explicitly bound regular files but
cannot consume stdin or emit clean application stdout/stderr for a shell pipeline.

This architectural increment adds actual standard-stream effects and caller
control over runtime reporting, through source, Core, STE, VM and Linux x86-64
ELF. A real JSON-lines filter and independent Python expectations demonstrate
composable programs. It does not claim interactive/unbounded streaming, arbitrary
precision, foreign calls or full Python/ASD-STE100 conformance.

Alternatives considered: treating /dev/stdin as a named file would violate the
regular-file contract and reopen rather than preserve an inherited cursor;
automatically selecting stream mode from source operations would mix authority
with compilation and fail ordinary programs. Explicit runtime authority and
injected library handles fit the existing file-resource boundary.

The continuing user instruction and developer autonomy authorize implementation
after this concrete design and plan. Preserve the shared worktree/read-only .git;
no commits, worktree creation, resets or publication. Baseline snapshots and the
plan ledger provide review evidence. Repeated skill approval gates would repeat
authorization already supplied for this ongoing goal.

## Source, Core and protocol

Application and C interop executable actions support:

```text
- the system reads text from standard input into data.body
- the system writes text data.body to standard output
- the system writes text data.warning to standard error
```

Operands are declared global Text fields. Operations run in source order inside
branches, loops, recovery scopes and called/repeated actions. Functions may
prepare Text values but cannot perform these effects. Descriptive functions,
compiler passes and other profiles reject recognized stream statements, including
malformed or punctuated variants. Existing unrelated syntax remains accepted.
The standard-stream parser precedes the file parser, whose broad write prefix
would otherwise claim valid stream writes.

Public AST: AilStandardStreamOperation::{Read{key},WriteOutput{source},
WriteError{source},Invalid(String)}; AilAction.standard_stream_operations;
AilActionStep::StandardStream(index). New module src/standard_streams.rs owns the
operation surface, validation and VM primitive execution.

Core node StandardStreamOperation has no type annotation and exactly stream_kind
= stdin_read|stdout_write|stderr_write. Exactly one ordered stream_step edge
owns it from an executable Action; the established numeric step_index and optional
operation_index are its only attributes. Reads have one attribute-free
stream_destination; writes have one attribute-free stream_source. Both target a
genuine global Text declaration, validated through float_operations::scoped_field.
Reject duplicate/foreign/orphan ownership, extra metadata, hidden edges and
function-local substitutes. Preserve legitimate provenance. Derived accesses
protect secret reads/writes even after descriptive access edges are removed.
Imports qualify fields only; standard stream identity is never namespaced.
Saved Core must retain duplicate structural edges so verification sees them.

| Opcode | Exact operands | STE template |
| --- | --- | --- |
| STDIN_READ_TEXT | key | Read text from standard input into field {key}. |
| STDOUT_WRITE_TEXT | source | Write text from field {source} to standard output. |
| STDERR_WRITE_TEXT | source | Write text from field {source} to standard error. |

Source/Core/protocol failures use AIL-STDIO-001. Require exact operand cardinality
and field syntax, permitted profile, and exclusion from INTEGER_FUNCTION and
VALUE_FUNCTION bodies before either backend executes. No new recovery categories.

## Authority and public VM API

```rust
#[derive(Default)]
pub struct AilStandardStreams<'a> {
    pub stdin: Option<&'a mut dyn std::io::Read>,
    pub stdout: Option<&'a mut dyn std::io::Write>,
    pub stderr: Option<&'a mut dyn std::io::Write>,
}

pub fn run_ail_bytecode_action_with_io(
    program: &AilBytecodeProgram,
    action_name: &str,
    runtime_state: BTreeMap<String, String>,
    files: &AilFileResources,
    streams: &mut AilStandardStreams<'_>,
) -> Result<AilRunResult, String>;
```

Each optional handle independently grants its endpoint. Default grants nothing;
the evaluator never opens ambient process streams. Existing run APIs delegate
with empty streams and remain source compatible. Validate bytecode and all file
bindings before touching a stream. Borrow the same context through action calls
and repeats, preserving cursor state and immediate output. Do not require Clone,
Debug, Send or Sync on adapters or buffer the entire execution's output.

Module runtime interface: execute(instruction, state, text_allocated, streams)
returns Result<(),String>, with mutable state/counter/context references. Its
parser/Core/namespace/instruction/verify/accesses interfaces follow file_operations
without named-resource metadata. Expose a normalized recognized_bullet guard for
root's legacy/compiler/function rejection paths.

## Runtime semantics

Authorize the specific endpoint first, before source lookup or consuming input.
Missing grants use fatal AIL-RUNTIME-CAPABILITY. A runtime field, package metadata,
file binding or filename cannot authorize standard streams.

Read from the current reader/descriptor position through EOF. Retry Interrupted,
continue positive short reads, and stop immediately after at most 1,048,577 bytes
have been consumed. Exactly 1,048,576 bytes require confirming EOF. Other errors,
including WouldBlock, are fatal IO. Check size before strict UTF-8; then reserve
max(byte_length,1) from the existing shared 67,108,864-byte Text result budget and
commit the destination. Invalid UTF-8 is catchable TEXT; size/arena limits are
fatal TEXT-LIMIT. Destination and failed-operation charge stay unchanged on
failure. Consumed bytes are not restored. Repeated reads after EOF return empty
Text, each charging one byte. Reads may block until EOF; there is no IO timeout.

Writes validate the entire existing bounded Text before the first output byte,
then complete positive short writes, retry Interrupted and flush the selected
adapter after every nonempty successful write. Zero progress during a nonempty
write, other write errors and flush errors are fatal IO. Empty writes validate
authority/source and perform no host IO or flush. Preserve UTF-8, NUL, CR/LF and
exact bytes with no added newline or decoration. Writes charge no result arena.
No durability guarantee or cross-process atomic-write guarantee is implied.

Missing/malformed Text is catchable TEXT; Text size limits remain fatal. Successful
and partial output, input consumption, file effects and earlier field writes
persist across later errors. Recovery neither closes streams nor refunds charges.
Temporary native read mappings join the existing recovery registry; persistent
owned read results use the Text arena and keep aliases immutable. The runtime
never closes/reopens/seeks fd0/1/2 or changes their inherited blocking flags.

Native --stdio setup ignores SIGPIPE so explicit writes to broken pipes return
fatal IO consistently with Rust's default handling. Fatal diagnostic delivery is
best effort: if stderr itself is unusable, guarantee nonzero exit without recursive
diagnostic attempts. These choices follow Linux short-I/O/error semantics
([read](https://man7.org/linux/man-pages/man2/read.2.html),
[write](https://man7.org/linux/man-pages/man2/write.2.html)) and
[Rust's SIGPIPE setup](https://doc.rust-lang.org/unstable-book/compiler-flags/on-broken-pipe.html).

## Runtime mode and reporting

The exact bare --stdio flag is accepted by ail-run, ail-vm and emitted ELF at
execution time. It grants stdin/stdout/stderr together and selects clean application
streams. Duplicate flags are idempotent. Malformed --stdio-prefixed variants fail
before effects with CAPABILITY; unsupported CLI commands reject the flag. Compile
time never embeds stream grants. Existing file bindings can be combined with it.

Without --stdio, existing CLI/native output stays unchanged and stream operations
have no grants. With it, suppress automatic status/state/trace printing, including
EMIT_TRACE console rendering, in source/Core/STE/bytecode routes and both native
emitters, even when no stream instruction exists. Retain VM result/trace metadata.
Still validate selected return fields. Only explicit writes reach successful
stdout/stderr. Earlier explicit bytes survive failure. Parse/check/verification
and fatal diagnostics go to stderr; invalid input artifacts leave stdout clean.
Named rule failures remain failed status/nonzero exit and report the failure name
on stderr without automatic field/trace reports. Backend diagnostic wording and
nonzero status numbers need not be byte-identical.

Scalar native root context's unused RBP+72 stores the mode/grant. Recovery and
function scopes retain it process-wide; ROOT_BYTES and handler layouts stay
unchanged. Scalar trace_value must guard both prefix and value, while fatal
diagnostics use an ungated path. Legacy native root startup may reserve an
80-byte context after capturing argv and use the same offset; its helpers preserve
RBP. Native worker exposes legacy_stdio_initialize(code)->Result<(),String> and
legacy_stdio_skip(code,target) through native_expr; root inserts these startup
and whole-output guards in ail.rs. Source operation admission forces scalar
execution, including through inlined actions and fuel scopes.

## Real program and verification

examples/json-lines-filter action Filter reads bounded stdin, splits LF, trims the
existing fixed Unicode whitespace set, skips blank rows and invokes a reusable
parse/divide function. Require unique decoded /value and /divisor integer members;
divide checked i64 toward zero. Recover JSON, JSON-PATH, DIVZERO and OVERFLOW per
row. For each accepted row write exactly {"value":<quotient>} followed by LF to
stdout in input order. At end explicitly write one LF-terminated JSON summary to
stderr with keys rows,accepted,rejected,errors; error keys ordered DIVZERO,JSON,
JSON-PATH,OVERFLOW with their AIL-RUNTIME- prefixes. Empty input produces only the
zero summary. Fatal input validation precedes every record; later fatal failures
retain the written prefix and omit the successful summary.

Independent tests cover all 12 public artifact/runtime routes, real multi-process
AIL pipelines, injected short/EINTR/zero/flush errors, large bidirectional pipes,
broken descriptors, endpoint-specific authority, malformed flags before effects,
exact byte/arena bounds, UTF-8 split across reads, cursor/recovery/alias semantics,
secret/ownership forgeries, imports and pure-function rejection. Child harnesses
feed stdin and drain both outputs concurrently with bounded timeouts. Compare
actual bytes against independent Python results, not CLI status text.

Review frontend/runtime, native, root CLI/legacy and oracle scopes independently.
Then run focused and all-target serial Rust tests, strict Clippy/fmt/diff/build,
new and prior differential runners, frozen benchmark and unchanged-input checks.
Preserve all existing frozen benchmark cases and SCORE.lock.


## Validated host-startup distinction

The independent oracle showed that preexec-close(0) before the Rust CLI yields
EOF, whereas direct ELF sees EBADF. Primary Rust Unix startup code confirms
sanitize_standard_fds opens /dev/null for closed fd0/1/2 before main. The language
operation consumes its actual supplied endpoint; it cannot reconstruct the prior
launch descriptor state. Therefore closed-before-start CLI probes assert host
normalization separately from direct-ELF IO failures. Injected VM adapter errors
and valid broken pipes retain the shared IO contract. No custom host entry point,
null-device rejection, or production endpoint reopening is introduced.
Source: https://doc.rust-lang.org/src/std/sys/pal/unix/mod.rs.html . This
implementation-discovered distinction is documented in chapter52 and the oracle.


## CLI input buffering correction

A shared-file cursor regression proved Rust StdinLock could prefetch 8,192 bytes
for the final one-byte sentinel request. Unix CLI stdin now uses a borrowed-fd
read adapter while retaining its stdin lock, making each read(2) request match
exactly the VM request without open/dup/seek/close. The portable injected VM API
continues to accept caller-owned Read adapters; their own buffering belongs to
the caller. CLI --stdio is supported on Unix hosts and explicitly rejected on
other hosts until an equally bounded platform adapter exists. Linux ELF remains
the standalone target. This bounds actual CLI descriptor consumption rather than
only bytes returned by a hidden buffer.

Terminal native startup diagnostics protect themselves from SIGPIPE even when a
malformed option prevents mode initialization. This error-only protection grants
no endpoints and always terminates. If protection itself fails, the executable
exits nonzero without attempting a potentially fatal diagnostic write. Normal
startup without --stdio retains its inherited signal disposition.
