# Exact word frequency and bounded stdin

The pure [Words library](../../examples/ail_std_words.ail/README.md) supplies exact
Unicode whitespace tokenization, frequency counts and top-five ranking entirely
in AIL. The [application](../../examples/word-frequency/README.md) owns stdin and
stdout. Generated Linux executables need no Python interpreter or host word parser.

## Public API

Package `ail.std.words` version 0.1.0 uses the Application profile:

```text
imports: ../ail_std_words.ail compatible ^0.1 as Words
```

The path is relative to the importing package. All three functions have no effects:

| Function | Result |
| --- | --- |
| `Words.tokens(text: Text) -> List<Text>` | Ordered exact words; no empty tokens. |
| `Words.summarize(text: Text) -> Json` | Successful count/unique/top-five object. |
| `Words.summarizeRequest(text: Text) -> Json` | Validates a JSON request with one unambiguous string `text`, then summarizes it. |

A word is a maximal nonempty sequence of Unicode scalars outside the pinned AIL
trim whitespace set: U+0009–000D, U+001C–0020, U+0085, U+00A0, U+1680,
U+2000–200A, U+2028–2029, U+202F, U+205F, U+3000. These are exactly 29 scalars.
Adjacent, leading and trailing separators create no empty words. Empty and
all-whitespace text produce an empty token list and a successful zero summary.

Spelling and token order survive unchanged. There is no case folding,
normalization, punctuation stripping, numeric conversion or language-specific
segmentation. NUL, U+200B, U+FEFF, quotes, slashes, tildes and backslashes remain
ordinary word contents. For example, using JSON string notation:

| Text | Tokens |
| --- | --- |
| `" a\tb\r\na "` | `["a","b","a"]` |
| `"A a a,"` | `["A","a","a,"]` |
| `"x\u200by x\ufeffy"` | `["x\u200by","x\ufeffy"]` |
| `"\u00a0\u3000"` | `[]` |
| `"é e\u0301"` | `["é","e\u0301"]` |

Counts use checked signed i64 arithmetic. Ranking sorts numerically by decreasing
count, then by ascending Unicode scalar text order of the exact word. It retains
at most five entries, including deterministic ties at the cutoff. Success and
request failure have this exact key order and compact encoding:

```json
{"ok":true,"words":3,"unique":2,"top":[{"word":"a","count":2},{"word":"b","count":1}],"error":""}
{"ok":false,"words":0,"unique":0,"top":[],"error":"REQUEST"}
```

Top entries have `word`, then `count`. JSON control characters are escaped;
non-control Unicode is emitted as UTF-8. The request wrapper validates the entire
JSON and Unicode before selection. It requires an object and exactly one string
member whose decoded name is `text`. Malformed JSON, wrong shape, absent/wrong-type
selected members and duplicate selected members (including escaped equivalent
keys) return `REQUEST`. Unrelated members and their duplicates are ignored after
validation. Json number tokens remain numbers, even if their spelling resembles
an integer. Fatal JSON depth, value, allocation and instruction limits propagate;
they never turn into successful partial counts.

## Executable actions

After `cargo build`, Analyze selects only `out.result`. Requests use JSON transport
so embedded NUL and line breaks can be carried safely through argv:

```sh
target/debug/ail ail-run examples/word-frequency --action Analyze 'data.request={"text":"b a a"}'
target/debug/ail ail-compile examples/word-frequency --action Analyze --target linux-x86_64-elf --out /tmp/word-frequency-analyze
/tmp/word-frequency-analyze 'data.request={"text":"b a a"}'
```

Ordinary VM reporting includes success and trace lines on stdout; native action
and trace reporting uses stderr. Analyze selects no private request fields.
Count reads all bounded raw UTF-8 stdin through EOF and calls `Words.summarize`.
It formats the result and appends exactly one LF. Empty input still emits a zero
summary, multiple lines form one text, and JSON-looking text is ordinary word
content. For example, `{"text":"a a"}` is two words, `{"text":"a` and `a"}`.
Bare `--stdio` grants standard streams and suppresses automatic reports:

```sh
target/debug/ail ail-run examples/word-frequency --action Count --stdio < examples/word-frequency/input.txt
target/debug/ail ail-compile examples/word-frequency --action Count --target linux-x86_64-elf --out /tmp/word-frequency
/tmp/word-frequency --stdio < examples/word-frequency/input.txt
```

The Unicode/case/punctuation sample has nine words, six unique words and ties;
[expected.json](../../examples/word-frequency/expected.json) contains its exact
compact output plus one LF. Successful Count has status zero and empty stderr.
The checked-in [STE export](../../examples/ste/word-frequency.ail-ste) includes
all imported functions and both actions:

```sh
target/debug/ail ail-vm examples/ste/word-frequency.ail-ste --action Count --stdio < examples/word-frequency/input.txt
target/debug/ail ail-compile examples/ste/word-frequency.ail-ste --action Count --target linux-x86_64-elf --out /tmp/word-frequency-ste
/tmp/word-frequency-ste --stdio < examples/word-frequency/input.txt
```

## Measured scope and bounds

Independent acceptance executes all 75 frozen R021 requests across 14 application
routes: source, saved Core and rendered Spec, each direct and through STE, on VM
and native; separately saved source bytecode is also executed and native compiled.
A separate token probe uses the same 14 routes with hand-calculated/constructed
vectors, including all 29 separators and preserved control/pointer/Unicode words.
Raw stdin success/fatal checks cover every application route. A copied library's
ranking decision is deliberately reversed; successful but wrong VM/native results
must differ from the independent oracle. Semantic IR is checked for library
purity and absence of external/process/host operations.

```sh
python3 -B scripts/ail_word_frequency_differential.py --artifact-dir /tmp/word-frequency-acceptance
```

The optional artifact directory must initially be empty. It retains commands,
statuses, input/output bytes and hashes, independent expectations, compiled
artifacts and mutation evidence. Every subprocess has a finite deadline through
the existing bounded process harness; acceptance rechecks the frozen oracle,
library, benchmark program and compiler identities.

Useful-size vectors cover 32,768-scalar ASCII and Unicode words, 32,768 separators,
1,024 repeated short words, and 128 distinct short words with ties and repeats.
Exactly 1 MiB of ASCII spaces succeeds with zero words. A raw input of 1 MiB+1
bytes fails `TEXT-LIMIT`; invalid UTF-8 fails `TEXT`; missing `--stdio` fails
`CAPABILITY`. All fatal checks require no result stdout and no private input value
in diagnostics. A 174,763-byte NUL-only word is valid Text but fails `LIST-LIMIT`
because its escaped list representation exceeds 1 MiB. An Analyze request with
65 nested unused arrays fails `JSON-LIMIT` despite a valid selected text.

Existing limits remain: 1 MiB Text/Json/list encodings, 65,536 list items/JSON
values, separate cumulative 64 MiB text and composite arenas, 64 call frames and
instruction fuel. Inputs beneath the stdin bound can still exhaust a later
limit. Whole-input buffering and immutable list/map copies mean that these
measured sizes do not imply linear-time, reclamation or arbitrary-size behavior.
The word definition is an exact contract, not general NLP or universal Python
word/tokenization behavior.
