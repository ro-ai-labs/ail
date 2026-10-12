# CSV aggregation

The application imports [ail.std.csv](../ail_std_csv.ail/README.md) as `Csv`.
Analyze selects only `out.result: Json`; Stream consumes bounded JSON-lines stdin;
WriteReport summarizes a caller-granted CSV file and writes a JSON report only
when analysis succeeds. The [manual](../../docs/ail/61-csv-aggregation.md) specifies
the dialect, error priorities, grants and measured runtime bounds.

After `cargo build`, run the VM:

```sh
target/debug/ail ail-run examples/csv-aggregate --action Analyze 'data.request={"csv":"category,amount\na,7\na,2\n"}'
target/debug/ail ail-run examples/csv-aggregate --action Stream --stdio < examples/csv-aggregate/input.jsonl
target/debug/ail ail-run examples/csv-aggregate --action WriteReport --stdio \
  --read-file=input=examples/csv-aggregate/input.csv --write-file=report=/tmp/csv-report.json
```

The Analyze result is:

```text
out.result={"ok":true,"rows":2,"total":9,"groups":[{"category":"a","rows":2,"total":9}],"error":""}
```

Compile each native entry action explicitly:

```sh
target/debug/ail ail-compile examples/csv-aggregate --action Analyze --target linux-x86_64-elf --out /tmp/csv-analyze
/tmp/csv-analyze 'data.request={"csv":"category,amount\na,7\na,2\n"}'
target/debug/ail ail-compile examples/csv-aggregate --action Stream --target linux-x86_64-elf --out /tmp/csv-stream
/tmp/csv-stream --stdio < examples/csv-aggregate/input.jsonl
target/debug/ail ail-compile examples/csv-aggregate --action WriteReport --target linux-x86_64-elf --out /tmp/csv-report
/tmp/csv-report --stdio --read-file=input=examples/csv-aggregate/input.csv --write-file=report=/tmp/csv-report-native.json
```

The sample stream's exact stdout is [expected.jsonl](expected.jsonl), with empty
stderr and status zero even after handled REQUEST/AMOUNT errors. Stream checks
trim-blank copies but passes original nonblank lines to the request wrapper.
WriteReport raises literal VALUE for CSV/HEADER/ROW/AMOUNT/OVERFLOW and preserves
preexisting report bytes. Arbitrary host write failures have no rollback promise.
Successful input/report bindings may deliberately name the same file because the
whole read finishes first. File paths and standard streams require caller grants.

The [STE export](../ste/csv-aggregate.ail-ste) contains the imported pure library.
`python3 scripts/ail_csv_differential.py` verifies independent frozen expectations,
parser vectors, actual I/O, runtime bounds and a deliberately mutated copy across
source/Core/rendered Spec/STE/saved bytecode on VM/native. Text limits, cumulative
arenas, JSON depth and instruction fuel still apply; plain 32 KiB fields pass,
while 8,192 doubled quote pairs in one roughly 16 KiB field exhaust ordinary fuel.
