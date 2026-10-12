# Caller-Bound Directory Resources

## Intent and current evidence

The active user goal remains Specs → ASD-STE100-based executable intermediate →
binary, followed by a language usable by AI agents as a Python replacement.
The previous turn made verified progress: process resources and a working AIL
coordinator passed 1,257 Rust tests, seventeen independent oracles and the frozen
16/16 benchmark. All 105 reviewed implementation hashes and 29 documentation
hashes match at this turn's start. Full Python replacement remains unproven.

Current file effects only operate on fixed caller paths. An AIL program cannot
enumerate a workspace or read/write a filename discovered at runtime. This
architectural increment supplies bounded directory enumeration and relative text
file operations, enabling real multi-file batch automation. It preserves existing
file/process/stream APIs and carries every operation through checked Core, STE,
VM and standalone Linux x86-64 ELF.

A shared no_std engine will implement path resolution, directory collection and
file transfer for both backends. This centralizes filesystem semantics; the
adapters retain grant authority and shared arena accounting. Separate native/VM
filesystem implementations would duplicate the most sensitive behavior. Caller-
bound process tools remain useful, but these operations belong in the language
runtime and do not require an external executable.

## Source and typed operation contract

Exact action forms:

```text
- the system lists names in directory resource workspace at data.path into data.names
- the system reads text from directory resource workspace at data.path into data.body
- the system writes text data.body to directory resource reports at data.path
```

`path` and `body` are declared global Text fields; `names` is List<Text>.
Application and C interop actions support the effects in ordered bodies,
branches, loops, called/repeated actions and recovery scopes. Functions and
compiler-pass bodies reject both valid and malformed reserved attempts. Imports
qualify resources and every operand. A read may overwrite its own Text path
field after successful completion. Writes may use the same path/source field.

AST `AilDirectoryOperation` has `ListNames { resource, path, key }`,
`ReadText { resource, path, key }`, `WriteText { resource, path, source }`, and
`Invalid(String)`. Actions store `directory_operations`; ordered steps use
`AilActionStep::Directory(index)`.

`DirectoryOperation` Core nodes have exactly `directory_kind=list_names|read_text|
write_text` and `resource`, with no type annotation. Exactly one executable
Action owns the node through an ordered `directory_step`: required step_index,
optional operation_index, integer values, no other metadata. Every operation has
one attribute-free `directory_path` edge. List/read have one
`directory_destination`; write has one `directory_source`. Operands target genuine
global fields of the required types with exactly one valid has_field declaration.

Reject missing/extra/duplicate/reversed/hidden edges, metadata, local fields,
competing runtime keys, unsupported profiles, and ambiguous node identities for
the operation, Action owner, operands, declaring Things and provenance targets.
Normal has_provenance edges must target unique Provenance nodes. Saved Core must
retain structural edge multiplicity. Semantic accesses independently derive path
reads, source reads and destination writes for secret checks.

Bytecode:

- DIRECTORY_LIST_NAMES: exactly resource,path,key.
- DIRECTORY_READ_TEXT: exactly resource,path,key.
- DIRECTORY_WRITE_TEXT: exactly resource,path,source.

Field operands must be exact field references. Entire-program verification applies,
including unreachable actions and function IRs. Source/Core errors use
AIL-DIRECTORY-001; bytecode uses verifier wrapper AILBC022.

STE templates (all within the existing twenty-word limit):

```text
List names in directory resource {resource} at field {path} into field {key}.
Read text from directory resource {resource} at field {path} into field {key}.
Write text from field {source} to directory resource {resource} at field {path}.
```

All placeholders are declared technical data. No grants are embedded in artifacts.

## Caller authority and public APIs

`AilDirectoryResources` derives Default/Clone/Debug and exposes two
BTreeMap<String,PathBuf> fields, `reads` and `writes`. Listing and reads require a
read-directory binding; writes require a write-directory binding. Neither grants
the other, and the same resource may map to different roots for each permission.

CLI execution flags:

```text
--read-directory=workspace=/absolute/root
--write-directory=reports=/absolute/output-root
```

A qualified ASCII resource name follows existing file-resource grammar, max256
bytes. Root paths must be absolute, nonempty, no NUL, max4,095 encoded bytes.
Every supplied binding validates before effects, including unused and earlier
duplicate flags; the last valid binding wins. Unused roots are not opened.
Malformed reserved prefixes and flags on unsupported commands fail CAPABILITY.
Grants occupy dedicated immutable native slots and never appear as unknown state
fields/default output. State, package metadata, file/process grants and stdio
access cannot supply directory authority.

Add an owned aggregate:

