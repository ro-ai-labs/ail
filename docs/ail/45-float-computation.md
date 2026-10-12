# Floating-point computation

`Float` is IEEE binary64 through checked source, Core, STE, the VM and standalone
Linux x86-64 executables. It supports fractions, scientific notation, signed
zero, subnormals, infinity and NaN. Integer computations keep checked i64
semantics; conversions between Int and Float are explicit.

```text
A Data has:
- total: Float
- count: Int
- countFloat: Float

An Output has:
- mean: Float
- whole: Int

Action: Average.
When Average happens:
- the system converts integer data.count to floating data.countFloat
- the system computes floating output.mean as data.total / data.countFloat
- the system converts floating output.mean to integer output.whole
- the system returns output.mean
```

## Expressions and control

Use `the system computes floating <field> as <expression>`. Expressions support
`+`, `-`, `*`, `/`, `%`, unary signs and parentheses, with normal arithmetic
precedence. All referenced fields must be declared Float. Numeric literals in a
floating expression are Float, including tokens such as `2`. No implicit
conversion permits an Int field in that expression.

```text
- if floating data.value < 0:
- the system computes floating data.value as -data.value
- otherwise:
- the system computes floating data.value as data.value / 2
- end if
- while floating data.value >= 1:
- the system computes floating data.value as data.value / 2
- end while
```

Floating comparisons support `==`, `!=`, `<`, `<=`, `>`, `>=`. NaN is unequal
to every value including itself; all other comparisons with NaN are false.
Signed zeros compare equal. Mixed integer and floating controls support existing
`break` and `continue` statements and retain instruction-fuel limits.

Arithmetic rounds to binary64; overflow can produce infinity and underflow can
produce a signed zero. Each operation rounds separately. Division or modulo by
either signed zero fails with `AIL-RUNTIME-DIVZERO`. Modulo follows Python's
divisor-sign rule, including signed-zero results. These are binary floating
values, so decimal fractions need not be exact. See the primary
[Python numeric semantics](https://docs.python.org/3/library/stdtypes.html#numeric-types-int-float-complex)
and [Rust binary64 reference](https://doc.rust-lang.org/core/primitive.f64.html).

Int-to-Float rounds to the nearest representable value, with ties to even.
Float-to-Int truncates toward zero; NaN, infinity and values outside i64 fail
with `AIL-RUNTIME-FLOAT`. In particular, positive `2^63` is out of range while
negative `-2^63` is valid. Large Int values can lose precision on conversion.

## Values, functions and JSON

Float input accepts ASCII decimal notation with optional sign, fraction and
`e`/`E` exponent, plus exact lowercase `inf`, `+inf`, `-inf`, `nan`, `+nan`,
`-nan`. Whitespace, underscores, hexadecimal and trailing data are rejected.
Inputs are bounded to 1 MiB. Missing or invalid input uses
`AIL-RUNTIME-FLOAT`; excessive input uses `AIL-RUNTIME-FLOAT-LIMIT`.
Diagnostics omit operand values.

Results use shortest round-trip lowercase scientific notation: `1.5e0`,
`-0e0`, `5e-324`; nonfinite results use `inf`, `-inf`, `nan`. NaN has one
canonical internal bit pattern. Numeric values use scalar storage and do not
charge the text or list arena.

[Value functions](44-value-functions.md) accept Float parameters, locals and
results. Use `the function` in place of `the system` for computation and explicit
conversions. Float arguments and returns require a bare declared field; compute
an expression into a local before returning it. Every parameter is validated,
including unused parameters, in sorted name order before the depth check.
Functions retain isolated locals, the shared 64-frame limit and existing fuel.

`the system assigns 1.25 to data.value` creates a Float constant when the
destination is Float. The literal must be a strict JSON number which converts
to a finite binary64 value. Negative zero is preserved.

JSON parse/get accepts a strict number that converts to a finite Float.
JSON format/set emits the canonical finite number and rejects infinity/NaN.
These boundaries use JSON/JSON-LIMIT errors and preserve existing source/path/
value validation order. General Json values retain lossless number tokens;
only a typed Float conversion rounds a token to binary64. Scalar parsing/get
does not consume an arena; formatted Text and updated Json follow their existing
allocation rules. [List<Float> operations](46-floating-lists.md) support numerical
collections, typed functions and finite JSON arrays.

## Core and STE

`FloatOperation` nodes have kind `compute`, `int_to_float` or `float_to_int`.
Ordered `float_step` edges attach them to actions. Explicit destination, source
and expression edges carry checked operands. `FloatExpression: Float` nodes
represent literals, fields, binary operations and negation; `FloatComparison:
Bool` binds two trees. Local trees carry their function owner. Checks reject
incorrect types, hidden globals, ambiguous storage, extra metadata, shared or
orphan operands and malformed expression trees. See the
[Core schema](18-ail-core-schema.md).

The STE forms are:

```text
Compute floating expression expression_data into field destination_data.
If floating comparison left_data comparison_data right_data is true, go to label label_data.
Convert integer field source_data into floating field destination_data.
Convert floating field source_data into integer field destination_data.
```

They lower to `EVAL_FLOAT`, `BRANCH_FLOAT`, `INT_TO_FLOAT`, `FLOAT_TO_INT`.
Postfix expressions use `d:<decimal>` literals, `f:<field>` references, binary
operator tokens and unary `neg`. Expression limits are 65,536 bytes, 1,024 nodes
and depth 128; comparisons share these bounds across both operands.
Application and C interop profiles support Float computation. Source/Core
diagnostics use `AIL-FLOAT-001` (function wrappers can use `AIL-FUNCTION-001`);
malformed floating bytecode uses `AILBC017`.

Native output includes an allocation-free numeric runtime compiled from
checked-in Rust sources when building AIL. Building this runtime requires the
Rust `x86_64-unknown-linux-gnu` target and toolchain `rust-lld`. AIL compilation
copies its validated ELF segments and does not launch a compiler or linker.
Generated programs require no interpreter, dynamic loader or runtime files.
Programs with no floating operations retain their existing native emission.

## Runnable numeric file report

The [temperature report](../../examples/float-measurements) reads JSON Celsius
measurements, calls a Float conversion function, counts readings below freezing,
and writes Fahrenheit values and their mean. The supplied input produces four
readings, mean `5.68625e1` and truncated mean `56`.

```bash
cargo run -- ail-run examples/float-measurements --action Report \
  --read-file=input=examples/float-measurements/input.json \
  --write-file=report=/tmp/float-report.json

cargo run -- ail-compile examples/float-measurements --action Report \
  --target linux-x86_64-elf --out /tmp/float-report
/tmp/float-report \
  --read-file=input=examples/float-measurements/input.json \
  --write-file=report=/tmp/float-report-native.json
```

The [exported STE](../../examples/ste/float-measurements.ail-ste) uses the same
execution commands. The [implementation record](../superpowers/plans/2026-10-11-float-computation.md)
tracks verification and review evidence. Broader collections, libraries and
Python replacement remain part of the active language-development objective.

After `cargo build`, run `python3 scripts/ail_float_differential.py` for the
independent Python oracle: 1,195 arithmetic/comparison input pairs, transport
and conversion boundaries, 53 pipeline round trips and 60 file reports.
The 2026-10-11 verification passes **923 Rust tests**, strict Clippy, formatting
and prior value/JSON/literal regression oracles. The frozen ladder remains
16/16; independent frontend and native reviews have no outstanding findings.
