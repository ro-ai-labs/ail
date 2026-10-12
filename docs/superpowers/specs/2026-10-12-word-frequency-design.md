# Word frequency and a bounded stdin tool written in AIL

## Intent and choice

Continue the authorized Specs → checked Core → ASD-STE100-based STE → VM/native
language-development goal after the completed CSV milestone. Add a reusable pure
AIL text-analysis library and a stdin tool for planned real-program rung R021.
The measured result must be executable counts and rankings, not generated reports
about capability. The overall Python-replacement and AI-agent language goal stays
active; this milestone does not satisfy the full alpha-publication gate.

This is an architectural library/application addition with a written spec and
plan. The standing instruction to continue reversible work supplies execution
authority; no additional design or permission pause is needed. Preserve the
established independent-oracle, sequential implementation and fresh-review method.
No git mutations, worktrees, dependencies, publication or unrelated refactors.

Considered approaches: adding a host word-count primitive bypasses language
composition; adding broad Unicode case-folding/category APIs expands the compiler
without being necessary for token frequencies. Use existing bulk text, List<Text>,
Json, checked integers, predicates and ordinary functions in pure AIL. This gives
agents a precise reusable operation while measuring another complete program.
Compiler/runtime changes require a reproduced defect and separately reviewed
scope decision; they are not part of the intended implementation.

## Token contract

A word is a maximal nonempty sequence of Unicode scalars outside the pinned AIL
whitespace set:

- U+0009–000D, U+001C–0020, U+0085, U+00A0, U+1680;
- U+2000–200A, U+2028–2029, U+202F, U+205F, U+3000.

These are exactly the 29 scalars documented by existing text trim. Adjacent,
leading and trailing separators create no empty words. Empty/all-whitespace text
has no words. Preserve word spelling and input order in tokenization: no case
folding, Unicode normalization, punctuation stripping, numeric conversion or
language-specific word segmentation. Punctuation, quotes, slashes, tildes,
backslashes, NUL, U+200B and U+FEFF remain ordinary word contents. Combining and
precomposed spellings are distinct. Uppercase and lowercase words are distinct.

Examples, using JSON string notation:

- `" a\tb\r\na "` → `["a","b","a"]`
- `"A a a,"` → `["A","a","a,"]`
- `"x\u200by x\ufeffy"` → `["x\u200by","x\ufeffy"]`
- `"\u00a0\u3000"` → `[]`

## Public library and result

Package `ail.std.words` version 0.1.0, Application profile, at
`examples/ail_std_words.ail`, imported as `Words`:

- `tokens(text: Text) -> List<Text>`: the exact ordered words above.
- `summarize(text: Text) -> Json`: counts every word and returns the summary below.
- `summarizeRequest(text: Text) -> Json`: validates a JSON request, then summarizes
  its selected text value; malformed requests return the fixed failure object.

All three public functions and every helper are pure. They may use ordinary
functions and built-in values, but no host parser, external/process call, I/O,
new opcode, instruction-budget padding or alternate runtime budget.

Counts are exact signed i64 values. Group by exact decoded word. Rank all distinct
words by descending numeric count, then ascending Unicode text order for equal
counts; take the first five. Ties do not depend on first appearance, map insertion
order, encoded JSON spelling or UTF-16 order. `words` is the total token count;
`unique` is the distinct count before truncation. `top` has min(unique,5) entries.

Success key order and compact JSON encoding are exactly:

```json
{"ok":true,"words":3,"unique":2,"top":[{"word":"a","count":2},{"word":"b","count":1}],"error":""}
```

Empty input succeeds with zero words/unique and an empty top array. Text inputs
have no semantic rejection beyond ordinary runtime type/resource failures.

A request is a JSON object containing exactly one unambiguous string member named
`text`. Escaped-equivalent selected names count as duplicates. Ignore unrelated
members, including unrelated duplicate keys, after full JSON syntax and Unicode
validation. Malformed JSON, wrong root/type, missing/duplicated selected member
and NaN/Infinity tokens return exactly:

```json
{"ok":false,"words":0,"unique":0,"top":[],"error":"REQUEST"}
```

JSON syntax numbers in ignored members are not coerced through Float. Invalid
Unicode anywhere in the request rejects. Ordinary fatal JSON depth/value/allocation
bounds propagate, rather than being caught and mislabeled REQUEST. String/control
escaping follows AIL's documented Json transport, including lowercase escapes for
C0/DEL/C1 other than its short quote/backslash/tab/CR/LF escapes.