```rust
#[derive(Default, Clone, Debug)]
pub struct AilHostResources {
    pub files: AilFileResources,
    pub processes: AilProcessResources,
    pub directories: AilDirectoryResources,
}
pub fn run_ail_bytecode_action_with_host_resources(
    program: &AilBytecodeProgram,
    action_name: &str,
    runtime_state: BTreeMap<String, String>,
    resources: &AilHostResources,
    streams: &mut AilStandardStreams<'_>,
) -> Result<AilRunResult, String>;
```

All previous execution APIs retain exact signatures and default-deny directories.
A private borrowed resource context delegates without cloning existing maps and
travels unchanged through calls/repeats. Whole-bytecode verification precedes all
binding checks; file, process, then directory bindings validate before dispatch.
`parse_ail_directory_binding` parses resource=absolute-root. CLI uses the aggregate.

## Paths, resolution and host boundaries

At invocation: check the required grant before any field input. Validate path
Text presence, max1,048,576 bytes and UTF-8, then path grammar. Paths use at most
4,095 UTF-8 bytes; no NUL or leading slash; each slash-separated component must be
nonempty and differ from `.` and `..`. An empty path denotes the root only for list.
No normalization, URL decoding, backslash rewriting, case folding or tilde expansion
occurs. Hidden names, spaces, Unicode, newlines, backslashes and equals are literal.
Write source Text validates only after path validation, before any filesystem open.

Each operation opens the caller root with normal host path semantics, including
root symlinks. It pins that directory fd until the operation ends. Relative paths
resolve with openat2 RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS, never a fallback based on
string prefixes or canonicalization. These flags also prevent magic-link traversal.
All relative symlinks, including ones targeting inside the root, are rejected.
Roots are reopened on every invocation; their pathname is not pinned across the
entire AIL run. Nested relative directories are supported. Missing parent
directories are not created.

The new effect requires Linux x86-64 with openat2 (Linux5.6+). Unsupported hosts
and missing syscalls report IO. Denied syscalls follow the exact errno table below;
EPERM/EACCES report PATH even when a syscall filter caused the refusal. Existing
effects retain their platform behavior. Mounts beneath a root and hard-linked files follow ordinary host
semantics; this is path-resolution authority, not inode isolation or an OS sandbox.
An in-root hard link can alias an inode reachable elsewhere. An opened object may
be renamed concurrently; no snapshot or transaction is promised.

Read/write opens use O_NONBLOCK|O_NOCTTY|O_CLOEXEC, then inspect the opened fd and
allow data transfer only for regular files. No FIFO wait, terminal acquisition,
read/truncate/write of a nonregular target is allowed. Opening a device may still
invoke its host open callback before its type is rejected, matching fixed-file
semantics. Directory enumeration does not follow or inspect child entry types.
All descriptors close before return/recovery. Close errors are ignored without
retry, consistent with existing file effects.

## Listing, text transfer and atomic results

List returns every immediate basename except . and .., including hidden names,
directories, symlinks and special files. It is nonrecursive. Names retain exact
Unicode and are sorted ascending lexicographically by UTF-8 bytes (equivalently
Unicode scalar order), without normalization. d_type is not used. There is no
concurrent-mutation snapshot guarantee; observed duplicate entries are retained.

Collect at most65,536 names and at most1,048,576 raw packed bytes excluding separators (descriptors
carry lengths). Reject collection overflow as LIST-LIMIT before UTF-8
validation. Validate every collected name as UTF-8, sort, then encode canonical
List<Text> using existing AIL escapes (including C0, DEL and C1 controls). Encoded
result must be at most1,048,576 bytes. Charge its full length to the shared
67,108,864-byte List/Json arena; an empty list costs2 bytes. No Text charge occurs.

Read consumes through EOF with at most1,048,577 observed bytes, so size overflow
wins over UTF-8 validation. Limit is1,048,576 bytes; validate UTF-8, then charge
max(length,1) to the shared67,108,864-byte Text arena. Write source has the same
1MiB Text limit; create a missing regular file with mode0666 subject to umask or
truncate an existing regular file only after validating source and opened type.
Exact bytes include NUL and CR/LF with no newline conversion.

List/read update destination and shared charge together only after all validation
and allocation succeed. Failed operations leave both unchanged; earlier aliases
retain old values. Writes have no persistent arena charge. Host writes take effect
immediately: a later failure does not roll them back, and a failed write may leave
a created/truncated/partially written file. No durable-storage promise is added.
Traces include operation and resource only, never paths, names or contents.

## Error categories and validation priority

