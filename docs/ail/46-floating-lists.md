# Floating lists

`List<Float>` supports creation, count, get, append and replacement through
checked source, Core, STE, the VM and standalone Linux x86-64 executables.
It extends the [existing lists](38-typed-lists.md) with the full
[binary64 Float](45-float-computation.md) value range.

[List transformations](50-list-transformations.md) add concat, slice, reverse and
stable sort. Float sorting preserves signed-zero order and puts NaNs last in both
ascending and descending results.

```text
A Data has:
- values: List<Float>
- item: Float
- index: Int

An Output has:
- values: List<Float>
- count: Int
- item: Float

Action: Transform.
When Transform happens:
- the system creates an empty list into output.values
- the system appends floating data.item / 2 to list output.values into output.values
- the system sets item -1 in list output.values to floating -data.item into output.values
- the system counts items in list data.values into output.count
- the system gets item data.index from list data.values into output.item
- the system returns output.values
- the system returns output.item
```

Indices are checked Int expressions, including negative indices counted from
the end. Append/set use `floating` followed by a Float expression; its fields
must be Float. Count returns Int and get returns Float. Source and destination
list types must match, with no implicit element conversions.

## Runtime values and JSON

Runtime lists contain Float tokens inside comma-separated brackets. ASCII JSON
whitespace is allowed around tokens and separators. Each token uses the scalar
Float grammar, so `[+001,.5,1.,-0,nan,+inf,-inf]` is valid runtime input.
Canonical results are `[1e0,5e-1,1e0,-0e0,nan,inf,-inf]`. Signed zero,
subnormals and nonfinite values survive get, append/set and function returns.
Reading does not rewrite the original transport. Strings, booleans, null,
nested values, missing items, trailing commas and extra data are rejected.

Use explicit JSON conversion for files and interoperability:

| Boundary | Accepted values |
| --- | --- |
| Runtime list input/function parameter | All scalar Float transports, including infinity/NaN |
| Literal assignment and JSON parse/get | Flat arrays of strict JSON numbers which round to finite binary64 |
| JSON format/set | Runtime list grammar with only finite values |

For example, `the system assigns [1.25,-0,5e-324] to data.values` creates a
finite floating list. Literal/JSON input rejects leading plus signs, leading
zeros, `.5`, `1.`, special tokens and overflow to infinity. JSON output uses
canonical scientific number tokens. General Json fields keep their original
number spellings; only typed Float conversion rounds numbers.

## Functions, limits and errors

Value functions declare List<Float> parameters, locals and results and use the
same statements with `the function`. Arguments/returns require bare declared
fields. Locals are isolated; list parameters are validated in sorted parameter
order before checking call depth, including unused parameters. Parameters borrow
immutable input, and successful list returns create one owned canonical copy.

The existing limits apply: **65,536 items**, **1,048,576 input/result bytes**,
and a shared **67,108,864-byte list/Json result arena**. Empty creation charges
2 bytes; append/set and function returns charge their final canonical length.
Count and Float get have no arena charge. Parameters, count and get accept raw
input whose canonical form would exceed the result bound; producing or returning
an oversized canonical list fails. JSON conversions retain their existing
Text/list allocation rules. Direct flat JSON conversion allows all 65,536 list
items; lookup from Json still validates the source document's total-value limit.

The whole source is validated before the index expression and range, then the
replacement expression, then result bounds and allocation. Invalid source uses
LIST/LIST-LIMIT; bad indices use INT/INDEX; floating expressions retain
FLOAT/FLOAT-LIMIT/DIVZERO. JSON boundaries use JSON/JSON-LIMIT. Failed operations
preserve destination and accounting. Aliased inputs/outputs and prior copies
retain immutable values. Repeated appends still copy and reparse bounded lists,
so they can take quadratic work; this is not a numerical array performance claim.

## Core and STE

Core uses `ListOperation` with `element_type=Float`. Action `list_value` edges
bind checked FloatExpression trees; local operations use typed expression roles.
Indices bind IntegerExpression trees. Checks enforce exact types/metadata,
operand ownership, secret reads/writes, and imported field qualification.
Only Application and C interop profiles execute these operations.

The existing LIST_EMPTY/LENGTH/GET/APPEND/SET opcodes and
[STE templates](33-ste-intermediate.md) now accept `type=Float`. Indices use Int
postfix; append/set values use Float postfix. Native descriptors store binary64
bits and use the embedded numeric helper for exact parsing/formatting, retaining
the same field layout and allocation rules as existing lists.

## Runnable statistics report

The [floating-lists example](../../examples/floating-lists) reads a JSON dataset,
calls reusable mean/centering functions, and writes centered values and population
variance alongside the original data. Its four sample values produce mean
`3.5e0` and variance `2.875e0`. The [exported STE](../../examples/ste/floating-lists.ail-ste)
uses the same CLI commands as the source package.

The [implementation record](../superpowers/plans/2026-10-11-floating-lists.md)
tracks tests and reviews. Broader collections, libraries and full Python
replacement remain on the active language-development objective.

After `cargo build`, run `python3 scripts/ail_float_list_differential.py` for the
independent Python oracle: 650 operation pipelines, 672 grammar cases, exact
size/allocation limits and 36 file reports. The 2026-10-11 verification passes
**957 Rust tests**, strict Clippy, formatting and prior language oracles.
The frozen ladder remains 16/16; both independent code reviews are approved.
