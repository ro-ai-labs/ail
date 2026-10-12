# Lossless JSON pretty printing

[Pretty](../ail_std_json_pretty.ail/README.md) is a reusable pure AIL formatter.
This application formats one raw JSON document from stdin and provides a separate
request action for machine acceptance. Generated Linux executables need no Python.

## Purpose

Two-space indentation and LF layout preserve array order, ordered duplicate object
pairs, every number token and decoded strings. Empty containers stay `[]` and `{}`.
The library adds no final LF; Format adds exactly one. Nonempty containers put
one element/member on each line; a member colon has one following space. There
are no blank lines, trailing spaces or trailing commas. Input escapes and source
whitespace may normalize according to the existing Json canonical encoder.

## Source and native commands

After `cargo build`, run from the repository root:

```sh
target/debug/ail ail-run examples/json-pretty --action Format --stdio < examples/json-pretty/input.json
target/debug/ail ail-compile examples/json-pretty --action Format --target linux-x86_64-elf --out /tmp/json-pretty
/tmp/json-pretty --stdio < examples/json-pretty/input.json
```

The [input](input.json) and exact [output](expected.json) demonstrate nested and
root duplicate keys, escaped-equivalent names, Unicode normalization variants,
empty containers, `-0`, `1.2500e-2` and `1E+0003`. Successful Format has status zero
and empty stderr. Raw stdin is one complete document, including scalar roots.
Wrapper-looking objects are formatted as objects; two documents separated by LF
are rejected. Already-pretty input and input without a final LF work.

## STE commands

The checked-in [STE](../ste/json-pretty.ail-ste) contains both actions and the
imported pure library:

```sh
target/debug/ail ail-vm examples/ste/json-pretty.ail-ste --action Format --stdio < examples/json-pretty/input.json
target/debug/ail ail-compile examples/ste/json-pretty.ail-ste --action Format --target linux-x86_64-elf --out /tmp/json-pretty-ste
/tmp/json-pretty-ste --stdio < examples/json-pretty/input.json
```

## Request transport

Analyze calls `Pretty.renderRequest`, compact-formats its Json result, encodes it
as UTF-8 Bytes, selects only `out.result` and records `Rendered`:

```sh
target/debug/ail ail-run examples/json-pretty --action Analyze 'data.request={"json":"[1,-0,{}]"}'
target/debug/ail ail-compile examples/json-pretty --action Analyze --target linux-x86_64-elf --out /tmp/json-pretty-analyze
/tmp/json-pretty-analyze 'data.request={"json":"[1,-0,{}]"}'
target/debug/ail ail-vm examples/ste/json-pretty.ail-ste --action Analyze 'data.request={"json":"[1,-0,{}]"}'
```

The complete selected state is `out.result=hex:<lowercase hex>`. Decode the bytes
as UTF-8 to obtain compact `{"ok":true,"text":"...","error":""}` Json; parse its
`text` string to obtain pretty text. Bytes keep U+2028/U+2029 and fake state-like
string contents intact under line-based state readers. Ordinary VM success/trace
reports use stdout; native action/trace reports use stderr. `--stdio` grants the
standard streams and suppresses these automatic reports for Format.

## Errors and bounds

The request must be an object with exactly one decoded string member `json`.
Wrapper syntax, Unicode, root, missing/wrong-type or ambiguous selected member
errors return `{"ok":false,"text":"","error":"REQUEST"}`. Unrelated members and
duplicates are ignored only after full wrapper validation. Selected malformed or
empty document text returns the same shape with `"error":"JSON"`; wrapper errors
have priority. Fatal resource errors propagate through all APIs.

Format rejects empty/whitespace-only input, invalid JSON/Unicode, non-JSON
whitespace and invalid UTF-8. Missing `--stdio` fails `CAPABILITY`; invalid UTF-8
fails `TEXT`; raw 1 MiB+1 fails `TEXT-LIMIT`. Exactly 1 MiB total consisting of
`null` and JSON whitespace succeeds. 65 open containers fail `JSON-LIMIT`; a
4000-scalar array exhausts instruction fuel. A 524673-byte array of 128 strings
of 4096 ASCII scalars reaches `TEXT-LIMIT` during formatting. These failures have
nonzero status, no result stdout and no private input-value leakage. All fallible
validation/formatting occurs before writing; arbitrary write failures have no
transactional rollback guarantee.

Existing limits include 1 MiB Text/Json values and results, 65536 Json values,
64 open containers/call frames, separate cumulative 64 MiB Text and composite
arenas and instruction fuel. Immutable construction copies values; pretty layout
and result-string escaping expand output. Useful successful vectors include
separate 32768-scalar ASCII/U+754C values and keys, a 5000-digit number, large
exponent spelling, 128 mixed scalar items/duplicate pairs and 64 nested arrays.
These measured sizes do not imply arbitrary-size or linear-time behavior.

## Independent acceptance

```sh
python3 -B scripts/ail_json_pretty_differential.py --artifact-dir /tmp/json-pretty-acceptance
```

The optional directory must initially be empty. Acceptance retains actual
commands, status/output hashes and bytes, independent expectations and generated
artifacts. It freezes/rechecks exact oracle members/fingerprint, library,
semantic dependencies and compiler. Fourteen app routes test all 162 frozen
requests and raw stdin; fourteen direct-render routes test independent pretty
bytes. Routes cover source, saved Core and rendered Spec, each direct/STE on
VM/native, plus separately executed and natively compiled saved bytecode. Exact
ordinary reports are pinned before matrices. Pure semantic IR and a successful
but wrong copied-library layout mutation are checked on both backends.

See the [language reference](../../docs/ail/63-json-pretty.md) for the public API.
