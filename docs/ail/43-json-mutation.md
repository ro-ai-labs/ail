# Updating JSON records

Set and remove create new `Json` values through source, checked Core, STE, VM
and direct Linux x86-64 ELF. Use them to transform records and build reports
from computed values.

```text
the system sets JSON value at path data.path in data.document to data.value into output.document
the system removes JSON value at path data.path from data.document into output.document
```

The source and destination must be global `Json` fields, and the path must be
a global `Text` field. Set accepts a global `Int`, `Float`, `Bool`, `Text`, `List<Int>`,
`List<Float>`, `List<Text>` or `Json` value. The declared type determines JSON conversion:
Text becomes a quoted string, typed lists become arrays, and Json retains
number spellings such as `1.2500e-2` and integers outside the i64 range.
Float values must be finite and use canonical scientific JSON number notation.

## Paths and changes

Paths use the same JSON Pointer syntax as [JSON lookup](42-recursive-json.md).
Parent containers must already exist. Keys match exact decoded Unicode,
including NUL; `~0` encodes a tilde and `~1` encodes a slash.

| Target | Set | Remove |
| --- | --- | --- |
| Empty path | Replace the root | Error |
| Unique existing object key | Replace in its original position | Delete the member |
| Missing object key | Append the member | Error |
| Existing array index | Replace the item | Delete and shift later items |
| Final `-` in an array | Append the item | Error |

Array indices are unsigned decimal integers without leading zeroes. A numeric
index equal to the array length fails; use `-` for append. For objects, `-` is
an ordinary key. Selecting a duplicated key fails as ambiguous, even if its
occurrences use different JSON escape spellings. Unselected duplicate pairs,
member order, and number spellings survive unchanged.

The result is immutable: another field holding the source keeps its original
value. The destination may also be the source or any other operand field when
using direct bytecode; all operands are read before the successful write.

## Validation, limits and storage

Validation runs in this order:

1. Validate the complete source, including raw and encoded bounds.
2. Validate the complete path and resolve its parent and final target.
3. For set, validate and encode the replacement value.
4. Validate the resulting structure and encoded length, then allocate the result.

A bad path therefore takes priority over a missing or invalid replacement.
Root replacement still validates the source. Source limits apply even when a
change would reduce its size.

The source, replacement Json, and result each allow at most **1,048,576 bytes**,
**64 open containers**, and **65,536 values** including the root. Object keys do
not count as values. Paths and all value transports also have a 1 MiB raw limit.
Existing typed list restrictions apply. Attached replacement values must fit
within the result's depth and count limits; discarded subtrees do not count.

Only the final encoded Json result charges the shared **64 MiB list budget**.
Temporary parsing and formatting consume bounded scratch storage, without a
text or list budget charge. Failure preserves the destination and allocation
counters. Diagnostics omit operand contents.

Errors use `AIL-RUNTIME-JSON` for missing or malformed values,
`AIL-RUNTIME-JSON-PATH` for invalid or unresolved targets, and
`AIL-RUNTIME-JSON-LIMIT` for size, structure and allocation limits. Successful
traces are `set JSON <destination>` and `removed JSON <destination>`.

## Core, STE and scope

`JsonMutation` has no node type. Set has exactly `json_mutation_kind=set` and
`value_type` attributes; remove has only `json_mutation_kind=remove`. One ordered
`json_mutation_step` edge binds the operation to an executable action. Operand
edges are `json_mutation_source`, `json_mutation_destination`,
`json_mutation_path`, and, for set, `json_mutation_value`. Operand edges have no
attributes. Actual operands imply secret reads and writes. Imports qualify
field references.

```text
Set type_data value value_data at JSON path path_data in field source_data into field destination_data.
Remove value at JSON path path_data from field source_data into field destination_data.
```

These templates lower to `JSON_SET` with exactly `key,path,source,type,value`
and `JSON_REMOVE` with exactly `key,path,source`. Application and C interop
profiles support the operations. Compiler passes reject them.
[Value functions](44-value-functions.md) use `the function` forms on declared
locals. Invalid action Core and protocol data receive `AIL-JSON-001`, wrapped as
`AILBC016` in bytecode verification.

## File transformation example

The [JSON transform package](../../examples/json-transform) removes each record's
`private` field, doubles its integer amount, adds a Boolean flag, and appends
the updated record to a report. It then adds the total, count and names and
writes one JSON file. Other number tokens retain their original spelling.

```bash
cargo run -- ail-run examples/json-transform --action Transform \
  --read-file=input=examples/json-transform/input.json \
  --write-file=report=/tmp/ail-report.json

cargo run -- ail-compile examples/json-transform --action Transform \
  --target linux-x86_64-elf --out /tmp/ail-transform
/tmp/ail-transform \
  --read-file=input=examples/json-transform/input.json \
  --write-file=report=/tmp/ail-report-native.json
```

The [exported STE](../../examples/ste/json-transform.ail-ste) runs through the
same commands. The example produces three records and a total of `80`.
[Value functions](44-value-functions.md) now support reusable record transforms.
[Floating-point computation](45-float-computation.md) extends numeric mutation
values with finite Float; broader Python library compatibility remains on the
active objective.

After `cargo build`, run `python3 scripts/ail_json_mutation_differential.py` for
an independent Python comparison covering 558 mutations, aliases, exact limits,
allocation budgets and 50 file transformations. The 2026-10-11 verification
covers 824 current Rust tests, strict Clippy and formatting, and the example's
exact output through all six routes. A separate native review checked another
931 mutation cases. Existing JSON/literal comparisons pass, and the frozen
benchmark remains 16/16.
