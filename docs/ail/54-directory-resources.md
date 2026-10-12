# Named directory resources

AIL actions can discover immediate entry names, read a computed relative file,
and write a computed relative file under caller-bound directory roots. Source,
checked Core, STE, the VM and standalone Linux x86-64 ELF preserve these effects.

```text
- the system lists names in directory resource workspace at data.path into data.names
- the system reads text from directory resource workspace at data.path into data.body
- the system writes text data.body to directory resource reports at data.path
```

Declare `path` and `body` as Text and `names` as List<Text>. These are global
fields in Application or C interop actions. The operations work inside branches,
loops, recovery scopes and called actions. Functions can compute paths and
contents; host effects require actions. A read may replace its path field, and
a write may use the same field for path and contents. Evaluation uses the values
before the operation publishes its result.

## Caller authority

```bash
target/debug/ail ail-run examples/directory-report --action Generate \
  --read-directory=workspace=/absolute/input \
  --write-directory=reports=/absolute/output
```

Both directories must already exist. The [example](../../examples/directory-report)
discovers input names, computes per-file JSONL reports, and writes an ordered
manifest. `ail-run`, `ail-vm` and emitted ELF accept directory flags at execution
time. Each flag is one argument; a root may contain spaces or `=`. Resource names
use the qualified ASCII [file-resource grammar](40-file-resources.md), at most
256 bytes. Roots must be absolute, contain no NUL, and fit 4,095 encoded bytes.
Imports qualify resource names and field references.

Read grants permit listing and reading; write grants permit writing. They are
independent, including when the resource has the same name. All supplied grants
are validated before effects, including unused and overwritten bindings. The
last valid binding for a name wins. Unused roots are not opened. Missing authority
fails before evaluating path or contents. State, metadata, fixed-file grants,
process grants and `--stdio` cannot grant directory access. Compiled artifacts
contain resource names; caller grants remain outside the program's mutable state
and default output.

The host API groups grants without changing earlier public structures:

```rust
let mut resources = AilHostResources::default();
resources.directories.reads.insert("workspace".into(), input_root);
resources.directories.writes.insert("reports".into(), output_root);
let result = run_ail_bytecode_action_with_host_resources(
    &program, "Generate", state, &resources, &mut streams,
)?;
```

`AilHostResources` contains `files`, `processes`, and `directories`.
`AilDirectoryResources` contains `reads` and `writes` maps.
`parse_ail_directory_binding` parses `resource=/absolute/root`. Earlier execution
APIs retain their signatures and grant no directories. Whole-program verification
and file, process, then directory binding validation precede effects. Nested
and repeated action calls borrow the same authority and result budgets.

## Relative paths and resolution

A path is exact UTF-8 Text, at most 4,095 bytes. It cannot contain NUL, begin or
end with `/`, contain repeated `/`, or include `.` or `..` components. There is no
normalization. Empty Text means the granted root only for listing. Nested paths
are allowed; write operations do not create parent directories.

