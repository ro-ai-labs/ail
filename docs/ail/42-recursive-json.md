# Recursive JSON values

Use `Json` for records, objects, mixed arrays and nested values. It executes
through checked Core, STE, the VM and direct Linux x86-64 ELF output.

```text
A Data has:
- raw: Text
- document: Json
- path: Text

An Output has:
- name: Text
- count: Int

Action: Extract.
When Extract happens:
- the system parses JSON text data.raw into data.document
- the system assigns "/records/0/name" to data.path
- the system gets JSON value at path data.path from data.document into output.name
- the system counts entries in JSON data.document into output.count
- the system returns output.name
- the system returns output.count
- the system records a trace event named Extracted
```

Typed literal assignment also accepts Json, for example
`the system assigns {"records":[{"name":"Ada"}],"metadata":null} to data.document`.
Use the existing JSON format operation to write a Json value as Text for a file.

## Value representation

Json accepts the value grammar in [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259):
objects, arrays, strings, numbers, booleans and null. Its arrays may contain
different types and nested containers. Input must be valid UTF-8, including
valid paired surrogate escapes; comments, trailing commas and trailing values
are errors.

Encoding removes separator whitespace and uses AIL's existing string escaping.
Object member order and duplicate members are preserved. Number tokens retain
their exact spelling: `-0`, `1.2500e-2` and `18446744073709551616` survive parse,
format and Json extraction without rounding. This preserves data; integer
computation still uses checked i64 arithmetic. Json encoding is deterministic
for this representation but is not a key-sorting or numeric canonicalization
standard.

## Lookup and counts

Paths use the string form of [JSON Pointer](https://www.rfc-editor.org/rfc/rfc6901).
The empty string selects the whole value. `/records/0/name` selects `name` in
the first record. Encode a key's tilde as `~0` and slash as `~1`; `/` selects
an empty object key. Decoding happens once, so `~01` selects a key named `~1`.
URI fragments beginning with `#` are not path strings.

Array indices are unsigned decimal integers with no leading zeroes. Negative
indices and `-` do not select an element. Keys match exact Unicode characters,
without normalization; keys can contain NUL. Missing values, traversal through
a scalar and lookup of a duplicated member name fail. Escaped spellings that
decode to the same name are duplicates for lookup.

The destination type controls extraction:

| Destination | Required selected value |
| --- | --- |
| `Json` | Any JSON value, retained in compact encoded form |
| `Text` | String, decoded into text |
| `Int` | Strict JSON integer in the signed 64-bit range |
| `Float` | JSON number which rounds to finite binary64 |
| `Bool` | Boolean |
| `List<Int>` / `List<Float>` / `List<Text>` | Flat array of the corresponding element type; Float numbers must round finite |

Count returns the number of immediate array elements or object member pairs.
Duplicate member pairs count separately. Counting a scalar fails. Lookup and
count validate the whole document, including unselected members.
[JSON inspection](47-json-inspection.md) adds root-kind discovery and ordered
object-key enumeration for unknown records. In-place Json
lookup is safe, and copies retain their original contents.

## Limits and errors

- Raw document, compact document, result and path: at most **1,048,576 bytes** each.
- Nesting: at most **64 simultaneously open containers**.
- Values: at most **65,536**, including the root; member names do not count.
- Json results share the existing **64 MiB list allocation budget** per root run.

Parse Json and get Json charge only final encoded bytes. Format Json charges
the text budget. Typed lookup charges its final Text/list result as ordinary
JSON parsing does; Int/Bool lookup and counts charge neither budget. Literals
remain constant data and consume no runtime result budget. Temporary document
and path storage does not add semantic allocation charges.

The complete bounded document is parsed before checking its encoded size, then
the whole path syntax is validated before traversal. Structural depth/value
limits may fail during parsing. Failed operations preserve the destination and
allocation counters. Diagnostics omit input contents.

`AIL-RUNTIME-JSON` covers invalid input/type/UTF-8 and missing fields.
`AIL-RUNTIME-JSON-PATH` covers malformed, missing, ambiguous or invalid traversal
paths. `AIL-RUNTIME-JSON-LIMIT` covers size, depth, value-count and allocation
limits. Native scratch allocation failure also uses JSON-LIMIT; recoverable
host out-of-memory errors are not a VM guarantee.

## Core, STE and scope

`JsonAccess` nodes carry `json_access_kind=get|count|kind|keys` and `value_type`. Ordered
`json_access_step` edges bind them to an executable action; typed global operand
edges are `json_access_source`, `json_access_destination` and, for get only,
`json_access_path`. Actual operands imply secret reads/writes. Imports qualify
field names. Application and C interop profiles support the operations;
compiler passes reject them. [Value functions](44-value-functions.md) support
local lookup, count, kind inspection and key enumeration with `the function` forms.

```text
Get type_data value at JSON path path_data from field source_data into field destination_data.
Count entries in JSON field source_data into field destination_data.
```

These lower to `JSON_GET` with exactly `key,path,source,type` and `JSON_COUNT`
with exactly `key,source`. Existing JSON parse/format and literal instructions
also accept the `Json` type.

## Nested record example

The [JSON records package](../../examples/json-records) reads nested records,
builds a dynamic pointer for each array index, extracts names and integer
amounts, and writes JSON names and their total:

```bash
cargo run -- ail-run examples/json-records --action Process \
  --read-file=input=examples/json-records/input.json \
  --write-file=names=/tmp/ail-record-names.json \
  --write-file=total=/tmp/ail-record-total.json

cargo run -- ail-compile examples/json-records --action Process \
  --target linux-x86_64-elf --out /tmp/ail-json-records
/tmp/ail-json-records \
  --read-file=input=examples/json-records/input.json \
  --write-file=names=/tmp/ail-native-record-names.json \
  --write-file=total=/tmp/ail-native-record-total.json
```

Expected files are `["Ada","Mihai","Zoë 🙂"]` and `40`, with no trailing newline.
The original payload's large request ID and decimal ratio are accepted without
being converted to machine integers or floats.
[Exported STE](../../examples/ste/json-records.ail-ste) runs or compiles directly.

After `cargo build`, run `python3 scripts/ail_structured_json_differential.py`
for independent Python comparison, strict-grammar/pointer probes, exact limits,
allocation checks and nested-file computations. The oracle retains number
tokens and object pairs, so ordinary floating-point rounding or dictionary
deduplication cannot hide a representation error.

The 2026-10-11 implementation passes **790 Rust tests** with the full suite run
serially, strict Clippy and formatting checks. The independent oracle passes
253 recursive round trips, 666 typed lookups, 100 counts, 40 file datasets and
its malformed/boundary/allocation probes. The example's exact files are verified
through source, saved Core and direct STE on both backends. Existing typed JSON
and literal differential checks pass; the frozen benchmark remains 16/16.

[JSON mutation](43-json-mutation.md) adds set/remove and nested report construction.
[Value functions](44-value-functions.md), [floating-point arithmetic](45-float-computation.md)
and [floating lists](46-floating-lists.md) extend reusable computation. Broader
collections, arbitrary precision and Python compatibility remain open.