Append catchable PATH at recovery index12; prior twelve indices stay fixed and
`any` becomes8191. PATH covers relative-path grammar/length, missing/unusable
roots or targets, nonregular file targets and host path refusal. Exact Linux errno
mapping: EPERM, ENOENT, ENXIO, EACCES, ENODEV, ENOTDIR, EISDIR, EXDEV, EROFS,
ENAMETOOLONG and ELOOP. Host resource exhaustion, disk full, unsupported syscall,
openat2 EAGAIN, malformed kernel records and other host failures are fatal IO.
Retry EINTR where the syscall has not completed; do not restart close or weaken
resolution flags. Invalid path/source/read/name UTF-8 or missing Text is TEXT;
Text limits and arena exhaustion are TEXT-LIMIT. Listing count/raw/encoded limits
and List arena exhaustion are LIST-LIMIT. Capability errors, TEXT-LIMIT and LIST-LIMIT remain fatal; the relative-path
grammar/4,095-byte limit is specifically recoverable PATH.

Ordering: verified program → all bindings → instruction grant → path Text → path
grammar → write source Text → host lookup/type/transfer → collection/read limits →
UTF-8 → listing sort/encoding → appropriate result arena → destination commit.
The filesystem may change while inspected; multiple independent host failures need
not have a stable order. Handlers begin after owned scratch/descriptors are released.

## Shared engine ABI and native integration

New `src/directory_host.rs` is usable by the VM and embedded no_std runtime.
No engine heap, libc, process invocation or runtime compiler is required.

```rust
#[repr(C)]
struct DirectoryRequest {
    kind: u64,                 // 0=list_names, 1=read_text, 2=write_text
    root: *const u8,            // validated caller-granted NUL-terminated path
    path: *const u8, path_len: u64,
    source: *const u8, source_len: u64,
    output: *mut u8, output_capacity: u64,
    scratch: *mut u8, scratch_capacity: u64,
} // 80 bytes, fields at offsets 0,8,...,72
#[repr(C)]
struct DirectoryResult { output_len: u64, entry_count: u64 } //16 bytes
unsafe extern "C" fn ail_directory_validate(request: *const DirectoryRequest) -> u64;
unsafe extern "C" fn ail_directory_run(request: *const DirectoryRequest,
                                     result: *mut DirectoryResult) -> u64;
```

Null path/source means missing Text; non-null zero length means present empty Text.
Unused source may be null for list/read. Validate checks kind and input validation
without reading root or requiring scratch/output, and without opening anything. Both adapters call it before
scratch allocation; run repeats it defensively. Caller supplies an output buffer
of1,048,577 bytes and fixed scratch of0x311000 bytes: two1MiB name-descriptor arrays,
1MiB packed names,64KiB getdents buffer,4KiB path C string. Descriptors are16 bytes;
all regions are aligned and checked. Read/write use the same fixed contract.

Return codes:0OK,1PATH,2TEXT,3TEXT-LIMIT,4LIST-LIMIT,5IO. Result is untouched on
error. Successful write reports0/0; read reports output bytes/0; list reports
canonical bytes/entry count. Output/scratch are private and may contain partial
data on failure. Adapters own persistent arena reservation and publication.

Native emission uses tracked scratch with cleanup on success and recovery,
callee-saved registers and aligned runtime calls. Directory effects select the
embedded runtime even without Float/process effects. Startup validates all reserved
directory flags for scalar and legacy emitters, including unused grants. Existing
programs requiring no embedded runtime retain their basic ELF route.

## Workload and acceptance

Create a pure-AIL workspace batch reporter: list the granted input root, read each
discovered name, parse/compute useful per-file JSON/text statistics, write one
computed report filename under an independently granted output root, and emit an
ordered manifest. Include hidden/Unicode/escaped names, malformed data, invalid
UTF-8 and nonregular entries. Recover PATH/TEXT/data errors per entry and continue;
fatal limits/IO retain earlier completed outputs. Report names use sorted numeric indices, so even a 255-byte input basename
fits without truncation or collision. No fixture filename list is hardcoded in source. A Python oracle independently computes files and manifest.

Acceptance covers source/saved Core/rendered source/direct STE × VM/native,
imports and whole-program checking, root/field/provenance identity, all grant
occurrences/default denial/privacy, path and permission precedence, symlink/traversal
attempts with outside sentinels, nested paths, known hard-link behavior, exact
Text/List/count/arena bounds, invalid names, aliases, cleanup under fd pressure,
partial writes and repeated calls. Explicit fixed-file compatibility tests retain
its existing symlink behavior. Races use bounded stress evidence without claiming
an atomic snapshot; required confinement is enforced by the kernel resolver.

Independent reviewers cover frontend/VM, host/native, root integration and oracle.
Final gates: fresh build, strict Clippy, all-target serial Rust tests, new oracle
and all17 prior focused oracles, frozen16/16 benchmark, broad differential with
no new divergences, unchanged reviewed hashes and exact STE exports.

Primary host references: [openat2](https://man7.org/linux/man-pages/man2/openat2.2.html),
[getdents64](https://man7.org/linux/man-pages/man2/getdents.2.html), and
[path resolution](https://man7.org/linux/man-pages/man7/path_resolution.7.html).
