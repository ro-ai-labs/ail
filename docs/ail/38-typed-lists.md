# Typed lists

[List transformations](50-list-transformations.md) add whole-list slicing,
concatenation, reversal and stable sorting for all three element types.

AIL actions can create, count, index, append and replace items in `List<Int>`
and `List<Text>` values; [floating lists](46-floating-lists.md) extend the same
operations to `List<Float>`. These operations execute through checked Core, STE,
the bytecode VM and native Linux x86-64 ELF. They compose with branches, loops,
text operations and integer function calls.

```text
An Input has:
- values: List<Int>
- index: Int

An Output has:
- values: List<Int>
- count: Int
- item: Int

Action: Process.
When Process happens:
- the system creates an empty list into output.values
- the system counts items in list input.values into output.count
- the system gets item input.index from list input.values into output.item
- the system appends integer output.item + 1 to list output.values into output.values
- the system sets item -1 in list output.values to integer 42 into output.values
- the system returns output.values
- the system records a trace event named Processed
```

For text lists, use `appends text input.name` and
`sets item input.index in list input.names to text input.name`.
Text values come from `Text` fields; integer values and indices accept checked
integer expressions. Empty-list creation infers the element type from its
destination. Other operations infer it from the source list. Count destinations
are `Int`, get destinations match the element type, and append/set destinations
match the source list type.

## Values and transport

Inputs use flat JSON arrays, for example `[1,-2,3]` and `["é","🙂"]`.
Integer items must be signed 64-bit JSON integers. Fractions, exponents, leading
zeros, booleans, mixed types and nested arrays are rejected. Text items accept
valid UTF-8 and standard JSON string escapes, including paired UTF-16 surrogate
escapes. Unpaired surrogates and malformed UTF-8 are errors.

Indices start at zero. Negative indices count from the end: -1 selects the last
item. Out-of-range indices fail; indexing an empty list always fails. The whole
source list is validated before the index and value expressions are evaluated.
Set checks the index range before evaluating the replacement value.

List operations produce immutable values. Appending or replacing into the source
field is safe; a previous copy retains its contents. Producers write compact JSON
with decimal integers. Text uses short escapes for quotes, backslashes, newline,
carriage return and tab; other U+0000–U+001F and U+007F–U+009F characters use
lowercase `\uXXXX` escapes. Other Unicode characters remain UTF-8. Reading a list
does not rewrite its original representation. Get returns the decoded element,
so a text item containing `\n` becomes text containing a newline.

The current runtime parses the source on each operation and copies produced
lists. Repeated appends can therefore require quadratic work. This is a bounded
value implementation, not a claim of Python collection performance.

## Limits and errors

- Maximum items per list: **65,536**.
- Maximum input or produced JSON value: **1,048,576 UTF-8 bytes**.
- Maximum decoded text item: **1,048,576 UTF-8 bytes**.
- Shared list-result allocation budget: **67,108,864 bytes** per root action.

Empty creation charges 2 bytes. Append and set charge the canonical result's
encoded length. The budget is shared across nested and repeated action calls;
overwriting a field does not refund it. Length and integer get do not consume
list-result budget. Text get charges `max(decoded_bytes, 1)` against the separate
[text-result budget](37-text-operations.md). Failed operations leave the
destination and allocation accounting unchanged.

Input validation precedes index evaluation and bounds checking, then value
evaluation, then result limits. An input can fit its size limit while its
canonical representation would exceed it. Length/get still accept such an
input; append/set check the size of their actual produced result.

Missing, malformed or wrongly typed lists report `AIL-RUNTIME-LIST`.
Item, encoded-size or allocation limits report `AIL-RUNTIME-LIST-LIMIT`.
Out-of-range indices report `AIL-RUNTIME-INDEX`. Index/value expressions retain
the integer and text runtime errors. Native failures produce no result stdout.

## Core and STE

`ListOperation` nodes carry `list_kind=empty|length|get|append|set` and
`element_type=Int|Text`. Ordered `list_step` edges associate them with actions.
`list_source` and `list_destination` reference typed global fields;
`list_index` references an `IntegerExpression`; `list_value` references an
integer expression or a text field. The checker enforces exact attributes,
operand multiplicities, ownership, types, execution order and profile support.
Secret-access rules derive reads/writes from these operands, including integer
expression leaves, even when saved Core omits descriptive access edges.

The exact STE templates are:

```text
Create empty list of type type_data into field destination_data.
Count items in type_data list source_data into field destination_data.
Get item index_data from type_data list source_data into field destination_data.
Append value value_data to type_data list source_data into field destination_data.
Set item index_data in type_data list source_data to value value_data into field destination_data.
```

`type_data` resolves to `Int` or `Text`. Indices and integer values use postfix
integer expressions; text values refer to field keys. The opcodes are
`LIST_EMPTY`, `LIST_LENGTH`, `LIST_GET`, `LIST_APPEND` and `LIST_SET`.
Bytecode verification requires exact operands and valid field/expression syntax.
Direct STE does not provide the specification's Core contract checks.

Lists execute in actions in the Application and C interop profiles. Executable
functions can use the local forms described in [value functions](44-value-functions.md).
Other profiles reject these operations.
Use [typed literal assignments](39-typed-literals.md) to construct list constants
in source. [Recursive Json](42-recursive-json.md) represents nested arrays and
objects with typed path lookup. [JSON mutation](43-json-mutation.md) supports
record updates and nested array append. General nested list operations remain
future work. List-valued arguments and returns are supported by
[value functions](44-value-functions.md).

## Run the examples

The [typed-lists package](../../examples/typed-lists) squares and sums integer
items using a function inside a loop, and edits Unicode text lists.
Its [exported STE](../../examples/ste/typed-lists.ail-ste) runs directly too.

```bash
cargo run -- ail-run examples/typed-lists --action Numbers 'input.values=[1,-2,3]'
cargo run -- ail-run examples/typed-lists --action Names \
  'input.names=["é","\ud83d\ude42","line\nnext"]' \
  input.index=-2 input.replacement=new
cargo run -- ail-compile examples/typed-lists --all-actions \
  --target linux-x86_64-elf --artifact-dir /tmp/ail-lists
/tmp/ail-lists/target-Numbers.elf 'input.values=[1,-2,3]'
```

`Numbers` returns count 3, total 14 and values `[1,4,9]`. `Names` returns count 3,
name `🙂` and names `["é","🙂","line\nnext","🙂"]`.

After `cargo build`, run `python3 scripts/ail_list_differential.py` for the
fixed-seed independent Python comparison: 600 list pipelines and 1,600 grammar
mutations, plus malformed-input, index and UTF-8 rejection checks. This milestone
passes 661 Rust tests and retains the unchanged 16/16 numeric ladder. These checks
establish the listed behavior; broader Python replacement remains the active goal.
