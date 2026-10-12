# CSV parsing and category aggregation

The [CSV library](../../examples/ail_std_csv.ail/README.md) implements parsing and
aggregation entirely in AIL. The [application](../../examples/csv-aggregate/README.md)
imports it as `Csv`; generated executables use ordinary text, Json, functions and
actions. There is no host CSV parser, interpreter dependency or CSV opcode.

## Public API

Package `ail.std.csv` version 0.1.0 uses the Application profile:

```text
imports: ../ail_std_csv.ail compatible ^0.1 as Csv
```

Paths are relative to the importing package. All three functions have no effects:

| Function | Result |
| --- | --- |
| `Csv.parse(text: Text) -> Json` | Array of row arrays of strings; malformed syntax raises catchable TEXT. |
| `Csv.summarize(text: Text) -> Json` | Exact success/failure object described below. |
| `Csv.summarizeRequest(text: Text) -> Json` | Requires a JSON object with one unambiguous string `csv`, ignores unrelated members, then summarizes. |

REQUEST covers invalid JSON, wrong shape, absent/wrong selected member and
duplicate selected `csv` members, including escaped equivalent keys. Ordinary
fatal JSON depth, allocation or instruction bounds propagate.

## CSV dialect and validation

Comma separates fields. A double quote opens a quoted field only at its start.
Doubled quotes inside a quoted field decode to one quote. Commas and CR/LF inside
quotes remain literal data. After a closing quote, only comma, a record terminator
or EOF is legal. Quotes within bare fields are invalid. Backslashes have no escape
meaning; there are no comments, trimming, BOM removal or dialect detection.
Unicode, NUL and spaces are ordinary contents.

Outside quotes, CRLF is one terminator; lone CR and LF also end records. A final
terminator creates no extra row. Empty input returns `[]`; a blank physical record
returns an empty row, distinct from a quoted empty field. For example:

| CSV text (JSON string notation) | Parsed rows |
| --- | --- |
| `""` | `[]` |
| `"\n"` | `[[]]` |
| `"\n\n"` | `[[],[]]` |
| `"\"\""` | `[[""]]` |
| `","` | `[["",""]]` |
| `"a,\r\n"` | `[["a",""]]` |
| `"\"a\r\nb\""` | `[["a\r\nb"]]` |

Summary requires a first-row header of distinct nonempty names containing
exactly one `category` and one `amount`. Additional named columns are accepted
and ignored. Every later row must match the entire header width. Header-only
input succeeds with zero rows, total and groups; an empty document fails HEADER.

Amounts match `-?(0|[1-9][0-9]*)` and fit signed i64. `-0` is valid. Plus signs,
spaces, leading zeros, fractions, exponent forms and out-of-range tokens fail
AMOUNT. Categories are exact decoded strings, including empty strings, controls,
slashes and tildes. Groups sort by ascending Unicode text order.

Validation priority is complete CSV syntax, header, **all** row widths, then
amounts/additions in input order. Each row parses its amount, adds to that
category's subtotal, then the global total using checked i64 operations. An
overflow fails immediately even if a later mathematical sum would fit.

Success and failure have this exact key order and compact encoding:

```json
{"ok":true,"rows":2,"total":9,"groups":[{"category":"a","rows":2,"total":9}],"error":""}
{"ok":false,"rows":0,"total":0,"groups":[],"error":"CODE"}
```

Codes are REQUEST (wrapper), CSV, HEADER, ROW, then AMOUNT/OVERFLOW in processing
order. Failures expose no partial aggregate.

## Executable actions

After `cargo build`, Analyze reads `data.request: Text` and selects only
`out.result: Json`. The argument is a JSON request; embedded NUL uses a JSON escape:

```sh
target/debug/ail ail-run examples/csv-aggregate --action Analyze 'data.request={"csv":"category,amount\na,7\na,2\n"}'
target/debug/ail ail-compile examples/csv-aggregate --action Analyze --target linux-x86_64-elf --out /tmp/csv-analyze
/tmp/csv-analyze 'data.request={"csv":"category,amount\na,7\na,2\n"}'
```

Stream reads bounded stdin through EOF, splits on LF, checks a trimmed copy for
blankness and passes each original nonblank line to summarizeRequest. CRLF JSON
lines and JSON whitespace are accepted. Nonblank lines retain other Unicode
whitespace: a request surrounded by U+00A0 returns REQUEST, because JSON permits
only its own whitespace characters. Each request emits exactly compact JSON
plus LF, continuing after structured errors. Bare `--stdio` grants streams and
suppresses automatic state/trace reports; handled domain failures have status
zero and empty stderr:

