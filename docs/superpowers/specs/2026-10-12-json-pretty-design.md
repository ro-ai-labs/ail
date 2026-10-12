# A lossless JSON pretty-printer written in AIL

## Intent and choice

Continue the authorized Specs → checked Core → ASD-STE100-based STE → VM/native
language-development goal after the completed 21/21 word-frequency milestone.
Implement planned real-program rung R023 as a reusable pure AIL formatter and a
bounded stdin tool for human/agent-readable JSON. This is a new library/application
interface with a written design and plan. Standing continuous reversible-work
authority supplies execution authority; no repeated design/permission pause,
git mutation, dependency installation or publication is needed.

The existing Json representation preserves object order, duplicate members and
number tokens. Pretty-printing must retain those properties. Considered approaches:
a host pretty-print primitive bypasses AIL composition; recursive JSON Pointer
traversal cannot access duplicate object names without ambiguity. Prefer validating
and compact-formatting Json through existing operations, then inserting layout
whitespace outside strings with ordinary AIL text/control operations. This adds
no compiler operation and preserves duplicate members without keyed lookup.
The formatter may use another pure algorithm satisfying the exact contract.

Local semantic references are [recursive JSON](../../ail/42-recursive-json.md),
[JSON inspection](../../ail/47-json-inspection.md),
[text operations](../../ail/37-text-operations.md) and
[text transformations](../../ail/49-text-transformations.md).
No compiler/runtime change is intended; any reproduced defect needs separate scope
and review. The broader Python-replacement/AI-agent goal remains active, and this
rung alone does not satisfy the complete alpha-publication gate.

## Exact layout and preservation

The result is deterministic pretty JSON with these rules:

- Indent by two ASCII spaces per open nonempty container. The root starts at
  indentation zero. Use LF only for layout; no trailing spaces or blank lines.
- Empty arrays and objects are exactly `[]` and `{}`, including when nested.
- Nonempty arrays place one complete element on each indented line. Nonempty
  objects place one member on each indented line, with a colon and one ASCII space
  between its encoded key and value. Nested values apply the same rules.
- Place a comma after every element/member except the last, then LF. Opening
  nonempty delimiters are followed by LF; closing delimiters occupy their own
  line at the parent's indentation. There is no trailing comma.
- Scalar roots are one JSON value with no layout whitespace. Library results
  have no final LF. The stdin action adds exactly one LF after a successful result.

For `{"a":[1,{},[]],"a":-0}`, the library returns exactly:

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

Preserve array order, object member-pair order and every duplicate decoded member
name. Escaped-equivalent keys remain separate pairs even though their canonical
encoded spelling becomes identical. Preserve every JSON number token byte for
byte: `-0`, `1.2500e-2`, `1E+0003`, large integers and large exponent spellings
must never pass through Int or Float conversion. Preserve decoded strings exactly,
including case, normalization variants, Unicode, NUL, slashes, tildes, quotes,
backslashes and punctuation that looks like JSON structure.

String/key encoding follows existing AIL Json canonical transport: quote/backslash
and tab/CR/LF have their documented short escapes; other C0/DEL/C1 controls use
lowercase four-digit Unicode escapes; other Unicode remains literal UTF-8.
Input escape spellings, insignificant whitespace and solidus escapes may normalize
as the existing Json formatter specifies. This is lossless for the documented
Json value representation, not byte-for-byte preservation of original source.
No key sorting, deduplication, Unicode normalization or number normalization occurs.
Pretty output must parse to the same ordered-pair/number-token representation.

## Pure library API

Package `ail.std.json.pretty` version 0.1.0, Application profile, directory
`examples/ail_std_json_pretty.ail`, imported as `Pretty`:

- `render(value: Json) -> Text`: produce the exact layout above for any valid
  Json root, including null, Boolean, number, string, array and object.
- `renderText(text: Text) -> Text`: completely parse one JSON document, then render
  it. Malformed JSON, invalid JSON Unicode or empty input raises ordinary catchable
  `JSON`. Existing fatal resource failures propagate.
- `renderRequest(text: Text) -> Json`: validate the request wrapper below, then
  return a structured render result. Keep wrapper and document failures distinct.

Every helper and public function is pure. Existing compact parse/format operations
are allowed; no host pretty-printer, process/external call, I/O, new opcode,
alternate budget, artificial word/token cap or instruction-padding helper is allowed.
Use coherent helpers and bulk runs/bounded windows where appropriate. Required
long strings must work without one root-loop iteration per Unicode scalar.

A request is one JSON object containing exactly one unambiguous string member
named `json`. Escaped-equivalent selected keys count as duplicates. Fully validate
all wrapper syntax and Unicode before selection. Ignore unrelated members and
unrelated duplicate names after that validation; retain ignored number-token typing
without Int/Float coercion. Invalid wrapper/root/type, absent or ambiguous selected
member, invalid wrapper Unicode and NaN/Infinity tokens return `REQUEST`.
The selected string is the complete JSON document to render. Malformed document
syntax/Unicode, empty text or NaN/Infinity there returns `JSON`. Wrapper errors have
priority over document errors. Documents may contain duplicate keys at any depth.

Success key order is `ok`, `text`, `error`, with compact encoded result:
`{"ok":true,"text":"<the exact pretty result>","error":""}`.
Failure has exactly `{"ok":false,"text":"","error":"REQUEST"}` or
`{"ok":false,"text":"","error":"JSON"}`. Pretty newlines appear as escaped
newlines inside this Json result's text string. Fatal depth/value/Text/List/Json/
allocation/frame/fuel limits remain failures, never structured domain errors or
successful partial output. Recovery must be narrow enough to preserve that rule.

