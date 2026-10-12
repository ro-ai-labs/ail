# Executable typed lists

The active objective remains an agent-usable Python replacement with STE between
specifications and binaries. Lists make batch processing and loops over runtime
data possible. This architectural increment adds `List<Int>` and `List<Text>`
to executable actions; it does not redefine the full objective as a scalar or
small-list language. General nested/dynamic collections remain later work.

## Source and values

```text
- the system creates an empty list into data.numbers
- the system appends integer input.value + 1 to list data.numbers into data.numbers
- the system appends text input.name to list data.names into data.names
- the system counts items in list data.numbers into output.count
- the system gets item input.index from list data.numbers into output.value
- the system sets item input.index in list data.numbers to integer input.value into data.numbers
- the system sets item input.index in list data.names to text input.name into data.names
```

Empty-list type comes from its destination. Other operations take a declared
list source; append/set destinations have the same list type. Length returns
`Int`; get returns the element type. Integer values and indices are checked
integer expressions. Text values name declared `Text` fields. Inputs and
outputs are flat JSON arrays, with JSON strings for text and strict JSON integer
tokens for Int (no leading plus/zero, fractional or exponent forms). Every input
element is validated even when only one is read. Reject wrong types and nested
containers without recursively parsing them. Text accepts UTF-8 and standard
JSON escapes including valid surrogate pairs. No implicit type coercion.

Indices are zero-based; negative indices count from the end. Out-of-range
indices, including any index into an empty list, fail with `AIL-RUNTIME-INDEX`.
Get/set do not clamp or extend. List inputs are validated first; then indices
are evaluated and range-checked; then an append/set value is evaluated. All
operands are read before the destination changes. Empty creation does not read
the old destination. Failed operations do not mutate destination or budgets.
Copies have value semantics: updating one list field does not mutate a copy.

Missing/malformed/wrong-element list input reports `AIL-RUNTIME-LIST`; exceeded
list limits report `AIL-RUNTIME-LIST-LIMIT`. Existing integer/text errors apply
to item values and indices. Native failures have no result stdout. Results use
compact canonical JSON: canonical i64 decimal, no spacing, and text quoted by
the existing `core_model::json_string` convention (quote/backslash/n/r/t short
escapes; other U+0000..001F and U+007F..009F as lowercase `\uXXXX`; other Unicode
as raw UTF-8). Canonicalization occurs on list-producing operations; reading an
input does not rewrite the source transport value.

## Limits and representation

Maximum 65,536 items and 1,048,576 encoded JSON bytes per input/result list.
Maximum decoded text element 1,048,576 bytes. List-producing operations charge
their canonical encoded byte length against a separate 67,108,864-byte list
result budget shared by root and nested/repeated action calls. Empty lists cost
2 bytes. Length/get Int do not charge a list budget; get Text charges the existing
text budget by max(decoded bytes,1). Native immutable arena storage preserves
copies; scratch parsing buffers are bounded and reclaimed. Text and list
result budgets stay separate and are never refunded during a root invocation.

`AilAction.list_operations` and `AilActionStep::List(index)` preserve ordering.
`AilListOperation` variants: Empty, Length, Get, Append, Set, Invalid; a value is
`AilListValue::Integer(IntExpr)` or `Text(String field)`. Lists are action values;
existing integer function signatures stay integer-only.

Core `ListOperation` nodes have `list_kind=empty|length|get|append|set` and
`element_type=Int|Text`. Ordered `list_step` edges connect executable actions.
`list_source` and `list_destination` reference global fields; `list_index`
references an Int expression, and `list_value` references an Int expression or
Text field according to element type. Check exact edge multiplicity, global
ownership, types, ordering, profile, and node attributes. Derive secret accesses
from executable operands, including expression leaves; saved Core must not
depend on optional descriptive reads/writes for access checks. Imports qualify
field references and integer expressions. Application/C interop lower lists;
other profiles reject executable list operations.

Bytecode exact operands:

- LIST_EMPTY: key, type
- LIST_LENGTH: key, source, type
- LIST_GET: key, source, type, index
- LIST_APPEND: key, source, type, value
- LIST_SET: key, source, type, index, value

`type` is `Int` or `Text`. Index and Int value operands are postfix expressions;
Text value is a field key. Exact STE templates bind technical-data IDs. The
normal checked Core -> STE -> verified bytecode path remains authoritative.

## Verification

Real list sum/map-style loops, integer functions operating on fetched items,
Unicode text lists, copies and in-place destinations; first/last/negative/extreme
indices; input whitespace, escapes, malformed numbers/strings and mixed/nested
types; source/Core/STE/import round trips; unsupported profiles and ownership;
secret reads/writes without descriptive edges; size/count/shared-budget failures;
native strict UTF-8 and atomic failure. Compare with independent Python list
oracles, retain the frozen 16/16 ladder, and run the complete Rust suite.