The effect requires Linux x86-64 with `openat2` (Linux 5.6 or later). The runtime
opens and pins the caller root for each operation, then resolves the relative
path with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`. There is no fallback that
weakens those flags. Symlinks in the caller-supplied root use ordinary host
semantics; symlinks in any relative component fail PATH. A renamed/replaced root
can therefore select a different tree on the next operation.

This controls path resolution, not inode ownership or all OS access. Hard links
and mounted subtrees retain ordinary host behavior. An opened object can be
renamed while the operation runs. There is no filesystem snapshot. Files are
opened nonblocking with no controlling-terminal acquisition; their type is
checked before transfer or truncation. Only regular files support text reads
and writes. A device's open callback can still run before type rejection.
Existing [fixed-file bindings](40-file-resources.md) keep their previous contract.

## Listing, text and limits

Listing returns all immediate basenames except `.` and `..`, including directory,
symlink and special-file names. It is nonrecursive and does not follow those
entries. Names are sorted by unsigned UTF-8 bytes, equivalent to Unicode scalar
lexical order. Results preserve spaces, controls and Unicode exactly in the
canonical List<Text> representation. Duplicate names observed during concurrent
directory mutation are retained; concurrent listings are not snapshots.

Collection permits at most 65,536 names and 1,048,576 packed raw name bytes,
excluding separators. These limits are checked before validating all names as
UTF-8. The resulting canonical list must also fit 1,048,576 bytes, including
quotes, commas and escapes. Thus an invalid name reports TEXT after bounded
collection, and escaped names can hit LIST-LIMIT before reaching the raw limit.

Text inputs and reads permit at most 1,048,576 bytes. Reads check size before
UTF-8 and preserve NUL, CR and LF without adding a newline. Writes validate path
and source before opening the root, create a missing file with mode 0666 subject
to the caller's umask, and truncate an existing regular file before transferring
the exact source bytes. There is no durability or rollback promise: a failed
write can leave a created, truncated or partially written file.

Successful listing charges its encoded length to the shared 67,108,864-byte
List/Json budget; `[]` costs two bytes and charges no Text budget. A read charges
`max(bytes, 1)` to the separate 67,108,864-byte Text budget. Writes charge neither
result budget. Listing/read destination and budget commit together after success;
failure changes neither, and previous copies retain their values.

## Errors and recovery

```text
- try:
- the system reads text from directory resource workspace at data.path into data.body
- recover PATH or TEXT into data.failure:
- the system assigns "unavailable" to data.body
- end try
```

| Diagnostic | Meaning |
| --- | --- |
| AIL-DIRECTORY-001 / AILBC022 | Invalid source, Core, profile or directory instruction |
| AIL-RUNTIME-CAPABILITY | Missing grant or malformed caller binding; fatal |
| AIL-RUNTIME-PATH | Invalid relative grammar/length, rejected file kind, or path access failure |
| AIL-RUNTIME-TEXT | Missing Text or invalid UTF-8 path, source, name or file contents |
| AIL-RUNTIME-TEXT-LIMIT | Text input/read or shared Text result budget exceeded; fatal |
| AIL-RUNTIME-LIST-LIMIT | Listing count, raw bytes, canonical bytes or List/Json budget exceeded; fatal |
| AIL-RUNTIME-IO | Other host/descriptor/allocation failures; fatal |

Validation order is grant, path Text presence/size/UTF-8, relative path, write
source Text, then host access. A path over the Text limit reports TEXT-LIMIT;
a valid Text path over 4,095 bytes reports PATH. PATH maps EPERM, ENOENT, ENXIO,
EACCES, ENODEV, ENOTDIR, EISDIR, EXDEV, EROFS, ENAMETOOLONG and ELOOP, plus
nonregular files. Other errors, including ENOSYS, EINVAL, EAGAIN, descriptor
exhaustion and disk full, map to IO. An operation blocked by syscall filtering
follows its returned errno; EPERM cannot identify the reason for the refusal.

PATH is appended at recovery index 12; `any` matches fifteen categories. Existing indices
remain unchanged. Owned descriptors and temporary buffers are released before a
handler runs; close is not retried and close errors are ignored. Earlier effects
remain. Traces identify operation and resource, without paths, names or contents.
See [runtime recovery](51-runtime-recovery.md).

## Core and STE

`DirectoryOperation` has exactly `directory_kind=list_names|read_text|write_text`
and `resource`, with no type annotation. It has one ordered `directory_step`
Action owner and exactly one attribute-free `directory_path` edge. List/read
have one `directory_destination`; write has one `directory_source`. Operands
target uniquely declared global fields of the required types. Hidden/duplicate
roles, metadata, ambiguous identities, function ownership and unsupported profiles
fail. Semantic access checks derive path/source reads and destination writes.

| Opcode | Exact operands | STE template |
| --- | --- | --- |
| DIRECTORY_LIST_NAMES | resource,path,key | `List names in directory resource {resource} at field {path} into field {key}.` |
| DIRECTORY_READ_TEXT | resource,path,key | `Read text from directory resource {resource} at field {path} into field {key}.` |
| DIRECTORY_WRITE_TEXT | resource,path,source | `Write text from field {source} to directory resource {resource} at field {path}.` |

All placeholders use declared technical data. Whole-program verification covers
unreachable instructions and function bodies. The
[implementation record](../superpowers/plans/2026-10-11-directory-resources.md)
tracks review and differential evidence. Directory discovery expands practical
agent automation; networking, richer filesystem operations, Python ecosystem
compatibility and the broader language goal remain future work.