## Useful sizes and ordinary bounds

Freeze independent examples before implementing AIL. Required successful examples:

- Single 32,768-scalar ASCII and U+754C string values, plus separate documents with
  32,768-scalar ASCII and U+754C object keys. Keep each argv fixture within the OS
  per-argument bound; do not combine large key and value in one argv fixture.
- A 5,000-digit integer and a large exponent token, with spelling preserved.
- At least 128 scalar array values and 128 object member pairs, with duplicate
  keys and mixed kinds represented among the useful-size cases.
- A chain of 64 open array containers with a scalar leaf; empty-container variants
  and mixed structures at smaller depths must also be covered.

These are useful-size requirements, not arbitrary new limits or a promise that
all documents of those lengths succeed. Existing limits remain: 1 MiB Text/Json
raw/compact values and results, 65,536 Json values, 64 open containers/call frames,
separate cumulative 64 MiB Text and List/Json arenas, and instruction fuel.
Immutable construction can copy values; do not claim linear time, reclamation or
unbounded formatting. Pretty whitespace and result-string escaping can expand
output beyond an input that fits. Observe and report actual fatal categories.

## Application and transport

`examples/json-pretty` imports the library:

- `Analyze`: `data.request: Text` → Pretty.renderRequest → compact Json text →
  UTF-8 Bytes, selecting only `out.result: Bytes` and recording `Rendered`. Runtime
  state is `out.result=hex:<lowercase UTF-8 bytes of the compact structured result>`.
  This existing transport keeps every byte observable by the unchanged line-based
  benchmark, including U+2028/U+2029 inside strings. The pure request API still
  returns Json; this action encodes its result for exact machine transport.
- `Format`: read bounded raw UTF-8 stdin through EOF → Pretty.renderText → write
  exact pretty text plus one LF. Empty input is invalid JSON. Raw stdin is the
  JSON document itself, not a request wrapper or JSON-lines protocol.

`--stdio` grants standard streams and suppresses automatic reports. Missing grants,
invalid UTF-8, malformed JSON/Unicode, oversize input and fatal bounds produce a
nonzero status with no result stdout or private input-value leakage. All fallible
validation/formatting precedes the output write; this is not a transactional
rollback guarantee for an arbitrary failure during a write.

Include meaningful nested/duplicate/Unicode/lossless-number sample JSON and exact
pretty output, source/VM/native commands and an exact checked-in STE export. No
caller-file output, selectable indentation, key sorting or JSON-lines interface
is included in this milestone. Generated native executables need no Python.

## Independent validation and integration

Create staged `bench/R023-json-pretty/{intent.md,reference.py,cases/*.args}` first.
The independent Python oracle uses an ordered object-pairs representation distinct
from arrays, opaque number tokens and full Unicode validation. Render the parsed
tree recursively with an independent string encoder; do not copy an AIL scanning
algorithm or expected outputs from AIL. Compare hand-calculated examples and
reparsed ordered-pair/number-token invariants, including seeded recursive trees.
Record actual missing-program RED via the unchanged benchmark evaluator, review,
then freeze exact case membership/hashes/fingerprint before AIL implementation.
All case files explicitly set `data.request`, including empty invalid requests.

Exercise all layouts/root kinds, nesting, empty containers, duplicate/escaped keys,
number spellings, every encoded control, Unicode normalization/case, delimiters
inside strings, odd/even backslash runs, wrapper versus document failures and
required useful sizes. Literal UTF-8 argv is needed for long Unicode strings;
escape U+0085/U+2028/U+2029 in fixture files for the unchanged splitlines loader.

The unchanged benchmark state extractor uses Python splitlines and can truncate
literal U+2028/U+2029 in Json output. A preimplementation transport probe records
this at /tmp/ail-pretty-transport-probe.json. Analyze uses existing Bytes transport
to make its complete structured result one unambiguous ASCII state line. No old
runner edit, changed string encoding or excluded Unicode case is required.

Direct-library checks establish render/renderText behavior and resource propagation.
Independent acceptance uses 14 app routes and 14 direct-render probe routes:
source, saved Core, rendered Spec, each direct and STE on VM/native, plus separately
executed and natively compiled saved bytecode. All frozen request cases and direct
hand/constructed render expectations compare exact encoded results. Direct probes
encode returned pretty Text as UTF-8 Bytes for complete state-line comparison. Inspect pure
semantic IR and require finite subprocess deadlines with cleanup.

Real Format stdin checks include scalar roots, nested duplicates, pretty input,
Unicode/NUL-escape strings, no final LF, empty/malformed input and useful sizes.
An exactly 1 MiB stdin document consisting of a small valid root and JSON
whitespace must succeed; 1 MiB+1 stdin must fail. Excessive Json depth and a representative formatting-expansion/
allocation bound must remain fatal with no stdout. Deliberately mutate a real
layout decision in an isolated library copy and reject the successful wrong output
on both backends using independently expected bytes.

All suffix-named packages need the seven standard learning-guide sections from
`tests/ail_toolchain.rs`, including the new library from Task2 onward. Register the
new library as support-only with existing catalog/story/request/response counts
unchanged at 128; the resulting actual `.ail` package count is 32. Run the four
focused inventory/guide/support/STE tests, actual support-loader closure probes,
local links, exact export and sample commands. Preserve existing compiler/runtime,
tests, old scripts/rungs and dependencies; unrelated completed suites need not
repeat when those inputs remain unchanged. Three task reviews and a fresh whole-
milestone review precede the guarded capability lock update from 21 to 22.
