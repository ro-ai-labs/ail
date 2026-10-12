# CSV parsing and aggregation written in AIL

Implementation design for the CSV milestone. The preceding trustworthy-edit
milestone passed its final gates. Independent R022 inputs are reviewed and frozen;
AIL implementation and application acceptance remain tracked in the plan.

## Intent and choice

The active goal remains executable Specs → checked Core → ASD-STE100-based STE
→ VM/native binary, then a useful Python replacement and AI-agent language.
Add an ordinary data-processing application: parse quoted CSV, aggregate signed
integer amounts by category, and expose the result as JSON through files and
JSON-lines. This measures planned Tier 2 rung R022 and gives agents a reusable
library. It does not complete the larger goal or alpha-publication gate.

Considered alternatives: more host/network effects expand integrations but do
not test composition of the language; a dedicated host CSV opcode bypasses the
language. A pure AIL library uses the existing text, Json, functions, predicates,
recovery and I/O mechanisms. Make compiler changes only for reproduced defects,
through a separately reviewed fix; do not silently grow this scope.

The CSV dialect is explicit rather than a general conformance claim. RFC 4180
describes common quoting and record conventions, while Python's csv reader
provides an independent implementation for accepted rows. References:
https://www.rfc-editor.org/rfc/rfc4180
https://docs.python.org/3/library/csv.html

## Public library

`examples/ail_std_csv.ail`, Application profile, imported as `Csv`:

- `parse(text: Text) -> Json`: array of row arrays of strings, or catchable TEXT
  for invalid CSV syntax. No implicit headers, trimming or numeric conversion.
- `summarize(text: Text) -> Json`: parse and aggregate under the contract below;
  structured domain failures, with ordinary fatal runtime failures propagating.
- `summarizeRequest(text: Text) -> Json`: parse a JSON object containing one
  unambiguous `csv` string, ignore unrelated members, then call summarize.
  Invalid JSON, wrong shape, missing/wrong/duplicated csv is REQUEST.

Ordinary source/runtime Text limits, 64 MiB cumulative arenas, function depth
and instruction fuel continue to apply. No promises of unbounded streaming or
automatic memory reclamation. Do not add a tiny fixed document/row limit merely
to make tests pass. Benchmark representative useful sizes independently.

## CSV dialect

Delimiter comma; quote double quote. A quoted field starts with a quote exactly
at field start. Inside it, doubled quotes decode to one quote, while commas and
CR/LF are data. After its closing quote, only delimiter, record terminator or
end of input is legal. A quote anywhere in an unquoted field is invalid.
No backslash escapes, comments, whitespace trimming, dialect sniffing or BOM
removal. Unicode, NUL and spaces are ordinary field contents.

Outside quoted fields, CRLF is one record terminator; lone CR and LF also end
records. Preserve quoted line endings exactly. Ending with a record terminator
does not add another record. Empty input yields no rows; a blank physical record
yields an empty row `[]`, distinct from a quoted empty field `[""]`. Trailing
commas and adjacent commas create empty fields. Examples:

- empty text → `[]`
- LF → `[[]]`
- comma → `[["",""]]`
- two quotes → `[[""]]`
- `a` + LF → `[["a"]]`
- two LF characters → `[[],[]]`

Parse the entire CSV syntax before header, width or numeric processing.

## Aggregation

The first row is a header. Require distinct nonempty header strings, including
exactly one `category` and one `amount`; other named columns are allowed and
their values are ignored. No header normalization. Every later row must match
the full header width, including otherwise blank rows. Header-only input is
valid and returns zero rows/total and no groups. Empty input fails HEADER.

Each amount must match `-?(0|[1-9][0-9]*)` and fit signed i64. `-0` is allowed;
spaces, plus signs, leading zeros, decimal/exponent forms and out-of-range
tokens are AMOUNT. Category is the exact string, including empty strings and
characters requiring JSON Pointer escaping. Group by exact decoded category.

Validate complete CSV, header, all row widths, then process data rows in input
order. Per row, parse amount, add to that category's sum, then global total.
Both additions use checked i64; a failing intermediate sum is OVERFLOW even if
later rows could bring the mathematical sum back into range. Counts are exact.
Sort groups by AIL's Unicode text ordering, ascending. Return key order exactly:

`{"ok":true,"rows":2,"total":9,"groups":[{"category":"a","rows":2,"total":9}],"error":""}`

