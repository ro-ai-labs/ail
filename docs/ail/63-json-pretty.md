# Lossless JSON pretty printing

The pure [Pretty library](../../examples/ail_std_json_pretty.ail/README.md) supplies
JSON layout entirely in AIL. The [application guide](../../examples/json-pretty/README.md)
contains tested source, STE and native raw-stdin/Analyze commands, exact samples,
transport decoding and independent acceptance instructions.

## Public API

Package `ail.std.json.pretty` 0.1.0 uses the Application profile:

```text
imports: ../ail_std_json_pretty.ail compatible ^0.1 as Pretty
```

The path is relative to the importing package. Every public function and helper
is pure; compact Json parse/format, Text and control operations supply the result.

| Function | Result |
| --- | --- |
| `Pretty.render(value: Json) -> Text` | Exact pretty text for any valid Json root. |
| `Pretty.renderText(text: Text) -> Text` | Fully parses one JSON document, then renders it; ordinary syntax/Unicode failures raise catchable `JSON`. |
| `Pretty.renderRequest(text: Text) -> Json` | Validates a request wrapper and returns ordered `ok`, `text`, `error` members. |

## Layout and semantic preservation

Use two ASCII spaces per open nonempty container and LF only. Scalar roots have
no layout whitespace. Empty containers remain `[]` and `{}` at every depth.
Nonempty containers start with an opening delimiter and LF, place each complete
element/member on an indented line, then close at the parent indentation. Separate
members with comma then LF; put one space after each colon. No blank lines,
trailing spaces or commas occur. Library results have no final LF; Format adds one.

```json
{
  "a": [
    1,
    {},
    []
  ],
  "a": -0
}
```

This is the exact result for `{"a":[1,{},[]],"a":-0}`. Array order and object
pair order, including duplicate decoded names at every depth, survive unchanged.
Escaped-equivalent keys may get the same canonical encoded spelling but remain
separate pairs. Number tokens such as `-0`, `1.2500e-2`, `1E+0003`, a 5000-digit
integer and large exponents retain every token byte without Int/Float conversion.
Strings preserve decoded Unicode scalars, case and normalization variants.

Canonical Json encoding escapes quotes/backslashes and uses short tab/CR/LF
escapes. Other C0/DEL/C1 controls use lowercase four-digit Unicode escapes; other
Unicode remains literal UTF-8. Input escape spelling, solidus escapes and
insignificant whitespace may normalize. The ordered-pair/number-token Json value
representation is preserved; source text is not retained byte for byte.

## Requests and raw stdin

A request is one object with exactly one unambiguous string member whose decoded
name is `json`. Validate all wrapper syntax and Unicode first. Root/type/selection
and malformed-wrapper errors return `{"ok":false,"text":"","error":"REQUEST"}`.
Unrelated members and duplicate names are ignored after validation. Selected
malformed/empty document text returns `{"ok":false,"text":"","error":"JSON"}`.
Wrapper errors take priority. Success is `{"ok":true,"text":"...","error":""}`,
with escaped layout newlines inside `text`. Fatal bounds never become domain errors.

Analyze selects only the UTF-8 Bytes of the compact structured result and records
`Rendered`. Its single lowercase `out.result=hex:` line preserves every byte,
including line separators and strings that resemble state reports. Decode hex,
then UTF-8, then compact Json to obtain `text`. Ordinary VM reports use stdout and
native reports use stderr; fixed reports reveal no unselected private state.

Format reads bounded raw UTF-8 stdin through EOF. It accepts one JSON document,
including scalar roots and wrapper-looking objects. Two documents separated by
LF, empty/whitespace-only input, invalid JSON/Unicode and NBSP outside a string
fail. `--stdio` grants streams and suppresses automatic reports. Success has
status zero, exact pretty bytes plus one LF and empty stderr. Validation and
formatting precede writing; arbitrary output-write failures are not transactional.

## Measured bounds and verification

Useful successful cases include separate 32768-scalar ASCII/U+754C values and
keys, 5000-digit integers, opaque large exponents, 128 mixed array values and
128 ordered duplicate member pairs, and 64 open arrays with a scalar leaf.
Exactly 1 MiB total raw stdin containing `null` plus JSON whitespace succeeds.
1 MiB+1 fails `TEXT-LIMIT`; missing streams fail `CAPABILITY`; invalid UTF-8 fails
`TEXT`. Depth 65 fails `JSON-LIMIT`, a 4000-scalar array exhausts instruction fuel,
and 128 strings of 4096 ASCII scalars (524673-byte document) reach `TEXT-LIMIT`
during formatting. Fatal checks require nonzero status, no result stdout and no
private input-value leakage.

Existing 1 MiB Text/Json values/results, 65536 Json values, 64 open containers/
frames, separate cumulative 64 MiB Text/composite arenas and fuel still apply.
Immutable construction may copy values; pretty whitespace and escaping may
expand output. Measured useful sizes imply no linear-time, reclamation or
unbounded-input guarantee.

The permanent [acceptance script](../../scripts/ail_json_pretty_differential.py)
uses the frozen independent [R023 oracle](../../bench/R023-json-pretty/reference.py):
ordered `Object.pairs`, opaque `Number.lexeme` and an independent recursive
renderer. All 162 requests are checked on 14 app routes; independent direct
render vectors use 14 probe routes. Raw success/error/bound checks cover every
app route. Source, saved Core, rendered Spec and STE run on VM/native, and saved
bytecode is separately executed and natively compiled. Every selected Bytes
payload is compared completely; ordinary reports are frontloaded, semantic IR
is checked for purity, and a copied-library layout mutation must produce successful
wrong bytes rejected by the oracle. Every subprocess has a finite deadline and
cleanup; optional retained artifacts record actual commands/statuses/I/O hashes.
