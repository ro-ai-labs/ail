# Lossless JSON pretty library

## Purpose

`ail.std.json.pretty` 0.1.0 is a pure Application-profile library imported as
`Pretty`. `render(value: Json) -> Text` formats an existing valid Json value;
`renderText(text: Text) -> Text` parses one complete document before formatting;
`renderRequest(text: Text) -> Json` validates a wrapper and returns ordered
`ok`, `text`, `error` fields. Every function uses ordinary AIL operations.

## Concepts Taught

Canonical compact JSON is scanned outside quoted strings to insert two-space
indentation and LF layout. Empty containers stay compact, members use colon plus
one space, and the result has no final LF. Bounded 256-scalar windows find quoted
and scalar token ends; the root layout loop processes each token in bulk. Quote
termination counts preceding backslashes, including across window boundaries.

Array order, ordered object pairs, duplicate decoded names and exact number tokens
survive. Formatting never looks up inner object members by key or converts numbers
through Int or Float. Canonical encoding preserves decoded Unicode and normalization:
quote/backslash/tab/CR/LF use short escapes, other C0/DEL/C1 controls use lowercase
four-digit Unicode escapes, and other scalars remain literal UTF-8. Input whitespace,
solidus escapes and alternative Unicode escape spellings may normalize.

For `{"a":[1E+0003,{},[]],"a":"界"}`, the exact pretty text is:

```json
{
  "a": [
    1E+0003,
    {},
    []
  ],
  "a": "界"
}
```

The displayed final code-block line break is not part of the library result.

The wrapper must be an object with exactly one unambiguous string member named
`json`. Full syntax and Unicode validation precede selection. Unrelated members
and duplicate unrelated names are accepted. Wrapper failures return `REQUEST`;
invalid selected documents return `JSON`. Success is
`{"ok":true,"text":"<pretty text>","error":""}`; failures have false, empty text
and the code. Narrow JSON/JSON-PATH recovery preserves fatal resource failures.

## Files To Inspect

Read [the manifest](ail-package.md), [pure functions](spec.ail-spec.md),
[the Analyze application](../json-pretty/spec.ail-spec.md), and
[the binding design](../../docs/superpowers/specs/2026-10-12-json-pretty-design.md).
The frozen independent oracle is [reference.py](../../bench/R023-json-pretty/reference.py).

## Expected Replay Artifacts

From the repository root, run:

```bash
target/debug/ail ail-check examples/json-pretty
target/debug/ail ail-run examples/json-pretty --action Analyze 'data.request={"json":"{\"a\":[1,{},[]],\"a\":-0}"}'
target/debug/ail ail-compile examples/json-pretty --action Analyze --target linux-x86_64-elf --out /tmp/json-pretty.elf
/tmp/json-pretty.elf 'data.request={"json":"[1E+0003,-0]"}'
```

Analyze selects only `out.result: Bytes`, containing UTF-8 of the compact result
Json. Its `hex:` state line preserves every byte, including literal U+2028/U+2029
inside strings, through line-based observers. Decode hex to inspect the result.
VM reports a combined trace on stdout; native reports fixed operation/trace lines
on stderr. Neither report exposes private wrapper or document fields.

## Rejected Fixtures

Empty/malformed documents, invalid Unicode and NaN/Infinity are JSON errors.
Invalid wrapper kind, absent/duplicate/incorrectly typed selected member, or invalid
ignored wrapper content are REQUEST errors. Existing limits still apply: 1 MiB
Text/Json values/results, 65,536 Json values, 64 open containers/function frames,
64 MiB cumulative Text and List/Json arenas, and fuel. Narrow catches do not turn
these fatal limits into domain results. Immutable joins copy values; no linear-time,
reclamation or arbitrary-size claim is made. Layout/result escaping may expand input.

## Next Example To Read

Read [json-pretty](../json-pretty/README.md) for the request action and
[JSON inspection](../json-inspection/README.md) for kind/key operations.
The latter's keyed traversal illustrates why duplicate-preserving formatting uses
compact token layout instead.

## v0.3 Learning Signal

The reusable formatter demonstrates pure value-function composition, lossless
transport, narrow recovery, bounded scanning, and independent VM/native byte
comparison without a new compiler opcode or host formatter. Required successes
include separate 32,768-scalar ASCII/U+754C keys and values, 5,000-digit numbers,
128 values/pairs, and 64 open arrays; these examples do not define new semantic caps.
