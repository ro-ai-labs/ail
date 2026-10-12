rung: R021-word-frequency
tier: 2
action: Analyze
targets: vm,native
blocked-on: pure AIL word tokenization and exact frequency ranking

# R021 — Count exact words and select the five most frequent

Analyze receives `data.request: Text` and selects only `out.result: Json`.
The request is a valid JSON object with exactly one actual string member named
`text`. Escaped-equivalent selected keys are duplicates. Validate the complete
JSON syntax and Unicode before ignoring unrelated members or their duplicates.
Ignored JSON numbers preserve syntax tokens without Float conversion; NaN and
Infinity are invalid. Invalid shape, syntax, Unicode or selected ambiguity returns
`{"ok":false,"words":0,"unique":0,"top":[],"error":"REQUEST"}`.
Ordinary fatal runtime resource bounds remain separate from handled REQUEST.

Words are maximal nonempty scalar sequences outside exactly these 29 separators:
U+0009–000D, U+001C–0020, U+0085, U+00A0, U+1680, U+2000–200A,
U+2028–2029, U+202F, U+205F and U+3000. Preserve case, normalization form,
punctuation, numeric spelling, NUL, U+200B and U+FEFF. Empty/separator-only text
succeeds with zero counts. Counts group exact words; rank by descending numeric
count, then ascending Unicode scalar lexical order; take at most five entries.
`words` counts all tokens and `unique` counts all distinct words before truncation.

Success key order is exactly
`{"ok":true,"words":3,"unique":2,"top":[{"word":"a","count":2},{"word":"b","count":1}],"error":""}`.
Each top object orders word before count. Compact encoding uses short quote,
backslash, tab, CR and LF escapes, lowercase unicode escapes for other C0 and
DEL/C1 controls, and UTF-8 for other scalars. Print exactly one
`out.result=<compact JSON>` assignment and one LF.

The independent Python stdlib oracle uses an explicit character-class regex,
Counter and sort(-count,word); request decoding preserves pairs and numeric token
types. Cases include manually calculated vectors, seed20261012 adversarial
inputs, all delimiters/non-delimiter controls, pointer keys, Unicode ties, a
five-entry cutoff, count10vs9, and invalid request categories. Required useful
sizes are32,768-scalar ASCII/Unicode words,1,024 repeated short words,128 distinct
words with ties/repeats, and32,768 separators. JSON escapes keep all argv inputs
single-line and NUL-free without changing the decoded text; noncontrol Unicode
remains literal to keep useful-size argv below the host single-argument bound.

This rung measures request-to-summary through argv. The independently accepted
Count application will separately establish raw stdin behavior. No benchmark
runner change, host word-count primitive, new opcode or alternate fuel is needed.
Binding contract: docs/superpowers/specs/2026-10-12-word-frequency-design.md.
Task1 preparation and independent evidence remain outside the frozen input tree.
