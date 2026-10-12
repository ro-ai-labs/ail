# Integer computation and returned fields

AIL executes checked integer expressions through the complete pipeline:

```text
AIL-Spec -> typed AIL-Core -> AIL-STE -> verified bytecode -> VM / native ELF
```

## Source syntax

Declare input and result fields as `Int`, then compute with dotted field names:

```text
The application Arithmetic manages integer results.

A Sum has:

- a: Int
- b: Int
- total: Int
- label: Text

Action: add numbers.

When add numbers happens:

- the system computes sum.total as (sum.a + sum.b) * 3
- the system computes sum.total as sum.total - 1
- the system returns sum.total
- the system returns sum.label
- the system records a trace event named Calculated
```

`computes` requires a declared `Int` destination and declared `Int` operands.
Numeric-looking `Text` does not qualify. Dotted identifiers have at least two
ASCII identifier segments. Each segment begins with a letter or underscore.

The grammar supports decimal integer literals, field references, parentheses,
unary minus, and binary `+`, `-`, `*`, `/`, `%`. Multiplication, division and
remainder bind before addition and subtraction. Binary operators associate to
the left. Nonliteral `increments ... by expression` and `decrements ... by
expression` use the same expression machinery; literal increments retain their
existing bytecode instruction and must target `Int` fields.

Writes, calls, and repeated calls execute in source order. Calls can refer to
actions declared later. Requirements and explicit reads precede that operation
sequence; guarantees and trace declarations follow it. These remain action
contracts, rather than arbitrary statement positions. `returns` declares a
result field and does not exit early.

## Integer and output semantics

`Int` in this implementation is signed 64-bit, from −9223372036854775808 through
9223372036854775807. Arithmetic checks overflow. Division truncates toward zero;
remainder has the dividend's sign. Both minimum-Int divided by −1 and
minimum-Int remainder −1 report overflow. A zero divisor reports an error.
These division and width rules differ from Python's integer semantics.

CLI values use string transport. At an integer read, the runtime accepts an
optional sign and ASCII decimal digits, with no surrounding whitespace or junk.
An integer result is canonical decimal: `007 + -0` becomes `7`. Reading an
integer does not rewrite its input representation. Text returned as `007`
remains `007`.

`the system returns field.name` selects a field for CLI/native stdout. Multiple
returns select multiple fields. Missing selected fields fail the action before
result stdout. Without selection, the legacy complete-state output remains.
The library run result retains `final_state` and separately exposes
`returned_fields` and `output_state()`. A called action validates its own
returned fields, but its selection does not change the caller's selection.
Native default full-state output orders known keys before unknown input keys;
consumers should compare mappings, not line order. Native arithmetic now prints
the final state once; intermediate updates remain in the stderr trace.

Error categories include `AIL-RUNTIME-INT`, `AIL-RUNTIME-OVERFLOW`,
`AIL-RUNTIME-DIVZERO`, and `AIL-RUNTIME-OUTPUT`. Native failures use a nonzero
exit code and stderr. Traces are separate from native result stdout.

## Core and STE representation

Core stores `IntegerExpression` nodes typed `Int`, with `expression_kind`
`literal`, `field`, or `binary`. Literals carry `value`; binary nodes carry
`operator`. `left_operand` and `right_operand` connect binary operands;
`expression_field` connects an operand to its declared field. A write edge's
`expression` attribute names the root using its stable `IntegerExpression:name`
label. Editing an operator changes execution; editing provenance does not.
The checker rejects invalid types, missing operands, cycles and excessive depth.

Writes, calls and repeats carry unique contiguous `step_index` attributes per
action. Separate `write_index`/`operation_index` values distinguish repeated
edges. Saving Core retains every operation. Older Core without step indices
keeps its historical calls/repeats/writes order. Expression limits are 65,536
bytes, 1,024 nodes and 128 levels, including reconstructed Core expressions.

STE has two additional templates:

```text
Evaluate integer expression expression_data into field result_data.
Select output field result_data.
```

Both operands are declared technical-data identifiers. The expression data uses
validated postfix tokens: `i:42` for literals, `f:sum.a` for fields, and binary
operator tokens. `f:sum.a f:sum.b +` becomes `EVAL_INT`; output selection becomes
`OUTPUT_FIELD`. Core's source type checks are not available to an independently
authored STE program, so authors must use integer operands deliberately.

Programs containing expressions, output selection, or literal increments use
the native scalar backend. It emits x86-64 machine code directly into ELF. It reads
mutable field storage, checks integer input and arithmetic, and preserves copy
values across later writes. There is no Python or C runtime dependency. It
supports scalar requirements, field equality branches and jumps; unsupported
mixed opcodes cause compilation errors.

## Run an example

```bash
cargo run -- ail-run bench/R001-add/program.ail --action AddNumbers sum.a=7 sum.b=9
cargo run -- ail-ste bench/R001-add/program.ail > /tmp/add.ail-ste
cargo run -- ail-compile /tmp/add.ail-ste --action AddNumbers \
  --target linux-x86_64-elf --out /tmp/ail-add
/tmp/ail-add sum.a=7 sum.b=9
python3 scripts/ail_bench.py
```

A ready-to-run STE example is [add.ail-ste](../../examples/ste/add.ail-ste).

The frozen ladder now passes 16/16 rungs. In addition to integer computation,
[comparisons and control flow](35-control-flow.md) support maximum, absolute
value and sign, sum-to-n, GCD, Collatz and primality. [Integer functions](36-integer-functions.md)
also support recursive factorial and Fibonacci. Collections, arbitrary-precision integers,
general I/O and Python compatibility also remain open. STE remains the limited
programming profile described in [the STE reference](33-ste-intermediate.md).
