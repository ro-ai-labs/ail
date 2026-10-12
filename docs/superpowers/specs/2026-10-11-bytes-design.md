# Bytes values and binary files

## Intent

The active goal remains Specs → ASD-STE100-based executable intermediate → binary
and a language useful for Python replacement and AI agents. Text-only file access
prevents processing ordinary binary artifacts. Add a distinct Bytes value type,
pure byte operations, typed function/literal support, and caller-bound binary
file access across source, Core, STE, bytecode, VM and native Linux x86-64.

This is an architectural language change. Reusing List<Int> would not enforce byte
width or provide a compact byte value; disguising bytes as Text would undermine
UTF-8 guarantees. Use explicit Bytes with a deterministic textual transport that
fits the existing state/bytecode protocol. Runtime values remain immutable;
destination fields may be overwritten without changing existing aliases.

## Representation and bounds

`Bytes` is a supported primitive for fields, literals, function parameters,
locals and results. Its exact runtime transport is `hex:` followed by an even
number of **lowercase ASCII hexadecimal digits**, no whitespace. Empty is `hex:`.
For example `hex:00ff80` represents three bytes. CLI state and default/selected
state output use this transport. Missing is distinct from present empty.
This is an external representation, not an implicit Text-to-Bytes cast.

Source literals are JSON strings, e.g. `the system assigns "hex:00ff80" to data.body`.
The destination type distinguishes Bytes from Text. Reject uppercase, odd digits,
bad prefix, other characters and non-string literals statically. Literal/runtime
transport length is at most 2,097,156 bytes, representing at most **1,048,576 raw
bytes**. Existing raw-literal bound 6,291,458 remains unchanged. Large valid Bytes
literals must survive source/Core/STE rendering even beyond Text's 1 MiB bound.

Computed Bytes and successful Bytes returns charge their complete canonical
transport length (`4 + 2*raw_length`, including four bytes for empty) to the shared
**67,108,864-byte List/Json result budget**. Static literals remain uncharged.
Borrowed function arguments validate but do not charge; returns charge one copy.
No new arena or public host-context field is needed. Failed operations preserve
destination and both counters. Existing successful inner work is not rolled back
by a later failure. Text results retain the existing separate 64 MiB Text budget,
max(length,1) charging and 1 MiB UTF-8 limit. Scalar results charge neither arena.
No implicit JSON conversion, List<Bytes>, truthiness or Text coercion is added;
explicit hex/UTF-8 conversion integrates with existing JSON/Text operations.

## Pure operations

Actions use `the system`; value functions use `the function` and declared locals.
Every source/other/destination is a field of the specified type. Bounds, index and
replacement value are checked Int expressions. Input aliases with output are valid.

| Source form after `the system` | Opcode / exact operands | Types |
| --- | --- | --- |
| counts bytes in SOURCE into KEY | BYTES_LENGTH source,key | Bytes → Int |
| gets byte INDEX from bytes SOURCE into KEY | BYTES_GET source,index,key | Bytes,Int → Int |
| slices bytes SOURCE from START to END into KEY | BYTES_SLICE source,start,end,key | Bytes,Int,Int → Bytes |
| joins bytes LEFT and RIGHT into KEY | BYTES_CONCAT left,right,key | Bytes,Bytes → Bytes |
| sets byte INDEX in bytes SOURCE to VALUE into KEY | BYTES_SET source,index,value,key | Bytes,Int,Int → Bytes |
| encodes text SOURCE as UTF-8 bytes into KEY | BYTES_FROM_UTF8 source,key | Text → Bytes |
| decodes bytes SOURCE as UTF-8 text into KEY | BYTES_TO_UTF8 source,key | Bytes → Text |
| decodes hexadecimal text SOURCE into bytes KEY | BYTES_FROM_HEX source,key | Text → Bytes |
| encodes bytes SOURCE as hexadecimal text into KEY | BYTES_TO_HEX source,key | Bytes → Text |
| compares bytes LEFT with RIGHT into KEY | BYTES_COMPARE left,right,key | Bytes,Bytes → Int |

Length/index/slices count raw bytes, not Unicode scalars. Index supports negatives
from the end; out-of-range or any index into empty reports INDEX. Slice is
end-exclusive, accepts negative/clamped bounds, and returns empty when end precedes
start. Set requires replacement 0..255 and returns a fresh value. Compare returns
-1,0,1 using unsigned byte lexicographic ordering. Concat preserves exact bytes.
No integer/native-endian reinterpretation or implicit decoding occurs.

