# Slice, concatenate, reverse and sort lists

AIL actions and value functions can transform `List<Int>`, `List<Float>` and
`List<Text>` through checked Core, STE, the VM and native Linux x86-64 ELF.
Source and destination lists must have the same element type.

```text
- the system concatenates list data.left and list data.right into data.result
- the system slices list data.values from data.start to data.end into data.result
- the system reverses list data.values into data.result
- the system sorts list data.values in ascending order into data.result
- the system sorts list data.values in descending order into data.result
```

Functions use `the function` and declared `local.*` fields. Slice bounds accept
checked integer expressions. Sort requires an explicit `ascending` or `descending`
direction. All inputs are read before replacing a destination, so either concat
input and every other list source can also be the destination. Earlier copies
retain their values.

## Operation semantics

Concatenation places all items from the first list before all items from the
second. Reversal produces the opposite item order. Slicing includes the start
and excludes the end, using step 1. Negative bounds count from the end and all
bounds clamp to the list endpoints; an end at or before the start produces `[]`.
Both bound expressions evaluate exactly once, start first, even for empty slices.

Sorting is stable in both directions: equal values retain their original relative
order. Integers compare as signed numbers. Text compares lexicographically by
Unicode scalar values, with a prefix before its extension in ascending order.
NUL, control characters and combining sequences are preserved; no normalization
or locale-specific collation is performed.

Float sorts numerically, including infinities. Positive and negative zero compare
equal and preserve their relative order. NaNs remain stable and appear **after
all numbers in both directions**. This is AIL's explicit policy for unordered
values. It differs from applying ordinary Python comparisons to interspersed NaNs.
Canonical Float transport, including signed zero and `nan`, remains unchanged.

Clamped slicing and stable ordered-value sorting follow the corresponding
[Python sequence](https://docs.python.org/3/library/stdtypes.html#common-sequence-operations)
and [sorting](https://docs.python.org/3/howto/sorting.html) contracts.

## Validation, allocation and performance

Each input list is limited to **1,048,576 raw bytes** and **65,536 items**.
Results are limited to **1,048,576 canonical bytes** and **65,536 items**.
Borrowed inputs can have canonical representations above the output byte bound;
a slice that reduces such a list can succeed. Producing the whole oversized
canonical list through reverse, sort or concat fails.

Every successful operation charges its final canonical result length once to
the shared **67,108,864-byte List/Json arena**, including unchanged results and
empty `[]` (2 bytes). No Text allocation is charged. Scratch does not consume
either result arena. Function returns add the existing one-copy charge; overwrites
do not refund previous allocations. Failures preserve destination and counters.

Validation order is part of the contract:

1. Concat validates the complete first list, then the complete second list, then
   combined item/byte limits and allocation.
2. Slice validates the complete source, evaluates start then end, then checks
   result bounds and allocation.
3. Reverse and sort validate the complete source before result bounds/allocation.

Malformed lists report `AIL-RUNTIME-LIST`. Input/result bounds and allocation
failures report `AIL-RUNTIME-LIST-LIMIT`. Slice expressions retain their existing
integer errors; out-of-range slice bounds clamp instead of producing an index
error. Diagnostics do not reveal operand contents.

Native sorting parses once and performs stable mergesort on descriptors with
O(n log n) comparisons and O(n) temporary descriptor storage. Text comparisons
also cost the bytes examined. Other operations take linear work in input/output
size. Each creates one final canonical result. These operations avoid implementing
whole-list transformations as repeated immutable append/set operations.

## Core, STE and profiles

`ListOperation` adds `list_kind=concat|slice|reverse|sort` and retains
`element_type=Int|Float|Text`. Sort alone requires `sort_order=ascending|descending`.
`list_source` and `list_destination` name typed fields; concat adds `list_other`,
and slice adds `list_start`/`list_end` IntegerExpression roots. The checker enforces
exact metadata, unique attribute-free operands, genuine field declarations,
expression scope and one ordered executable owner. Secret reads include both
concat inputs and both bound expressions, even without descriptive access edges.
Imports qualify fields and expressions. Functions use the typed local schema.

| Opcode | STE template |
| --- | --- |
| `LIST_CONCAT` | `Concatenate {type} list {source} and list {other} into field {key}.` |
| `LIST_SLICE` | `Slice {type} list {source} from integer {start} to integer {end} into field {key}.` |
| `LIST_REVERSE` | `Reverse {type} list {source} into field {key}.` |
| `LIST_SORT` | `Sort {type} list {source} in {order} order into field {key}.` |

`type` names the element type; bounds use Int postfix and `order` is an exact
direction. Separately declared technical data carries all operands. These
operations require Application or C interop. Descriptive functions and compiler
passes reject them. Static errors use `AIL-LIST-001`; protocol errors use
`AILBC013`. Action traces retain `computed list <destination>`; local computations
add no implicit trace.

## File statistics example

[List Statistics](../../examples/list-statistics) reads two JSON measurement
batches, merges and sorts them, calculates their median and selects a highest-value
window through reusable functions. It writes an actual JSON report. Empty batches
produce a null median and empty lists.

```bash
cargo run -- ail-run examples/list-statistics --action Analyze \
  --read-file=input=examples/list-statistics/input.json \
  --write-file=report=/tmp/list-statistics.json
```

See its [exported STE](../../examples/ste/list-statistics.ail-ste). After
`cargo build`, run `python3 scripts/ail_list_transform_differential.py` for
independent expectations across types, aliases, limits and real report files.
The [implementation record](../superpowers/plans/2026-10-11-list-transformations.md)
tracks verification and reviews. General Python replacement remains the active
objective, including broader types, effects, libraries and agent workflows.