Failure is always:
`{"ok":false,"rows":0,"total":0,"groups":[],"error":"CODE"}`

Codes in priority order: REQUEST (request wrapper), CSV (complete syntax),
HEADER, ROW (any width mismatch), then AMOUNT/OVERFLOW in processing order.
Do not expose partial aggregates as successful output.

## Implementation constraints from feasibility evidence

An isolated character-slicing loop succeeds for 1,024 characters but exhausts
the existing action instruction budget by 4,096; both VM/native agree. Evidence
`/tmp/ail-csv-character-probe.json` and its throwaway package. This is a design
constraint, not a compiler defect. Avoid one root-loop iteration per character.

A separate bounded-window delimiter prototype passes 32 KiB ASCII and Unicode
fields, 256 CRLF rows, and 1,024 LF rows on both backends. Its observed VM times
are under one second and native times under 0.12 seconds on this machine.
Evidence `/tmp/ail-csv-window-probe.json`; this proves only delimiter scanning,
not CSV parsing or end-to-end aggregation performance.

Use separately called field/record scanners and bulk text operations. A useful
candidate is a bounded-window search helper over the original text and cursor:
slice up to 512 scalars, find syntax delimiters in that window, and return an
absolute index. This avoids repeatedly copying the entire remaining document.
Quoted fields can validate quote pairs by search, slice contents once and
decode doubled quotes in one replacement. Private structured scanner results
may use Json or a small custom variant; public results remain ordinary Json.
These are implementation options, not a requirement to pad instruction counts
or adjust runtime budgets. Test 32 KiB fields and practical multi-row documents.

## Executables

`examples/csv-aggregate` imports the library:

- `Analyze`: reads `data.request: Text`, calls summarizeRequest, selects only
  `out.result: Json`, suitable for single-line benchmark argument transport.
- `Stream`: bounded stdin of JSON-lines requests; skip trim-empty lines using
  a copy, pass original nonblank line, emit exactly one compact result plus LF
  per request and continue after structured errors. `--stdio` grants streams.
- `WriteReport`: reads caller-bound file resource `input`, calls summarize,
  checks ok before opening caller-bound `report`, then writes compact JSON plus
  LF. Structured failure raises literal VALUE and preserves existing report
  bytes. Other runtime failures propagate; arbitrary write-I/O rollback is not
  promised. Result codes remain inspectable through Analyze/Stream.

Include sample CSV, a mixed request stream and exact generated STE export.
No Python interpreter, csv process, external tools or host parser in the result.

## Independent evidence and review

Before AIL implementation, independently author R022-csv-aggregate intent,
Python reference and cases. Freeze hashes and show missing-program RED; no
existing rung changes or --bless-ladder. Python csv.reader with newline='' and
strict=True supplies accepted-row parsing; add an independent declarative
whole-input grammar check to reject bare-field quotes that Python tolerates.
Use a grammar-based regex/token validator rather than copying the AIL scanner.
Expected aggregation uses Python integers/dictionaries plus explicit i64 checks
and a stable encoder matching AIL's documented JSON transport.

Cases cover blank/empty/trailing records, all line endings, escaped and embedded
quotes/newlines, Unicode/NUL/BOM, reordered/extra/duplicate/empty/missing headers,
width mismatches, canonical integer grammar and extrema, group/global overflow,
error precedence, escaped pointer keys, deterministic sorting, request shape
and duplicate selected JSON members. Freeze generated cases before program work.

Independent differential acceptance compares actual results from source, saved
Core, rendered Spec, STE and saved bytecode on VM/native, including direct parse
library results. Assert actual file preservation, exact stdout/stderr/status,
mixed-stream continuation, and one deliberately mutated AIL operation failing
the oracle. Require useful sizes including 32 KiB long fields, quote runs across
search-window boundaries and hundreds of short records; report ordinary runtime
limits separately from domain errors.

Compiler/source dependencies remain byte-identical unless a reproduced defect
requires a scoped reviewed fix. If unchanged, existing completed broad semantic
gates are retained; run the new acceptance, unchanged full benchmark ratchet,
affected documentation/package tests, link checks and STE regeneration. Record
the new score only after all required gates pass. No duplicate broad suites
merely for new example files.

Continue locally without git mutations, dependencies, publication or another
approval pause under the standing autonomous objective. Preserve prior work and
all old oracle/rung inputs. The plan records implementation and verification evidence; this design alone
carries no implementation completion claim.
