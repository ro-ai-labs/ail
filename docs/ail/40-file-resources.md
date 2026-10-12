# Named text file resources

AIL actions can read and write UTF-8 files through the same checked
Specs → Core → STE → VM/native path as arithmetic, text and lists.

```text
- the system reads text from file resource input into data.body
- the system writes text data.body to file resource output
```

Both fields must be declared global `Text` fields. Reads replace their destination
only after the complete file passes validation. Writes replace the file contents
with the source's exact UTF-8 bytes, without an extra newline. Empty text and NUL
characters are supported. Operations execute in source order, including inside
branches, loops and called actions. The read result is immutable: a later read
into the same field does not change an earlier copy.

## Caller bindings

A program names a resource. The caller supplies its path and permission:

```bash
cargo run -- ail-run examples/text-files --action Process \
  --read-file=input=examples/text-files/input.txt \
  --write-file=output=/tmp/ail-processed.txt

cargo run -- ail-compile examples/text-files --action Process \
  --target linux-x86_64-elf --out /tmp/ail-text-files
/tmp/ail-text-files \
  --read-file=input=examples/text-files/input.txt \
  --write-file=output=/tmp/ail-processed-native.txt
```

Each flag is one argument; quote the entire argument when the path contains
spaces. Paths can contain `=`. A repeated binding uses the last path. A read
grant does not grant writes, and a write grant does not grant reads. The same
resource name can have separate bindings for each permission. A field assignment
such as `input=/tmp/data` cannot grant file access. Unused bindings do not open or
create files. Grants are shared through action calls.

Resource names contain ASCII identifier segments separated by dots, for example
`input`, `report_output`, or `library.input`, up to 256 bytes. Each segment begins
with a letter or underscore and continues with letters, digits or underscores.
Imports qualify resource names with their alias, so an imported `util.Read` action
uses a binding such as `--read-file=util.input=/tmp/input.txt`.

These are exact caller-selected paths, resolved when the operation runs. Relative
paths use the process's current directory, and symlinks follow normal host
semantics. A program cannot construct or replace a bound path. Bindings are not
directory confinement or an operating-system sandbox for arbitrary binaries.
Package `capability-grants` metadata continues to govern imported descriptive
effects; it does not itself authorize these filesystem operations.

## Runtime API

`AilFileResources` has public `reads` and `writes` maps from resource names to
`PathBuf`. Every binding, including unused ones, is validated before execution;
invalid names, empty paths and NUL-containing paths fail with
`AIL-RUNTIME-CAPABILITY`. `parse_ail_file_binding` and `valid_ail_file_resource`
provide the CLI binding parser and name validator. Pass the resource maps to `run_ail_bytecode_action_with_files`:

```rust
let mut files = AilFileResources::default();
files.reads.insert("input".into(), input_path);
files.writes.insert("output".into(), output_path);
let result = run_ail_bytecode_action_with_files(
    &bytecode, "Process", state, &files,
)?;
```

The existing `run_ail_bytecode_action` and document execution API default to no
file grants. `ail-run` and `ail-vm` accept the flags above, including saved Core,
bytecode and direct STE inputs supported by those commands.

## File behavior and limits

Only regular files are accepted. Reads open at the start on every execution and
consume through EOF. Writes create a missing file with host permissions (mode
0666 subject to umask), or truncate an existing regular file after validating the
source value and file type. Parent directories are not created. Directories,
FIFOs, sockets and devices fail. The Linux implementation opens with `O_NONBLOCK`
to allow FIFO rejection before a blocking open, and uses `O_NOCTTY` to avoid
acquiring a terminal. These flags follow the Linux [open semantics](https://man7.org/linux/man-pages/man2/open.2.html).

Files and write operands are limited to **1,048,576 UTF-8 bytes**. A successful
read charges `max(bytes, 1)` to the root execution's shared **64 MiB text result
budget**, also used by text joins, slices and text list extraction. Read scratch
storage is bounded and reclaimed. Writes do not charge this allocation budget.
UTF-8 is strict; invalid byte sequences are errors.

Permissions are checked first. A write then validates its source before opening
or truncating the file. A read checks its byte limit, UTF-8 validity, then remaining
allocation budget before updating the field. File-operation traces name the operation and
resource, without file paths or contents.

| Error | Meaning |
| --- | --- |
| `AIL-FILE-001` | Invalid source/Core/bytecode file operation |
| `AIL-RUNTIME-CAPABILITY` | Missing read or write resource grant |
| `AIL-RUNTIME-IO` | Host open/type/read/truncate/write or native scratch allocation failure |
| `AIL-RUNTIME-TEXT` | Missing text field or invalid UTF-8 |
| `AIL-RUNTIME-TEXT-LIMIT` | Text byte limit or shared allocation budget exceeded |

Writes take effect immediately. A later failure does not undo a completed write;
a failed write can leave a truncated or partial file. Successful writes do not
promise durable storage. Close errors are ignored in both runtimes; the Linux
implementation does not retry close after interruption, consistent with its
[file descriptor release behavior](https://man7.org/linux/man-pages/man2/close.2.html).

## Core and STE

A `FileOperation` node has exactly `file_kind=read|write` and `resource` attributes.
A `file_step` edge gives it exactly one executable Action owner and a source-order
index. A read has one `file_destination`; a write has one `file_source`. Each
operand references a declared global Text field. The checker rejects extra
attributes, conflicting operands, unsupported profiles and malformed ownership.
Semantic reads/writes derive from these operands and participate in secret-field
checks even if descriptive edges are absent.

The STE templates are:

```text
Read text from file resource resource_data into field destination_data.
Write text from field source_data to file resource resource_data.
```

The opcodes are `FILE_READ_TEXT` with exactly `resource,key` and `FILE_WRITE_TEXT`
with exactly `resource,source`. Resources and field names use STE technical data
declarations. File operations are available in Application and C interop actions;
compiler-pass bodies and integer functions reject them. Direct bytecode and STE
also reject file operations under unsupported profiles.

The [text-files example](../../examples/text-files) reads a document, adds a
heading, counts Unicode characters and calls a separate action to save the
result. Its [STE export](../../examples/ste/text-files.ail-ste) also runs directly.
After `cargo build`, run `python3 scripts/ail_file_differential.py` for independent
Python comparisons and file-permission/error probes. This increment passes
720 Rust tests, 200 independent Python file transforms and the unchanged 16/16
numeric capability ladder.

[Standard streams](52-standard-streams.md) add explicitly granted stdin/stdout/
stderr without changing these regular-file semantics. [Value functions](44-value-functions.md)
support typed reusable computations. [Directory resources](54-directory-resources.md)
provide discovery and computed relative file access. [Bytes](56-bytes.md) adds
binary values and fixed-file reads/writes. Broader Python compatibility remains
open work.

## Binary file forms

[Bytes](56-bytes.md#file-effects) adds `reads bytes from file resource` and
`writes bytes SOURCE to file resource` using the same grants and host APIs.
These operations require Bytes fields, preserve arbitrary octets, and charge
read results to the shared List/Json arena. Text operations retain strict UTF-8.
