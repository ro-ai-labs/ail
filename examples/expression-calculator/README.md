# Expression calculator

This package imports the [AIL expression library](../ail_std_expression.ail/README.md)
and exposes single-expression and JSON-lines actions. The tokenizer, parser and
evaluator are written in AIL and run in both the VM and standalone native output.

From the repository root:

```bash
cargo build
target/debug/ail ail-run examples/expression-calculator --action Evaluate \
  'data.expression=2 + rate * (units - 1)' \
  'data.variables={"rate":7,"units":4}'
```

The selected output is `out.result={"ok":true,"value":23,"error":""}`.

Run the sample request stream:

```bash
target/debug/ail ail-run examples/expression-calculator --action Stream --stdio \
  < examples/expression-calculator/input.jsonl

target/debug/ail ail-compile examples/expression-calculator --action Stream \
  --target linux-x86_64-elf --out /tmp/ail-calculator
/tmp/ail-calculator --stdio < examples/expression-calculator/input.jsonl
```

Each request requires `expression` and `variables`; each nonblank line produces
one JSON result. Errors use `ok:false`, `value:0` and a stable `error` code, and
later requests continue. No external calculator or Python runtime is invoked.
Standard input is read once under the existing 1 MiB bound.

The [language guide](../../docs/ail/55-expression-calculator.md) describes grammar,
precedence, exact limits, error priority and the checked i64 differences from
Python. Inspect [source](spec.ail-spec.md), [sample](input.jsonl), and
[generated STE](../ste/expression-calculator.ail-ste).

After building, run `python3 scripts/ail_calculator_differential.py` for independent
generated and boundary comparisons, or
`python3 scripts/ail_bench.py --rung R026-expression-calculator` for the frozen rung.
