# Standard streams

AIL actions can consume stdin and write exact application bytes to stdout and
stderr through source, checked Core, STE, VM and standalone Linux x86-64 ELF.

```text
- the system reads text from standard input into data.body
- the system writes text data.body to standard output
- the system writes text data.warning to standard error
```

Each operand is a declared global `Text` field. These operations execute in source
order, including inside loops, recovery scopes and called/repeated actions.
Application and C interop actions support them. Functions can prepare Text values;
stream effects belong in actions. Compiler-pass bodies and other profiles reject
the operations. Imports qualify field names and retain the same standard streams.

## Running a program

The exact bare `--stdio` flag grants all three streams and suppresses automatic
successful status, field and trace reports:

```bash
cargo run --quiet -- ail-run examples/json-lines-filter --action Filter --stdio \
  < examples/json-lines-filter/input.jsonl > /tmp/accepted.jsonl 2> /tmp/summary.json

cargo run --quiet -- ail-compile examples/json-lines-filter --action Filter \
  --target linux-x86_64-elf --out /tmp/json-lines-filter
/tmp/json-lines-filter --stdio < examples/json-lines-filter/input.jsonl
```

On Unix hosts, `ail-run` and `ail-vm` accept the flag at execution time; emitted
Linux x86-64 ELF accepts it as well. Other CLI hosts currently reject the mode.
Saved Core,
bytecode and direct STE routes use the same mode. Compiling a program never embeds
a stream grant. Duplicate flags are harmless; malformed `--stdio` variants and
use on unsupported CLI commands fail with `AIL-RUNTIME-CAPABILITY`. Named file
bindings can accompany the flag. State values, package metadata, file grants and
paths cannot authorize standard streams.

Without the flag, the existing CLI/native reports remain and stream operations
are denied. With it, even programs without stream operations omit automatic
output. Selected return values are still validated. Named rule failures remain
nonzero and report their failure name on stderr. Parse/check/runtime errors go to
stderr; an invalid artifact leaves stdout clean. Explicit output already written
by an application remains visible after failure.

## Borrowed VM endpoints

Library callers grant each endpoint independently using `AilStandardStreams`:

```rust
let mut streams = AilStandardStreams {
    stdin: Some(&mut input),
    stdout: Some(&mut output),
    stderr: None,
};
let result = run_ail_bytecode_action_with_io(
    &program, "Filter", state, &files, &mut streams,
)?;
```

The optional handles borrow `dyn Read` / `dyn Write`; adapters need no `Clone`,
`Send` or `Sync` implementation. `Default` grants nothing. The earlier run APIs
also grant no streams. Bytecode and all named file bindings are validated before
stream access. The same handles pass through action calls and repeats; there is
no global stdout redirection or whole-execution output buffer. The injected
library API is host-independent. `AilRunResult`
retains its state and trace metadata regardless of CLI reporting mode.

## Exact bytes and limits

A read consumes from the current cursor through EOF, without reopening, seeking
or closing it. It can block until the producer closes its write end. Positive
short reads continue; interrupted operations retry. Each read consumes at most
**1,048,577 bytes**: the extra byte detects the **1,048,576-byte Text limit** and
fails immediately, even if the producer stays open. An exactly full Text requires
EOF confirmation. Repeated reads after EOF return empty Text.

After complete bounded input, the runtime checks strict UTF-8, reserves
`max(bytes, 1)` from the shared **67,108,864-byte Text arena**, then updates the
destination. Failure leaves the destination and allocation charge unchanged;
consumed input remains consumed. Prior aliases retain their original values.
Empty successful reads charge one byte. Temporary native input buffers are
reclaimed on normal return and recovery.

Writes check endpoint permission first, then validate the complete bounded Text
before the first byte. They preserve Unicode, NUL, CR/LF and exact bytes without
adding a newline. Positive short writes continue and interruptions retry. The VM
flushes the adapter after each nonempty write; native direct write-all completion
has the same visibility and uses no `fsync`. Empty writes validate permission and
source, then perform no IO or flush. Writes charge no Text/List arena.

Other read/write errors, zero progress during a nonempty write, and flush errors
are fatal IO. Inherited nonblocking descriptors retain their flags; `WouldBlock`
is an IO error. Native `--stdio` ignores SIGPIPE so a broken output pipe can return
an IO error. Fatal diagnostic delivery is best effort: a broken stderr guarantees
nonzero termination, not a diagnostic message.


Host startup matters when a descriptor is already closed before launch. Rust's
[Unix runtime](https://doc.rust-lang.org/src/std/sys/pal/unix/mod.rs.html) replaces
closed standard descriptors with `/dev/null` before CLI main runs. The CLI uses
those resulting endpoints (EOF/discard); standalone ELF has no Rust startup and
reports IO on a nonempty operation against a closed descriptor. Borrowed VM
adapters expose their actual errors. Valid broken pipes still fail in both
backends. AIL operations themselves never reopen or close their endpoints.

| Diagnostic | Meaning |
| --- | --- |
| `AIL-STDIO-001` | Invalid source, Core, profile or bytecode stream operation |
| `AIL-RUNTIME-CAPABILITY` | Missing endpoint grant or invalid mode option |
| `AIL-RUNTIME-IO` | Host IO/flush or native setup/scratch allocation failure |
| `AIL-RUNTIME-TEXT` | Missing Text or invalid UTF-8; catchable data error |
| `AIL-RUNTIME-TEXT-LIMIT` | Per-value or shared Text arena limit; fatal |

Earlier output, partial writes and consumed input cannot be rolled back. Recovery
keeps prior state/file effects and allocation charges. Writes promise neither
durable storage nor atomicity against concurrent writers. This is bounded EOF
input with immediate output, not interactive or unbounded input streaming.

## Core and STE

`StandardStreamOperation` has no type annotation and exactly one `stream_kind`
attribute: `stdin_read`, `stdout_write` or `stderr_write`. One `stream_step` from
an executable Action owns it, with required numeric `step_index` and optional
numeric `operation_index`. A read has one attribute-free `stream_destination`;
a write has one attribute-free `stream_source`. Each operand targets a genuine
global Text declaration. Extra attributes/edges, duplicate or foreign ownership,
function-local substitutes and unsupported profiles are errors. Semantic access
checks derive reads/writes from these operands to protect secret fields.

| Opcode | Exact operands | STE template |
| --- | --- | --- |
| `STDIN_READ_TEXT` | `key` | `Read text from standard input into field {key}.` |
| `STDOUT_WRITE_TEXT` | `source` | `Write text from field {source} to standard output.` |
| `STDERR_WRITE_TEXT` | `source` | `Write text from field {source} to standard error.` |

Placeholders resolve through technical-data declarations. Both function IR kinds
reject these effects before execution. No new recovery category is introduced.

The [JSON-lines filter](../../examples/json-lines-filter) emits accepted records
on stdout and an explicit summary on stderr. Its
[STE export](../../examples/ste/json-lines-filter.ail-ste) runs directly. After
`cargo build`, `python3 scripts/ail_stdio_differential.py` checks independent byte
expectations across all twelve source/Core/rendered-source, direct/STE, VM/native
routes and a real two-process AIL pipeline. See the
[implementation record](../superpowers/plans/2026-10-11-standard-streams.md).
