rung: R022-csv-aggregate
tier: 2
action: Analyze
targets: vm,native
blocked-on: pure AIL strict CSV parser and checked category aggregation

# R022 — Parse CSV and aggregate signed amounts

Analyze reads `data.request: Text`, a JSON object with exactly one selected
`csv` string, and selects only `out.result: Json`. Unrelated members and their
duplicates are ignored. Invalid JSON, nonobjects and absent, wrong-type or
duplicated selected csv members produce REQUEST. Equivalent escaped key
spellings count as duplicates. All JSON must have valid Unicode scalar strings.

CSV delimiter is comma and quote is double quote. Quotes start exactly at field
start; doubled quotes decode inside quoted fields. Quoted commas and CR/LF are
data. Closing quotes permit only delimiter, terminator or EOF. Bare-field quotes
are invalid. Outside quotes CRLF, CR and LF terminate records. A final terminator
adds no record. Empty input is []; blank records are [], quoted empty is [""],
and adjacent/trailing commas produce empty fields. No trimming or BOM removal.

Require a first header row with distinct nonempty exact names including category
and amount. Extra named columns are allowed. All data widths must match the
entire header before numeric processing. Amount grammar is -?(0|[1-9][0-9]*),
within signed i64, with -0 allowed. Process rows in order: amount, checked group
addition, checked global addition. Any intermediate overflow is OVERFLOW even
when later rows would compensate. Categories group by exact decoded strings;
sort groups by Unicode scalar lexical order ascending, without normalization.

Success key order is exactly:
`{"ok":true,"rows":2,"total":9,"groups":[{"category":"a","rows":2,"total":9}],"error":""}`.
Failure is exactly:
`{"ok":false,"rows":0,"total":0,"groups":[],"error":"CODE"}`.
Priority is REQUEST, entire CSV syntax (CSV), HEADER, all widths (ROW), then
AMOUNT/OVERFLOW in row processing order. No partial aggregates escape.

Compact output uses established Json escaping: quote/backslash, LF/CR/TAB short
escapes, other C0 and DEL/C1 controls lowercase \uXXXX, other scalars as UTF-8.
No host CSV operation, external process or dedicated CSV opcode may implement
the AIL program. Ordinary runtime bounds remain separate from domain errors.

The independently authored oracle uses a declarative whole-input regular grammar
and Python csv.reader(strict=True, newline='') for decoding, then Python integer
and dictionary calculations with explicit i64 checks. Fixed seed 20261012 cases
and hand-calculated vectors precede AIL implementation. Inputs are frozen before
byte-identical integration. Full contract:
docs/superpowers/specs/2026-10-12-csv-aggregation-design.md. Preparation hashes
and independent checks are recorded separately from the benchmark inputs.

On Python versions whose csv module rejects NUL, the oracle substitutes an
absent non-syntax scalar only around csv.reader and restores it afterward.
The original complete-input grammar and exact decoded field values are retained.
