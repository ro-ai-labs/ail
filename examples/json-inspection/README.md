# Inspect arbitrary JSON

This package reads a JSON file, discovers its shape, recursively visits object
keys and array elements, and writes its root kind and scalar-leaf count alongside
the unchanged input value. It accepts field names that were not known when the
program was written. `pointerToken` escapes dynamic names using AIL text
operations and direct Text comparisons; `leafCount` recursively handles all six
JSON kinds using Boolean predicates.

```bash
cargo run -- ail-run examples/json-inspection --action Inspect \
  --read-file=input=examples/json-inspection/input.json \
  --write-file=report=/tmp/json-inspection-report.json

cargo run -- ail-compile examples/json-inspection --action Inspect \
  --target linux-x86_64-elf --out /tmp/json-inspection
/tmp/json-inspection \
  --read-file=input=examples/json-inspection/input.json \
  --write-file=report=/tmp/json-inspection-native.json
```

The supplied input has kind `object` and **8 scalar leaves**. Empty containers
have zero leaves; strings, numbers, booleans and null each count as one. The
sample includes slash, tilde, empty, NUL and Unicode names and values. Number
spellings such as `1.2500e-2` and `-0` remain unchanged in the report.

Duplicate names remain visible in key enumeration. The example fails on an
ambiguous duplicate-name lookup, preserving the existing JSON Pointer contract.
The existing JSON bounds, shared function depth/fuel and allocation limits apply;
this recursive example can reach function-depth limits before JSON depth limits.
Errors encountered before the final write preserve an existing report file.

The [exported STE](../ste/json-inspection.ail-ste) uses the same commands.
See [JSON inspection](../../docs/ail/47-json-inspection.md) for the operation
contract and verification evidence.
