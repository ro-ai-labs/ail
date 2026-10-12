# Bytes values and binary files

`Bytes` represents immutable sequences of arbitrary octets, including NUL and
invalid UTF-8. It is a primitive field, literal, parameter, local and result type.
The same operations execute through source, saved Core, rendered source, STE,
bytecode, VM and standalone Linux x86-64 binaries.

## Values and transport

Runtime state uses `hex:` followed by even lowercase hexadecimal digits. Empty
is `hex:`; it differs from a missing field. For example, `hex:00ff80` contains
three bytes. Source literals use a JSON string and the destination's declared
type:

```text
A Data has:
- body: Bytes
- text: Text
- count: Int

Action: Prepare.
When Prepare happens:
- the system assigns "hex:00ff80" to data.body
- the system counts bytes in data.body into data.count
- the system records a trace event named Prepared
```

Uppercase transport digits, odd lengths, missing prefixes, whitespace and
non-string Bytes literals reject. A Text field containing the same spelling
remains Text. No implicit Text, Json, Bool or list conversion is provided, and
`List<Bytes>` is not an executable list type. Use explicit UTF-8/hex conversion
to bridge text and JSON APIs.

Legacy `changes SOURCE as DESTINATION` copies can alias Bytes only when both
declared fields are Bytes. Cross-type copies reject, and Bytes constants require
the explicit typed `assigns` form.

## Pure operations

Use these exact action forms; replace `the system` with `the function` inside
an executable typed function. Field names below illustrate declared operands.

| Source form | Result |
| --- | --- |
| `counts bytes in data.body into data.count` | Int raw byte length |
| `gets byte data.index from bytes data.body into data.value` | Int from 0 through 255 |
| `slices bytes data.body from data.start to data.end into data.part` | Bytes slice |
| `joins bytes data.left and data.right into data.body` | Bytes concatenation |
| `sets byte data.index in bytes data.body to data.value into data.result` | New Bytes value |
| `encodes text data.text as UTF-8 bytes into data.body` | Bytes UTF-8 encoding |
| `decodes bytes data.body as UTF-8 text into data.text` | Text, strict UTF-8 |
| `decodes hexadecimal text data.text into bytes data.body` | Bytes hex decoding |
| `encodes bytes data.body as hexadecimal text into data.text` | Text lowercase hex |
| `compares bytes data.left with data.right into data.order` | Int -1, 0 or 1 |

Index, replacement value, start and end accept checked Int expressions. Get/set
indices count from the end when negative and reject positions outside the value.
Slices exclude the end, clamp bounds, support negative bounds and return empty
when reversed. Set accepts only 0..255. Comparison uses unsigned byte
lexicographic order. All inputs can alias the destination without mutation of
other values that share their storage.

Hex decoding accepts either case, with even length and no prefix or whitespace;
encoding returns lowercase digits without `hex:`. UTF-8 decoding rejects invalid,
overlong, surrogate and incomplete sequences. Encoding preserves exact UTF-8.

All source transports validate first, left then right. Numeric expressions then
evaluate in source order: index before value, or start before end. Only then do
index/range checks occur. Thus a failing value expression precedes an invalid
set index, but an invalid index precedes a replacement value outside 0..255.
Successful publication happens after result and allocation checks. Failure leaves
the destination and persistent counters unchanged; earlier successful work
remains charged.

## File effects

```text
- the system reads bytes from file resource input into data.body
- the system writes bytes data.body to file resource output
```

Use the existing `--read-file=input=PATH` and `--write-file=output=PATH` grants,
`AilFileResources`, and public host APIs. Paths belong to the caller, not to the
program or mutable state. All bindings validate at startup, including unused and
duplicate bindings. Each operation checks its grant before processing data.
Read/write grants remain separate.

Reads consume a regular file through EOF without decoding UTF-8. Writes validate
and decode the complete transport before opening or truncating the output, and
add no newline. Reads publish atomically. Partial external writes can survive a
later IO failure. Existing relative-path and symlink behavior of fixed file grants
is unchanged; use directory resources for their different containment rules.
Directory, process and standard-stream operations continue to use Text.

