# JSON inspection

JSON kind inspection and object-key enumeration let programs process documents
whose field names and nested structure are unknown when the program is written.
They execute through checked source, Core, STE, the VM and standalone native ELF.

```text
A Data has:
- document: Json

An Output has:
- kind: Text
- keys: List<Text>

Action: Inspect.
When Inspect happens:
- the system gets JSON kind from data.document into output.kind
- the system gets JSON keys from data.document into output.keys
- the system returns output.kind
- the system returns output.keys
```

The source must be `Json`. Kind requires a `Text` destination and returns one of
`null`, `boolean`, `number`, `string`, `array`, `object`. Classification does not
convert number tokens, so arbitrarily large JSON numbers within the document
bounds still have kind `number`.

Keys requires an object and a `List<Text>` destination. It returns decoded member
names in encounter order, including duplicates. For example,
`{"a":1,"\u0061":2,"":null,"a/b":true}` gives `["a","a","","a/b"]`.
Names may contain NUL, slash, tilde and Unicode. There is no key sorting,
deduplication or Unicode normalization. Arrays and scalars fail key enumeration.

Use the same operations with `the function` on declared locals in an executable
value function. Combined with collection counts, loops, recursive functions and
[JSON Pointer lookup](42-recursive-json.md), they support generic tree traversal.
Escape tilde and slash in a discovered key as `~0` and `~1` before constructing
a JSON Pointer. Duplicate-name lookup retains its existing ambiguity error.

## Validation, allocation and errors

Both operations validate the complete source, including every unselected child,
before inspecting its root. The existing limits apply: **1,048,576 raw and compact
bytes**, **65,536 values including the root**, and **64 open containers**.
Thus an object can contain at most 65,535 direct member values; key names do not
count as values. Malformed or oversized nested data fails even when only its
root kind was requested.

Kind produces an owned Text word and charges its length once to the shared
64 MiB text arena. Keys produces one canonical List<Text> and charges its final
encoded bytes once to the shared 64 MiB list/Json arena. Empty-object keys are
`[]` and cost two bytes. Temporary parsing and encoding do not consume result
budgets. A function that returns the produced Text/list also makes its normal
one-copy return charge. Replacing a destination never refunds prior charges.

Source syntax, structural and compact-size checks precede root-type checks,
then result allocation. Invalid/missing source or wrong key-enumeration type
uses `AIL-RUNTIME-JSON`; size, structure and allocation failures use
`AIL-RUNTIME-JSON-LIMIT`. Diagnostics omit data contents. Failed operations
preserve destination and counters. Direct-protocol source/destination aliases
are safe and successful outputs survive later parsing or mutation.

## Core and STE

`JsonAccess` uses `json_access_kind=kind|keys` with `value_type=Text|List<Text>`
respectively. Its existing ordered step, source and destination edges carry
checked operands. These forms have no path edge. Checks enforce exact metadata,
unique genuine global storage, local ownership, types, secret reads/writes and
import qualification. Application and C interop support execution; descriptive
function and compiler-pass bodies reject the new statements.

```text
Get JSON kind from field source_data into field destination_data.
Get JSON keys from field source_data into field destination_data.
```

`JSON_KIND` and `JSON_KEYS` each require exactly `key,source`. They reuse the
JSON diagnostic families `AIL-JSON-001` and `AILBC016`. Native output reuses the
bounded recursive parser and decoded descriptors, then copies one final result.
Inspecting a lossless number does not require floating-point runtime helpers.

## Generic file traversal

The [JSON inspection example](../../examples/json-inspection) recursively counts
scalar leaves in objects and heterogeneous arrays. Its AIL pointer-escaping
helper handles unknown keys, including slash, tilde, empty and NUL names. The
sample has kind `object` and **8 scalar leaves**; the output report retains its
original number tokens. [Exported STE](../../examples/ste/json-inspection.ail-ste)
runs through the same CLI as the source package.

Empty containers contribute zero leaves; all scalar kinds contribute one.
Ordinary function-depth, fuel and allocation limits still apply. Enumeration
preserves duplicate keys, while this example's subsequent ambiguous lookup
fails before the output file is written.

See the [implementation record](../superpowers/plans/2026-10-11-json-inspection.md)
for verification evidence. The broader Python-replacement goal remains active.


After `cargo build`, run `python3 scripts/ail_json_inspection_differential.py`
for independent checks of 607 inspections, 60 aliases, malformed inputs, exact
limits/arena accounting and 54 recursive file trees. The 2026-10-11 verification
passes **981 Rust tests**, strict Clippy, formatting and prior language oracles.
Both independent code reviews are approved; the frozen ladder remains 16/16.
