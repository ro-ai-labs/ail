# JSON-lines filter

Read bounded stdin, recover from bad records, and emit accepted quotients as
JSON lines in input order. Each nonblank row needs unique `value` and `divisor`
members containing signed 64-bit integers. Division rounds toward zero.

```bash
cargo run --quiet -- ail-run examples/json-lines-filter --action Filter --stdio \
  < examples/json-lines-filter/input.jsonl > /tmp/accepted.jsonl 2> /tmp/summary.json
```

The sample writes these exact lines to stdout:

```json
{"value":5}
{"value":-2}
```

It writes one LF-terminated summary to stderr:

```json
{"rows":7,"accepted":2,"rejected":5,"errors":{"AIL-RUNTIME-DIVZERO":1,"AIL-RUNTIME-JSON":2,"AIL-RUNTIME-JSON-PATH":1,"AIL-RUNTIME-OVERFLOW":1}}
```

`--stdio` grants all three standard streams and suppresses automatic state/status/
trace reports. The summary is an explicit application write. Compilation does not
embed this grant; pass it again when executing the standalone Linux x86-64 binary:

```bash
cargo run --quiet -- ail-compile examples/json-lines-filter --action Filter \
  --target linux-x86_64-elf --out /tmp/json-lines-filter
/tmp/json-lines-filter --stdio < examples/json-lines-filter/input.jsonl
```

The filter first reads through EOF, with a 1,048,576-byte UTF-8 limit. It then
splits at LF, trims AIL's fixed Unicode whitespace set and skips blank rows.
Empty input produces no stdout and a zero summary. Wrong JSON/types, missing or
ambiguous keys, division by zero and integer overflow reject one row. A reusable
function handles these data errors without exposing the rejected input.

IO, allocation and execution limits remain fatal. A fatal error before processing
leaves stdout empty; a later fatal error retains already written records and
omits the final summary. The example uses bounded immutable values and shared
arenas, so the input byte limit alone does not guarantee a batch will fit.

See [standard streams](../../docs/ail/52-standard-streams.md),
[runtime recovery](../../docs/ail/51-runtime-recovery.md) and
[exported STE](../ste/json-lines-filter.ail-ste). The independent oracle in
`scripts/ail_stdio_differential.py` checks bytes across source/Core/STE, VM/native,
and an actual two-process AIL pipeline.
