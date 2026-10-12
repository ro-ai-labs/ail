# Typed literal assignments

Use `the system assigns <JSON literal> to <field>` to create values directly in
source. The destination's declared type determines the accepted literal.

| Destination type | Example |
| --- | --- |
| `Int` | `the system assigns -42 to data.number` |
| `Float` | `the system assigns 1.25e2 to data.measurement` |
| `Bool` | `the system assigns true to output.ready` |
| `Text` | `the system assigns "Hello, world!\n🙂" to output.message` |
| `List<Int>` | `the system assigns [1, -2, 3] to data.numbers` |
| `List<Float>` | `the system assigns [1.25, -0, 5e-324] to data.samples` |
| `List<Text>` | `the system assigns ["é", "\ud83d\ude42"] to data.names` |
| `Json` | `the system assigns {"items":[1,null,{"ok":true}]} to data.document` |

Empty text is `""`; an empty list is `[]`, with its element type inferred from
the destination. Fields must be declared before execution. These statements
execute in order, including inside branches, loops and called/imported actions.
Copies retain their previous value when a field is assigned again.

## Literal grammar

Values use strict JSON syntax, with ASCII JSON whitespace around them. `Int` values
must fit signed 64-bit range. Leading plus signs, leading zeros, decimal fractions
and exponents are rejected. Booleans are exactly `true` or `false`. Text must be
quoted and supports standard JSON escapes, including `\n`, `\t`, `\u0000`
and paired surrogate escapes for non-BMP characters. Raw line breaks inside a
string, invalid escapes and lone surrogates are errors.

Float accepts JSON numbers that convert to finite binary64, including fractions
and exponents. It preserves signed zero; overflow to infinity is rejected. See
[floating-point computation](45-float-computation.md).

Typed lists are flat and homogeneous. Use [Json](42-recursive-json.md) for mixed
or nested arrays, objects, null and lossless number tokens. The checker rejects mismatches such as assigning `"42"` to an
`Int` field. There is no implicit conversion.

Literal text is opaque data. A string containing `to output.message` or an
instruction cannot introduce another statement. Imports qualify destination
fields while preserving literal text. Source rendering canonicalizes JSON
escaping and spacing without changing the value. Runtime text is decoded;
runtime lists use the [canonical list JSON representation](38-typed-lists.md).

This explicit statement coexists with older prose assignment forms. Use it for
arbitrary text and lists; legacy prose forms retain their previous semantics.

## Validation and limits

Supported destination types are `Int`, `Float`, `Bool`, `Text`, `Bytes`, `List<Int>`,
`List<Float>`, `List<Text>` and `Json`. Literal assignments execute in Application and C interop actions.
Other profiles and compiler-pass actions reject them.
[Value functions](44-value-functions.md) use `the function assigns` for declared
local fields; literals retain the same limits and remain free of arena charges.

Text constants can contain at most **1,048,576 decoded UTF-8 bytes**. Lists can
contain at most **65,536 items**, and both their input JSON and canonical JSON
must fit **1,048,576 bytes**. Raw literal syntax is bounded to **6,291,458 bytes**
to accommodate a maximum-size text constant written with six-byte escapes.
Malformed values, wrong types and exceeded limits fail validation with
`AIL-LITERAL-001` before execution.

Json constants additionally obey the recursive value bounds: 1 MiB raw and
encoded bytes, 64 open containers and 65,536 total values including the root.

Constants are immutable program data. Assigning one consumes no runtime text
or list result budget, including when repeated in a loop. Subsequent operations
such as text join, list append and extracting text from a list retain their
existing allocation charges.

## Core and STE

`LiteralAssignment` nodes carry `value_type` and `literal` attributes.
`literal_step` orders them within an action; `literal_destination` references
the declared global field. The checker validates exact attributes, values,
ownership, operand multiplicity, types, order and supported profiles. Secret
writes derive from the destination edge and require protection even if a saved
Core artifact omits descriptive write edges.

The saved-Core parser treats quotes, escapes, brackets and commas in literal
attributes as JSON data. Duplicate attribute keys are rejected. Historical
unquoted prose attributes retain their existing parsing rules.

The STE template is:

```text
Assign literal value_data of type type_data to field destination_data.
```

It lowers to `ASSIGN_LITERAL` with exactly `key`, `type` and `value` operands.
The value operand contains JSON literal syntax, including quotes for text. For
example, a technical-data declaration for the text `hello` is
`Data greeting = "\"hello\""`. Direct STE validates literal syntax and types;
the source/Core path additionally checks destination declarations and contracts.

## Run the example

The [typed-constants package](../../examples/typed-constants) constructs its own
input data, joins Unicode text, and squares/sums a list with a function and loop.
It needs no runtime arguments.

```bash
cargo run -- ail-run examples/typed-constants --action Demo
cargo run -- ail-compile examples/typed-constants --action Demo \
  --target linux-x86_64-elf --out /tmp/ail-constants
/tmp/ail-constants
```

Results are `output.message=Hello, world! 🙂`, `output.ready=true`,
`output.squares=[1,4,9]` and `output.total=14`. Its
[exported STE](../../examples/ste/typed-constants.ail-ste) can also run and compile
directly. General I/O, richer collection/function types, arbitrary precision and
broader Python compatibility remain part of the active goal.

After `cargo build`, run `python3 scripts/ail_literal_differential.py` for the
independent fixed-seed checks: 500 typed values with copies and reassignment,
50 source values through saved Core/rendered source/STE/VM/native, 85 malformed
cases and 13 exact/overflow boundaries. The probes include NUL, escaped Unicode,
source keywords and text that resembles Core attribute syntax.

This milestone passes 684 Rust tests and retains the unchanged 16/16 numeric
capability ladder. These checks verify the documented literal behavior; broader
Python replacement remains the active goal.

## Bytes literals

[Bytes](56-bytes.md) adds JSON-string literals containing canonical lowercase
`hex:` transport. Destination typing distinguishes Bytes from Text. Empty is
`"hex:"`; at most 1 MiB raw bytes are allowed, so valid Bytes literals can
exceed Text’s decoded size limit. The existing raw-literal bound remains.