UTF-8 encode preserves exact bytes of validated Text; decode is strict, rejecting
overlong, surrogate, truncated and out-of-range sequences with TEXT. Hex decode
accepts even-length ASCII 0-9/a-f/A-F without prefix or whitespace, emits canonical
Bytes, and rejects other input with BYTES. Hex encode emits lowercase unprefixed
Text. Hex output can exceed Text's 1 MiB limit even when Bytes input is valid;
that is TEXT-LIMIT. Conversely hex Text input is bounded by Text's existing limit.

Order: validate source transport (left then right for binary operations), evaluate
Int operands (index then value; start then end), validate index before replacement
range, generate/check result, reserve the appropriate shared arena, publish once.
All Int operands evaluate before operation-specific bounds checks. For instance
an error in the value expression can precede an invalid index in BYTES_SET.
Temporary host allocation failures need not have deterministic priority over
later result checks, but persistent result budgets change only on success.

## Diagnostics and recovery

Static Bytes operations use AIL-BYTES-001; malformed bytecode uses AILBC023.
Existing literal/function/file static diagnostic families stay in place.
Append catchable BYTES at recovery index13; existing indices unchanged, any=16383.
BYTES covers missing/malformed canonical transport, invalid hex input and a set
value outside 0..255. INDEX covers get/set position; TEXT covers missing/invalid
Text and invalid UTF-8 decode. Int expression errors preserve their categories.
BYTES-LIMIT covers raw/canonical Bytes bounds and Bytes allocation failure against
the shared List/Json result budget; it is fatal. TEXT-LIMIT and host IO remain
fatal. Failed effects do not become rollback transactions. Diagnostics/traces
omit operand contents and caller paths.

## Core, functions and STE

Add `AilBytesOperation` and ordered `AilActionStep::Bytes` with corresponding
`AilAction.bytes_operations`. Use untyped `BytesOperation` nodes with exactly
`bytes_kind=length|get|slice|concat|set|from_utf8|to_utf8|from_hex|to_hex|compare`.
One ordered `bytes_step` belongs to an executable Action; step_index required,
operation_index optional, no unrelated metadata. Roles are bytes_source,
bytes_other, bytes_destination, bytes_index, bytes_start, bytes_end, bytes_value.
Concat/compare's left uses bytes_source and right uses bytes_other. Numeric roles
target typed IntegerExpression trees; other roles target genuine global fields.
Require exact unique roles/types, unique participating identities and provenance,
correct ownership, no hidden/reversed/extra edges, and semantic reads/writes even
without descriptive access edges. Namespaced imports qualify fields/expressions.
Saved Core must retain duplicate structural edges for rejection.

Value functions reuse the established typed local operation schema and scoped
operand trees. All ten pure operations work in recursive value functions; binary
file effects remain action-only. Application and C interop support execution;
compiler-pass/descriptive functions and malformed reserved forms reject. Whole
program verification includes unreachable bodies. Bytes cannot be used in an Int,
Text, list, JSON or Bool slot or operation by representation coincidence.

Exact STE templates (operands remain technical data):

```text
Count bytes in field {source} into field {key}.
Get byte at index {index} from field {source} into field {key}.
Slice bytes in field {source} from {start} to {end} into field {key}.
Join bytes in field {left} and field {right} into field {key}.
Set byte at index {index} in field {source} to value {value} into field {key}.
Encode UTF-8 text from field {source} into bytes field {key}.
Decode UTF-8 bytes from field {source} into text field {key}.
Decode hexadecimal text from field {source} into bytes field {key}.
Encode bytes from field {source} into hexadecimal text field {key}.
Compare bytes in field {left} with field {right} into field {key}.
Read bytes from file resource {resource} into field {key}.
Write bytes from field {source} to file resource {resource}.
```

## Shared pure engine ABI

`src/byte_engine.rs` is dependency-free/no_std-compatible and shared by VM/native.
Kinds0..9 follow the operation table above. Internal kinds10=raw→transport and
11=transport→raw support binary files; they are not new public bytecode operations.

