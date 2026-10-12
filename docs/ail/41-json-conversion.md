# Typed JSON conversion

Use explicit JSON parsing and formatting to connect text files with typed AIL
computations:

```text
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.values
- the system formats data.values as JSON text into output.text
- the system writes text output.text to file resource output
```

Parsing takes a global Text field and uses the destination's declared type.
Formatting uses the source's declared type and writes a global Text field.
Supported value types are `Int`, `Float`, `Bool`, `Text`, `List<Int>`, `List<Float>`, `List<Text>` and `Json`.
No implicit conversion is performed. Both operations execute in source order,
including branches, loops and called actions. In-place Text conversion is safe,
and copied values retain their contents after the source field changes.

## Values and syntax

| Type | JSON input | Parsed runtime value | Formatted JSON |
| --- | --- | --- | --- |
| `Int` | `-42` | `-42` | `-42` |
| `Float` | `1.25` | `1.25e0` | `1.25e0` |
| `Bool` | `true` | `true` | `true` |
| `Text` | `"hello\n🙂"` | Text containing a newline and emoji | `"hello\n🙂"` |
| `List<Int>` | `[1, -2, 3]` | `[1,-2,3]` | `[1,-2,3]` |
| `List<Float>` | `[1.25, -0]` | `[1.25e0,-0e0]` | `[1.25e0,-0e0]` |
| `List<Text>` | `["a", "\u00e9"]` | `["a","é"]` | `["a","é"]` |
| `Json` | `{"items":[1,null]}` | `{"items":[1,null]}` | `{"items":[1,null]}` |

The accepted JSON syntax follows the relevant number, string and array grammar
in [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259). AIL's supported destination
types restrict the accepted values. Use [Json](42-recursive-json.md) for objects,
null, nested or heterogeneous arrays and lossless fractional/exponent numbers.

Parsing permits only JSON's ASCII space, tab, CR and LF around and between
tokens. It rejects comments, trailing commas, trailing values, malformed UTF-8,
invalid escapes, lone UTF-16 surrogates, mismatched types and out-of-range i64
`Int` values. JSON integers cannot have a leading plus sign or leading zeros;
an Int destination converts `-0` to `0`. Json preserves number tokens. A JSON string is required for a Text destination: unquoted
`hello` is an error. Valid paired surrogate escapes decode to their Unicode
scalar value. Decoded text may contain NUL.

Formatting creates compact JSON with no separator spaces or trailing newline.
Strings quote double quotes/backslashes, use `\n`, `\r`, `\t`, and encode other
Unicode control characters as lowercase `\uXXXX`; other characters remain
UTF-8. This is the same canonical escaping used by AIL list values and literals.
Booleans must have exactly `true` or `false` runtime transport. Int formatting
accepts the same transport as integer arithmetic (`+001` becomes `1`, with no
whitespace or out-of-range values). Lists are fully validated and canonicalized.
Float parsing rounds strict JSON numbers to binary64, preserving signed zero.
Parsing and formatting require a finite result; NaN and infinities are rejected.
See [floating-point computation](45-float-computation.md) and
[floating lists](46-floating-lists.md). List<Float> parse/get requires strict
finite JSON numbers; format/set accepts runtime Float tokens but rejects
infinity and NaN. Direct flat conversion permits all 65,536 items.

## Limits, errors and allocation

Both the input operand and produced value are bounded to **1,048,576 UTF-8
bytes**. Lists contain at most **65,536 items**, and their canonical encoding
must fit the byte limit. Text escaping can expand a value, so a valid source
Text can still produce an oversized JSON result.

| Operation | Shared result allocation charged |
| --- | --- |
| Parse Int, Float or Bool | None |
| Parse Text | `max(decoded bytes, 1)` to text budget |
| Parse a list | Canonical list bytes to list budget |
| Parse Json | Encoded Json bytes to list budget |
| Format any supported value | Encoded JSON bytes to text budget |

The text and list budgets are each **64 MiB per root execution**, shared through
action calls and with the existing operations. Scratch storage does not consume
these semantic budgets. Formatting a list does not allocate an intermediate
list result. Parsing/formatting never changes the destination on failure.

`AIL-JSON-001` diagnoses unsupported source/Core/protocol types or malformed
operations. `AIL-RUNTIME-JSON` diagnoses missing or malformed runtime values,
invalid UTF-8/JSON, type mismatch and integer overflow. `AIL-RUNTIME-JSON-LIMIT`
diagnoses input/result byte limits, list item limits and exhausted result budgets.
Native scratch allocation failure also uses JSON-LIMIT; physical host memory
exhaustion is not a portable, recoverable VM error guarantee.

An oversized input fails before grammar validation. For bounded input, the
complete value is validated before encoding size and allocation checks. Successful
conversion traces contain `parsed JSON <field>` or `formatted JSON <field>`;
conversion diagnostics and traces omit input values.

## Core and STE

A `JsonConversion` node has exactly `json_kind=parse|format` and `value_type`
attributes, and no node type. A `json_step` edge gives it exactly one executable
Action owner and a source-order index. `json_source` and `json_destination`
reference the typed global fields. Derived reads/writes participate in secret
checks even when descriptive edges are absent. Imports qualify field names.

The STE templates are:

```text
Parse JSON text source_data as type type_data into field destination_data.
Format type_data field source_data as JSON text into field destination_data.
```

They lower to `JSON_PARSE` and `JSON_FORMAT`, each with exactly `key`, `source`
and `type` operands. Type names and field references use technical data
declarations. Core and direct bytecode/STE enforce Application or C interop
profiles. Compiler-pass bodies reject conversions.
[Value functions](44-value-functions.md) support conversion of declared locals
using `the function` forms.

## File computation example

The [JSON batch package](../../examples/json-batch) reads an integer array,
squares each item using a function and loop, and writes the squared list and
sum as JSON files:

```bash
cargo run -- ail-run examples/json-batch --action Process \
  --read-file=input=examples/json-batch/input.json \
  --write-file=squares=/tmp/ail-squares.json \
  --write-file=total=/tmp/ail-total.json

cargo run -- ail-compile examples/json-batch --action Process \
  --target linux-x86_64-elf --out /tmp/ail-json-batch
/tmp/ail-json-batch \
  --read-file=input=examples/json-batch/input.json \
  --write-file=squares=/tmp/ail-native-squares.json \
  --write-file=total=/tmp/ail-native-total.json
```

For `[1,-2,3,12]`, the files contain `[1,4,9,144]` and `158`, with no trailing
newline. The selected state reports `output.count=4` and `output.total=158`.
[Exported STE](../../examples/ste/json-batch.ail-ste) can run or compile directly.

## Verification and remaining scope

The 2026-10-11 implementation passes all **752 Rust tests**, strict Clippy and
formatting checks. After `cargo build`, run
`python3 scripts/ail_json_differential.py` to reproduce 500 Python/VM/native
round trips, 132 malformed conversions, 60 file datasets, 13 exact/overflow
boundaries and shared-allocation checks. Source, saved Core and direct STE
each produce the example's exact files on the VM and native ELF runtime.
Existing text, list, literal and file differential checks also pass; the
frozen benchmark remains 16/16.

[Recursive Json values](42-recursive-json.md) extend this codec with nested
containers and typed lookup. [Float computation](45-float-computation.md) and
[value functions](44-value-functions.md) extend typed numeric and reusable
operations; wider Python compatibility remains part of the active goal.
