# Filter JSON records with Boolean predicates

`eligible` combines a Bool flag, Text alternatives, an Int bound and a Float
threshold into a reusable predicate. The action reads an array of records,
selects matching names and writes a JSON report. The sample accepts `ship parser`
and `Unicode λ fix`: **2 accepted, 3 rejected**.

```bash
cargo run -- ail-run examples/boolean-filter --action Filter \
  --read-file=input=examples/boolean-filter/input.json \
  --write-file=report=/tmp/boolean-filter-report.json

cargo run -- ail-compile examples/boolean-filter --action Filter \
  --target linux-x86_64-elf --out /tmp/boolean-filter
/tmp/boolean-filter \
  --read-file=input=examples/boolean-filter/input.json \
  --write-file=report=/tmp/boolean-filter-native.json
```

`and`/`or` evaluate left to right and skip unnecessary operands. Function entry
still validates every argument. This example loads and checks all required JSON
record fields before calling the predicate, including for disabled records.
The report is written only after all records succeed.

See [Boolean predicates](../../docs/ail/48-boolean-predicates.md) and
[exported STE](../ste/boolean-filter.ail-ste).