## Implementation and useful sizes

Prefer bulk trim/replace/split operations over one root-loop iteration per scalar.
One viable tokenizer trims first, normalizes only the pinned delimiters to a single
separator using bulk operations, splits, and filters empty items. Checking for a
needle before replace can avoid unchanged full-text copies. An implementation may
choose a different ordinary AIL algorithm that preserves the contract and sizes.

Counts can use a Json object with correctly escaped JSON Pointer keys (`~` before
`/`), or equivalent ordinary values. Ranking must compare counts numerically and
break ties explicitly or by a proven deterministic traversal. Do not expose raw
map order as ranking. No tiny word/count limit may be added to make tests pass.

Required useful-size checks include single 32,768-scalar ASCII and Unicode words,
1,024 repeated short words, and at least 128 distinct short words with ties and
repeats. Bulk all-whitespace input must retain empty-input semantics at practical
sizes, including 32,768 separators. The independent oracle freezes representative
vectors before AIL source is authored. Existing 1 MiB Text/Json, 65,536 list/JSON
values, separate cumulative 64 MiB arenas, depth/frame and instruction fuel limits
remain. Immutable values may copy; no linear-time, reclamation, arbitrary input
size, universal Python tokenization or general NLP claim is made.

## Executables

`examples/word-frequency` imports the library and owns I/O:

- `Analyze`: `data.request: Text` → Words.summarizeRequest → selected
  `out.result: Json` only. This supplies safe JSON transport for benchmark argv,
  including embedded NUL and multiline text.
- `Count`: read bounded raw stdin through EOF, call Words.summarize, print exactly
  compact result JSON plus one LF. Even empty stdin produces one success result.
  `--stdio` grants streams and suppresses automatic reports. Stdin is raw text,
  not a JSON request or a line-by-line protocol. Missing grants, invalid UTF-8,
  oversize input and fatal runtime limits fail normally, with no result stdout.

Include a meaningful Unicode/tie/punctuation sample, exact expected JSON+LF,
source/VM/native usage and an exact checked-in STE export. No Python interpreter
is required by generated executables. General file reporting and JSON-lines
protocols are outside this small tool's scope; existing applications cover them.

## Independent measurement and review

Create `bench/R021-word-frequency/{intent.md,reference.py,cases/*.args}` first,
under staging until independently reviewed. The oracle derives tokenization from
an explicit regex character class, uses Python Counter and sorts by (-count,word).
It must preserve request duplicate information and number-token typing, validate
Unicode, and emit the documented AIL-compatible JSON encoding independently.
Use manually calculated checks and seeded cases, not results copied from AIL.
Show missing-program RED through the unchanged bench evaluator; freeze exact
inputs and fingerprint before implementation. Old rung inputs/programs stay fixed.

Cases exercise every delimiter, non-delimiter controls/Unicode, punctuation,
case/spelling distinctions, pointer metacharacters, empty/one/five/more-than-five
unique words, numerical count ordering, ties at the cutoff and across permutations,
request validation/ambiguity and required sizes. Benchmark program Analyze imports
the same library as the app. The rung measures request-to-summary through argv;
independent actual stdin checks separately establish R021's from-stdin behavior.
Do not change the existing benchmark runner to add a second transport.

Independent acceptance executes source, saved Core, rendered Spec and STE on
VM/native, with separately executed and natively compiled saved bytecode. Obtain
all frozen expectations from the independent oracle and direct token vectors from
hand calculations. Compare selected result bytes exactly; inspect real stdin/stdout,
status/stderr, grants, invalid UTF-8 and bounded-resource errors. Test Count with
Unicode, NUL, all whitespace, no final LF, multiple lines and JSON-looking raw text.
Deliberately mutate one ranking/count decision in an isolated copied library and
prove correct status plus wrong output is rejected on VM and native.

Task reviews and one whole-milestone review precede the capability lock update.
If compiler/runtime inputs remain unchanged, use new focused acceptance, affected
package/docs tests, links, exact STE and full capability scoring; do not repeat
unrelated completed compiler/differential suites. The controller records the new
score only after required gates, retaining old fingerprints and passing entries.
