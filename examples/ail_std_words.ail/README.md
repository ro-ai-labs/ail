# Exact word frequencies

## Purpose

`ail.std.words` 0.1.0 is a pure Application-profile library imported as `Words`.
Its public functions are `tokens(text: Text) -> List<Text>`,
`summarize(text: Text) -> Json`, and `summarizeRequest(text: Text) -> Json`.

## Concepts Taught

Words are maximal nonempty runs outside AIL's pinned 29-scalar trim whitespace
set: U+0009–000D, U+001C–0020, U+0085, U+00A0, U+1680, U+2000–200A,
U+2028–2029, U+202F, U+205F, U+3000. Token order and exact spelling survive;
case, punctuation, NUL, U+200B, BOM and normalization variants remain distinct.
Bulk trim/replace/split operations normalize separators; empty parts are omitted.

Summaries contain `ok`, `words`, `unique`, `top`, `error`, in that order.
The top five entries have `word` and `count`; numeric count descends, then
Unicode scalar text order ascends. Counts use checked signed i64 arithmetic.
Pointer escaping handles tilde before slash. Ranking scans lexically sorted
remaining keys and replaces its winner only for a strictly greater count.

A request is a JSON object with one unambiguous string member named `text`.
Full JSON and Unicode validation precedes selection. Unrelated members and their
duplicates are ignored. Malformed requests return
`{"ok":false,"words":0,"unique":0,"top":[],"error":"REQUEST"}`;
ordinary fatal depth, value, allocation and instruction limits propagate.

Existing 1 MiB value bounds, 65,536 list items/JSON values, separate cumulative
64 MiB text and composite arenas, 64 frames and instruction fuel still apply.
Immutable list/map updates copy values. No arbitrary word cap, linear-time,
unbounded-input, general NLP or Python tokenization claim is made.
See the [application](../word-frequency) and [binding design](../../docs/superpowers/specs/2026-10-12-word-frequency-design.md).

## Files To Inspect

Read [ail-package.md](ail-package.md) for the import/version contract and
[spec.ail-spec.md](spec.ail-spec.md) for bulk tokenization, safe JSON Pointer keys,
checked frequency updates, lexical tie order and request recovery. The
[application](../word-frequency/README.md) owns I/O; the
[reference](../../docs/ail/62-word-frequency.md) gives executable commands and bounds.

## Expected Replay Artifacts

This support-only package has no counted replay-catalog entry. R021 and
[scripts/ail_word_frequency_differential.py](../../scripts/ail_word_frequency_differential.py)
check independently expected requests/tokens across source, saved Core, rendered
Spec, STE and saved bytecode on VM/native. Optional acceptance artifacts retain
real command records, stdin/stdout/stderr bytes, hashes and a successful-status
ranking mutation mismatch. The app's [STE export](../ste/word-frequency.ail-ste)
contains the imported library functions.

## Rejected Fixtures

The independent R021 cases exercise malformed JSON, wrong request shape, missing
or wrong-type selected `text`, and duplicates including escaped equivalent names.
These return the exact REQUEST object. Application acceptance also checks fatal
UTF-8, capability, Text/List encoding and JSON depth limits without partial output.
No package-local rejected conformance fixture is declared here.

## Next Example To Read

Run [word frequency](../word-frequency/README.md) to see the pure functions composed
with bounded raw stdin. Compare [CSV aggregation](../csv-aggregate/README.md) for
another pure text library with request validation and explicit I/O actions.

## v0.3 Learning Signal

Exact Unicode whitespace words, numeric count ordering and deterministic lexical
ties compose through existing Text, List<Text>, Json and value functions. Useful
measured sizes and independent VM/native outputs demonstrate this bounded program;
they do not establish general Python replacement or the alpha-publication gate.
