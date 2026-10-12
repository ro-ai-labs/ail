# Summarize tab-separated records

This example normalizes CRLF lines, splits lines and columns, trims Unicode
whitespace, filters rows and joins accepted names into a summary. Reusable AIL
functions handle normalization and column cleanup; the action writes a JSON report.

The input has a header row followed by three unquoted tab-separated columns:
`item`, `units`, `unit_cents`. The first row is skipped. Empty lines are ignored;
wrong column counts, blank fields, nonpositive units and negative prices count as
rejected rows. Numeric fields must otherwise be strict JSON integers. Malformed
numbers and arithmetic overflow fail before the report is written. Tabs and
newlines delimit fields; this format has no CSV quoting or embedded-delimiter rule.

```bash
cargo run -- ail-run examples/tabular-report --action Summarize \
  --read-file=input=examples/tabular-report/input.tsv \
  --write-file=report=/tmp/tabular-report.json

cargo run -- ail-compile examples/tabular-report --action Summarize \
  --target linux-x86_64-elf --out /tmp/tabular-report
/tmp/tabular-report \
  --read-file=input=examples/tabular-report/input.tsv \
  --write-file=report=/tmp/tabular-report-native.json
```

The sample accepts **2 rows**, rejects **2 rows**, sums **5 units** and
**850 cents**, and produces `Apple | Unicode λ`. Whole-number prices avoid
rounding in this example. The report also retains names as a List<Text>.

See [text transformations](../../docs/ail/49-text-transformations.md) and
[exported STE](../ste/tabular-report.ail-ste). File, expression, list, function
fuel/depth and shared allocation limits still apply.
