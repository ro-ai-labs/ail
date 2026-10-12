# Statistics from two measurement batches

This program reads JSON containing `first` and `second` arrays of finite numbers
and an integer `limit`. It concatenates the batches, sorts them, calculates their
median, then slices and reverses the highest requested values. Reusable functions
handle batch merging, the output window and the median calculation.

```bash
cargo run -- ail-run examples/list-statistics --action Analyze \
  --read-file=input=examples/list-statistics/input.json \
  --write-file=report=/tmp/list-statistics.json

cargo run -- ail-compile examples/list-statistics --action Analyze \
  --target linux-x86_64-elf --out /tmp/list-statistics
/tmp/list-statistics \
  --read-file=input=examples/list-statistics/input.json \
  --write-file=report=/tmp/list-statistics-native.json
```

The sample reports count **5**, median **2**, sorted values
`[1.5,1.5,2,3,9]` and highest values `[9,3,2]`. Both empty batches produce count 0,
median `null` and empty lists. A negative limit produces an empty highest window;
a limit above the count includes all values. Highest values reverse a stable
ascending slice, so equal values appear in reverse source order in that window.

Median arithmetic uses binary64 `(lower + upper) / 2` for an even count.
If that produces a nonfinite value, JSON report conversion fails before writing.
Malformed input also fails before writing; previously written report bytes remain.
The `sortedMedian` function expects a nonempty sorted list; the action checks it.

See [list transformations](../../docs/ail/50-list-transformations.md) and the
[exported STE](../ste/list-statistics.ail-ste).