See the [binary header inspector and exact file copy](../../examples/binary-files)
and its [STE export](../../examples/ste/binary-files.ail-ste).

## Limits and errors

A Bytes value contains at most **1,048,576 raw bytes**; canonical transport can
therefore occupy **2,097,156 bytes**. Bytes literals can exceed the Text bound but
still obey the existing 6,291,458-byte raw source-literal bound. Static literals
are uncharged. Computed Bytes and successful function returns charge their whole
transport, including four bytes for empty, to the shared **67,108,864-byte
List/Json arena**. Borrowed function arguments validate without charging. A
successful Bytes return copies once; earlier inner results retain their charges.

Text conversions obey the separate 1 MiB Text limit and 64 MiB Text arena. A valid
Bytes value can be too large for hexadecimal Text encoding; that fails with
TEXT-LIMIT. Scalar length/get/compare results charge neither arena. File reads
observe at most 1 MiB plus one sentinel byte before reporting overflow.

| Diagnostic | Meaning |
| --- | --- |
| `AIL-BYTES-001` | Invalid source/Core types, structure or operation |
| `AILBC023` | Invalid bytecode byte operation or profile |
| `AIL-RUNTIME-BYTES` | Missing/malformed Bytes, invalid hex Text, invalid replacement byte |
| `AIL-RUNTIME-INDEX` | Out-of-range get/set index |
| `AIL-RUNTIME-TEXT` | Missing Text or invalid UTF-8 decoding |
| `AIL-RUNTIME-BYTES-LIMIT` | Bytes size or shared result budget exceeded |
| `AIL-RUNTIME-TEXT-LIMIT` | Text input/output size or Text arena exceeded |
| `AIL-RUNTIME-CAPABILITY` | Invalid or absent caller grant |
| `AIL-RUNTIME-IO` | Host file or byte adapter failure |

BYTES is catchable alongside existing data-error categories. BYTES-LIMIT,
TEXT-LIMIT, CAPABILITY and IO are fatal. Existing integer errors retain their
categories. Recovery appends BYTES at index 13; `any` covers all fifteen current
[categories](51-runtime-recovery.md), including VALUE for executable variants.
Diagnostics and traces omit operand contents and caller paths.

## Core and STE

An untyped `BytesOperation` has exactly one `bytes_kind` attribute with values
`length`, `get`, `slice`, `concat`, `set`, `from_utf8`, `to_utf8`, `from_hex`,
`to_hex`, or `compare`. One executable Action owns it through `bytes_step`, with
required `step_index` and optional `operation_index`. Operand roles are
`bytes_source`, `bytes_other`, `bytes_destination`, `bytes_index`, `bytes_start`,
`bytes_end` and `bytes_value`. Numeric roles own checked IntegerExpression trees;
other roles reference declared global fields. Concat/compare left and right use
source and other respectively. Exact roles/types, ownership, identities,
provenance and source order are checked, and semantic field reads/writes remain
subject to access checks even without descriptive edges.

Functions use the existing scoped typed operand schema. Recursion, immutable
aliases and explicit Bytes returns are supported. Binary file effects remain
action-only. Application and C interop profiles support execution; descriptive
function/compiler-pass bodies and malformed reserved forms reject. Verification
covers the whole program, including unreachable code.

Bytecode uses `BYTES_LENGTH`, `BYTES_GET`, `BYTES_SLICE`, `BYTES_CONCAT`,
`BYTES_SET`, `BYTES_FROM_UTF8`, `BYTES_TO_UTF8`, `BYTES_FROM_HEX`, `BYTES_TO_HEX`
and `BYTES_COMPARE`; files use `FILE_READ_BYTES` and `FILE_WRITE_BYTES`.
STE uses one controlled imperative sentence per instruction with operands in
technical data. The exact templates, in the opcode order above, are:

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

The established AIL-STE programming profile is based on ASD-STE100; it is not a
claim that arbitrary AIL documents conform to the full standard.
