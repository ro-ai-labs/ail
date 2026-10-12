# Floating-list statistics

This package reads a nonempty JSON array, calls reusable `mean` and `center`
functions, and writes a report with centered samples and population variance.
The original JSON fields and number spellings are retained. Computation uses
binary64 rounding and the existing bounded immutable-list implementation.

```bash
cargo run -- ail-run examples/floating-lists --action Analyze \
  --read-file=input=examples/floating-lists/input.json \
  --write-file=report=/tmp/floating-lists-report.json

cargo run -- ail-compile examples/floating-lists --action Analyze \
  --target linux-x86_64-elf --out /tmp/floating-lists
/tmp/floating-lists \
  --read-file=input=examples/floating-lists/input.json \
  --write-file=report=/tmp/floating-lists-native.json
```

The supplied input produces count `4`, mean `3.5e0`, centered samples
`[-2e0,-1e0,5e-1,2.5e0]` and population variance `2.875e0`.
Empty data fails when computing the mean. Nonfinite input or results fail at
the JSON boundary before the output file is written.

The [exported STE](../ste/floating-lists.ail-ste) runs through the same CLI.
Read the [floating-list guide](../../docs/ail/46-floating-lists.md) for runtime
transport, JSON boundaries, function returns and allocation limits.