```sh
target/debug/ail ail-run examples/csv-aggregate --action Stream --stdio < examples/csv-aggregate/input.jsonl
target/debug/ail ail-compile examples/csv-aggregate --action Stream --target linux-x86_64-elf --out /tmp/csv-stream
/tmp/csv-stream --stdio < examples/csv-aggregate/input.jsonl
```

The sample's exact expected output is
[expected.jsonl](../../examples/csv-aggregate/expected.jsonl). This is a bounded
whole-input action, rather than an incremental or unbounded streaming parser.

WriteReport reads a caller-granted input file, summarizes it, and checks `ok`
before the first report write/open. Structured failure raises literal VALUE and
preserves existing report bytes. Ordinary file/capability/runtime failures
propagate; arbitrary write failures have no transactional rollback guarantee.
The existing whole-read behavior permits intentional input/report path aliasing.

```sh
target/debug/ail ail-run examples/csv-aggregate --action WriteReport --stdio \
  --read-file=input=examples/csv-aggregate/input.csv --write-file=report=/tmp/csv-report.json
target/debug/ail ail-compile examples/csv-aggregate --action WriteReport --target linux-x86_64-elf --out /tmp/csv-report
/tmp/csv-report --stdio --read-file=input=examples/csv-aggregate/input.csv --write-file=report=/tmp/csv-report-native.json
```

The successful file contains exactly the success JSON plus LF. Missing or wrong
file grants cannot authorize a write. Paths and ignored private request members
are never part of the report. Categories deliberately returned by contract are
visible. Missing stdio permission and invalid UTF-8 remain ordinary failures.

## Artifact routes and independent acceptance

The [checked-in STE](../../examples/ste/csv-aggregate.ail-ste) contains the imported
library and is generated exactly with:

```sh
target/debug/ail ail-ste examples/csv-aggregate > /tmp/csv-aggregate.ail-ste
cmp /tmp/csv-aggregate.ail-ste examples/ste/csv-aggregate.ail-ste
target/debug/ail ail-vm examples/ste/csv-aggregate.ail-ste --action Stream --stdio < examples/csv-aggregate/input.jsonl
python3 scripts/ail_csv_differential.py
```

Acceptance obtains all 129 aggregation expectations from frozen R022 inputs and
its independent Python csv.reader/grammar oracle. Hand-calculated parser vectors
cover empty/blank/quoted-empty distinctions, mixed terminators, quote escapes,
Unicode/NUL, malformed tails and 511/512/513-scalar boundaries. Actual source,
saved Core, rendered Spec and their STE forms execute on VM/native; a saved
bytecode artifact is separately executed and compiled to native. Analyze must
select only its result. Bytecode checks forbid external/process/host parser
operations and effects in Csv routines. Actions own the file and stream effects.

The gate verifies exact streams, domain continuation, report preservation,
missing/wrong grants, aliasing, input UTF-8 and runtime bounds. An isolated copied
library changes group sort order; independent expected output must reject it on
VM/native. `--artifact-dir /tmp/csv-acceptance` retains bounded command evidence
and artifacts in an initially empty directory. No session-specific `/tmp` files
are needed by the script.

## Measured sizes and runtime bounds

Plain 32,768-scalar ASCII and Unicode fields, 1,024 direct-parser records and
frozen aggregation cases with 384 CRLF or 768 LF data rows pass on both backends.
Quote pairs and CRLF crossing the 512-scalar search windows are covered. This is
representative acceptance, with no universal byte/row threshold or general
Python CSV compatibility claim.

Input content matters: a valid field containing **8,192 doubled quote pairs**
(roughly 16 KiB of CSV) exhausts ordinary function instruction fuel on VM/native,
although plain 32 KiB fields pass. A request with 65 nested arrays in an unused
member fails ordinary JSON-LIMIT, rather than returning REQUEST. These failures
propagate with no selected result.

Text/Json values have the existing 1,048,576-byte limits, 65,536 JSON-value bound,
depth/frame bounds and separate cumulative 67,108,864-byte Text and List/Json
arenas. The bounded stdin reader consumes at most 1,048,577 bytes to detect an
oversized Text. Immutable row/group values can be copied and scalar-indexed text
operations can rescan prefixes; no linear-time or automatic reclamation claim
is made. The [runtime reference](52-standard-streams.md) explains stream grants,
UTF-8, visibility and non-rollback behavior.
