# Floating-point temperature report

This package reads a nonempty JSON array of Celsius measurements, calls a typed
Float function for each conversion, counts readings below freezing, and writes
the Fahrenheit values and their arithmetic mean. Integer loop indices and counts
remain Int; the mean calculation converts the count explicitly to Float.

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

The supplied measurements produce four readings, one below freezing, mean
`5.68625e1` and truncated mean `56`. Empty input fails with DIVZERO when taking
the mean. Invalid or nonfinite JSON numbers fail before the report is written.
Float uses binary64 rounding, not exact decimal arithmetic.

The [saved STE](../ste/float-measurements.ail-ste) executes through the same CLI.
Read the [Float guide](../../docs/ail/45-float-computation.md) for expressions,
explicit conversions, numeric limits and finite JSON boundaries.
