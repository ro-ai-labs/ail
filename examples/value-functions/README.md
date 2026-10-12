# Reusable JSON report functions

This package reads `input.json`, transforms each record with `transformRecord`,
and builds a report using `double` (Int), `label` (Text) and `addName`
(List<Text>). The record transform removes `private`, doubles `metrics.amount`
and adds `processed: true`; unrelated JSON number tokens retain their spelling.
Labels are `processed: Ada`, `processed: Mihai` and `processed: Zoë 🙂`.
The report contains three records and total `80`.

From the repository root:

```bash
cargo run -- ail-run examples/value-functions --action Transform \
  --read-file=input=examples/value-functions/input.json \
  --write-file=report=/tmp/value-report.json

cargo run -- ail-compile examples/value-functions --action Transform \
  --target linux-x86_64-elf --out /tmp/value-report
/tmp/value-report \
  --read-file=input=examples/value-functions/input.json \
  --write-file=report=/tmp/value-report-native.json
```

The [saved STE](../ste/value-functions.ail-ste) executes through the same CLI.
Read the [value-function guide](../../docs/ail/44-value-functions.md) for source
syntax, typed boundaries, immutable returns and shared memory limits.