```rust
#[repr(C)]
pub struct ByteRequest {
    pub kind: u64,
    pub left: *const u8, pub left_len: u64,
    pub right: *const u8, pub right_len: u64,
    pub first: i64, pub second: i64,
    pub output: *mut u8, pub output_capacity: u64,
} //72 bytes, offsets0,8,...,64
#[repr(C)]
pub struct ByteResult { pub output_len: u64, pub scalar: i64 } //16 bytes
pub unsafe extern "C" fn validate(request: *const ByteRequest) -> u64;
pub unsafe extern "C" fn run(request: *const ByteRequest, result: *mut ByteResult) -> u64;
pub fn validate_transport(value: &[u8]) -> Result<usize,u64>;
```

validate checks kind and input transports (including from-hex grammar), without
accessing output or validating numeric operation bounds. Call it before operand
evaluation/output allocation. run repeats validation, checks numeric/result rules,
then writes output/result; result descriptor remains untouched on failure. Scalar
operations require no output allocation. Output may need2,097,156 bytes. Null input
means missing; present empty Text has non-null pointer/zero length. No heap/global
state, host effects or overlapping input/output buffers in this engine.

Statuses0 OK,1 BYTES,2 BYTES-LIMIT,3 TEXT,4 TEXT-LIMIT,5 INDEX,6 IO (bad ABI/capacity).
The adapters own arena charging and atomic field publication. Existing embedded
runtime exports add ail_bytes_validate/ail_bytes_run without changing old ABIs.
Native callers preserve registers/alignment and free tracked scratch before
recovery. Bytes parameters borrow validated transport; returns materialize one
copy into the shared List/Json arena, with BYTES-LIMIT on failure.

## Binary file effects

```text
- the system reads bytes from file resource input into data.body
- the system writes bytes data.body to file resource output
```

Extend AilFileOperation with ReadBytes/WriteBytes; FileOperation file_kind gains
read_bytes/write_bytes, same resource/operand roles with Bytes type. Bytecode is
FILE_READ_BYTES resource,key and FILE_WRITE_BYTES resource,source. Reuse existing
caller --read-file/--write-file and AilFileResources; earlier APIs/struct layouts
stay unchanged. Native/VM both validate all caller bindings before effects and
check operation grant before reading data. Mutable state cannot grant authority.

Read regular files as arbitrary bytes through EOF, observing at most1,048,577 to
detect overflow before encoding. Charge the full canonical transport once to the
shared List/Json arena and publish atomically. Write validates/decode Bytes before
opening or truncating; exact raw bytes, no newline conversion. Open regular files
with current no-terminal/nonblocking behavior; fixed caller paths retain their
normal host symlink/relative-path semantics. Read/write IO failures stay fatal IO;
size/budget failures use BYTES-LIMIT. Close owned descriptors before propagation;
retry EINTR but do not retry close. Writes can leave partial external effects.
Share existing file open/type/read/write machinery where practical; do not weaken
Text validation or duplicate the entire filesystem implementation. The VM file
execute signature can retain its single counter parameter; dispatch selects Text
or List/Json counter by opcode. Native bytes writes can use the existing raw
file_write_text syscall helper after explicit byte decoding.

Directory/process/stdio effects retain current Text contracts in this milestone;
the Bytes type/conversions are reusable foundations for later binary effect forms.

## Acceptance and workflow

Tests independently compare arbitrary bytes and Python byte/string operations,
all256 byte values, embedded NUL/invalid UTF-8, valid Unicode boundaries, aliases,
literal/function/recursive transports, error ordering, exact1MiB values,1MiB Text
conversion cutoffs, shared64MiB arenas, caller grants and real filesystem writes.
Exercise source, saved Core, rendered source, STE and bytecode on VM/native;
reject malformed Core/bytecode and reserved source forms, with mutation-sensitive
expected results. Keep all17 old rung definitions/score intact. Add a real binary
file example using reusable AIL functions and a generated STE export.

Continue in the shared workspace; .git is read-only, no commits/worktrees/reset.
Preserve all earlier work. The previous calculator milestone was concrete progress;
its98 final artifacts were rehashed unchanged, and the new implementation baseline
is /tmp/ail-bytes-baseline.json. Existing autonomous authorization covers this
reversible language work. Full Python replacement remains active after the Bytes
milestone; this does not establish ecosystem compatibility or all I/O surfaces.
