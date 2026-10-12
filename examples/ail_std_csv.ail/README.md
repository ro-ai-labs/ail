# CSV library

## Purpose

`ail.std.csv` version 0.1.0 is an Application-profile library. Import it as `Csv`
to parse an explicit CSV dialect and aggregate signed i64 amounts by category.
The implementation uses ordinary AIL text, Json, functions and checked integer
operations.

## Concepts Taught

- `Csv.parse(text: Text) -> Json` returns row arrays of strings. Invalid CSV
  syntax raises catchable `TEXT`. Empty input yields `[]`; blank records yield
  `[]` rows; quoted empty fields and trailing commas remain actual fields.
- `Csv.summarize(text: Text) -> Json` requires distinct nonempty headers with
  `category` and `amount`, matching data widths, and canonical signed integer
  amounts. It returns `ok`, `rows`, `total`, sorted `groups`, and `error`, in that
  order. Failure returns false, zero counts/totals, empty groups and a code.
- `Csv.summarizeRequest(text: Text) -> Json` accepts a JSON object with exactly
  one decoded `csv` string member. Other members are ignored. Invalid requests
  return `REQUEST`; duplicate selected members are rejected.

Comma separates fields. Double quotes begin only at field start; doubled quotes
escape a quote. Quoted CR/LF remain data. Outside quotes, CRLF, CR and LF end a
record, and a final terminator adds no extra row. Spaces, Unicode, NUL and BOM
are preserved. Characters after a closing quote and bare-field quotes reject.
Amounts match `-?(0|[1-9][0-9]*)` and signed i64 bounds; `-0` is accepted.

Validation order is complete CSV syntax, header, every row width, then each
amount and its category/global additions in input order. Domain errors are
`CSV`, `HEADER`, `ROW`, `AMOUNT` and `OVERFLOW`. Fatal runtime limits propagate.
Category and header keys escape tilde and slash before JSON Pointer lookup.
Groups sort by AIL's ascending Unicode text ordering.

Field searches inspect bounded windows of 512 Unicode scalars. Field and record
helpers receive ordinary fresh function budgets; no custom CSV opcode or host
parser is used. Runtime bounds still apply: 1 MiB Text/Json values, 65,536 JSON
values, 64 container depth/call frames, separate shared 64 MiB text and list/Json
arenas, and instruction fuel per function. The parser builds the full result and
aggregation validates it before summing; this is bounded document processing.
No additional fixed document or row limit is imposed by the library.

## Files To Inspect

- `ail-package.md`: package name, version and Application profile.
- `spec.ail-spec.md`: delimiter, field and record helpers; public parse and
  summary functions; checked additions and structured domain recovery.
- `../csv-aggregate/spec.ail-spec.md`: the importing Analyze application.

## Expected Replay Artifacts

`ail-check` verifies the library or importing app. `ail-core` and `ail-ste`
export checked representations; `ail-run` executes Analyze on the VM and
`ail-compile --target linux-x86_64-elf` produces its native executable.
Analyze selects only `out.result`; the task evidence records direct parser
checks and the independent R022 cases. Full multi-route and file/stream
acceptance are separate application work.

## Rejected Fixtures

The R022 case set rejects bare quotes, unclosed quotes, closing-quote junk,
invalid/duplicate headers, any wrong row width, noncanonical/out-of-range
amounts, intermediate group/global overflow, and malformed/ambiguous requests.
The direct parser harness additionally checks blank records, NUL/multiline
transport, boundary-spanning quote pairs and CRLF, and malformed many-row tails.

## Next Example To Read

Read `../csv-aggregate/README.md` for Analyze's JSON request transport, or
`../ail_std_expression.ail/README.md` for another pure data-processing library.

## v0.3 Learning Signal

A reusable imported pure AIL library can express quoted CSV parsing and checked
aggregation through existing operations. Staged acceptance evidence does not
claim general Python csv compatibility, unbounded streaming, or completion of
the wider language milestone.
