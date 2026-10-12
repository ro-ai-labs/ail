rung: R023-json-pretty
tier: 2
action: Analyze
targets: vm,native
blocked-on: pure AIL lossless JSON pretty-printer

# R023 — Render lossless JSON with deterministic two-space layout

Analyze reads explicit `data.request: Text`, validates a JSON object containing
exactly one unambiguous actual string member named `json`, then renders that
member's complete JSON document. Fully validate wrapper syntax and Unicode,
including ignored members; escaped-equivalent selected names are duplicates.
Unrelated members and their duplicates are ignored after full validation.
Invalid wrapper/root/type/selection returns REQUEST; malformed selected document
returns JSON. Wrapper invalidity has priority. NaN/Infinity are invalid JSON.

The result has exact ordered keys `ok`, `text`, `error`: success is
`{"ok":true,"text":"<pretty JSON>","error":""}`; failure is
`{"ok":false,"text":"","error":"REQUEST"}` or the same with error JSON.
Analyze encodes the compact result Json as UTF-8 Bytes, selecting only
`out.result: Bytes`, observed as `out.result=hex:<lowercase exact UTF-8 hex>`.
This preserves the complete U+2028/U+2029 result through the unchanged line-based
benchmark extractor; it does not alter canonical Json encoding.

Render every JSON root kind. Retain object member-pair order and duplicate decoded
names, array order, decoded string scalars and every number token's exact spelling.
Use two-space indentation, LF layout, colon plus space and compact empty containers.
Nonempty containers place each member/element on its own indented line and the
closing delimiter at its parent's indentation. No trailing comma, spaces or LF.
Input whitespace and escape spellings canonicalize. Encode quotes/backslashes and
LF/CR/TAB with short escapes; other C0 and DEL/C1 controls use lowercase four-digit
Unicode escapes; other Unicode remains literal. No sorting or normalization.

The independent stdlib Python oracle parses ordered object pairs distinct from
arrays and opaque number tokens distinct from strings/Int/Float, validates all
Unicode recursively, and renders the tree with its own encoder. Fixed cases and
seeded property checks precede AIL source. Useful successes include separate
32768-scalar ASCII/U+754C keys and values, a 5000-digit integer, 128 scalar array
items, 128 mixed duplicate object pairs and 64 open arrays. Fatal resource cases
remain separate runtime acceptance; ordinary bounds are not converted to errors.

Binding contract: docs/superpowers/specs/2026-10-12-json-pretty-design.md.
Independent preparation evidence and frozen hashes live outside this rung.
