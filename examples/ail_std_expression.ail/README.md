# AIL expression library

## Purpose

This reusable Application package implements tokenization, shunting-yard parsing
and checked integer evaluation in ordinary AIL functions. It has no host effect.

## Concepts Taught

Public functions:

- `evaluate(expression: Text, variables: Json) -> Json` evaluates an expression
  using the supplied object of variable values.
- `evaluateRequest(text: Text) -> Json` parses a request object with required
  `expression` and `variables` members, then evaluates it.

Both return `{"ok":true,"value":14,"error":""}` on success, or
`{"ok":false,"value":0,"error":"CODE"}` on a structured failure. Other
functions are implementation helpers.

For a neighboring package, add `imports: ../ail_std_expression.ail as Calc` to
its manifest. Invoke `Calc.evaluate` or `Calc.evaluateRequest` with named arguments
and a declared Json destination. The [calculator](../expression-calculator/README.md)
and [benchmark](../../bench/R026-expression-calculator/intent.md) are callers.

## Files To Inspect

See the [reference](../../docs/ail/55-expression-calculator.md) for the grammar,
4096-character/512-token bounds, error codes, integer semantics and commands.
The [source](spec.ail-spec.md) contains the actual interpreter. The package is
support-only in the historical replay catalog and does not inflate its count.

## Expected Replay Artifacts

Run `python3 scripts/ail_calculator_differential.py` after `cargo build`.
The runner constructs checked Core, STE, bytecode and native artifacts in a
temporary workspace and compares actual outputs to independent calculations.
The checked-in [STE export](../ste/expression-calculator.ail-ste) includes the
library as imported by the calculator; this library has no action entrypoint.

## Rejected Fixtures

The [rung cases](../../bench/R026-expression-calculator/cases) and independent
oracle cover malformed expression syntax, unsupported characters, variable types,
overflow, division by zero and bounds. These are runtime request failures with
structured results, not checker-rejected AIL source fixtures.

## Next Example To Read

Run the [JSON-lines calculator](../expression-calculator/README.md) to see pure
functions composed with explicitly granted standard streams.

## v0.3 Learning Signal

This package supplies executable parsing and evaluation rather than only a
contract for them. Capability rung R026 checks the program through VM and native
execution; the historical v0.3 replay count is unchanged. Remaining capability
tiers and general Python compatibility still require their own evidence.
