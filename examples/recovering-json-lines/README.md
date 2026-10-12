# Recovering JSON Lines

Read numeric rows, recover from bad data inside a reusable function, and write
one JSON report. Each nonblank line must contain a JSON object with signed64-bit
integer `value` and `divisor` fields. Division rounds toward zero. Accepted
quotients are sorted; rejected rows are counted by stable error category.

```bash
cargo run -- ail-run examples/recovering-json-lines --action Analyze \
  --read-file=input=examples/recovering-json-lines/input.jsonl \
  --write-file=report=/tmp/recovered-report.json
```

The sample produces:

```json
{"rows":7,"accepted":2,"rejected":5,"values":[-2,5],"errors":{"AIL-RUNTIME-DIVZERO":1,"AIL-RUNTIME-JSON":2,"AIL-RUNTIME-JSON-PATH":1,"AIL-RUNTIME-OVERFLOW":1}}
```

Lines are split at LF, trimmed with AIL's fixed Unicode whitespace set, and empty
lines are skipped. Missing or ambiguous duplicate keys report JSON-PATH;
malformed JSON and wrong value types report JSON. A zero divisor reports DIVZERO,
and the minimum integer divided by -1 reports OVERFLOW. Extra fields are allowed.
The handler receives only a category code, never the rejected row's contents.

File, allocation and execution limits remain fatal. Output is written after all
rows are processed, so earlier report bytes survive input or computation errors
that happen before that write. This example retains the existing bounded,
immutable collection and arena semantics.

See [runtime recovery](../../docs/ail/51-runtime-recovery.md) and the
[exported STE](../ste/recovering-json-lines.ail-ste). The compiler emits a
standalone Linux x86-64 executable with the same named file bindings:

```bash
cargo run -- ail-compile examples/recovering-json-lines --action Analyze \
  --target linux-x86_64-elf --out /tmp/recovering-json-lines
/tmp/recovering-json-lines \
  --read-file=input=examples/recovering-json-lines/input.jsonl \
  --write-file=report=/tmp/recovered-native-report.json
```
