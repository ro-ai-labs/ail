# Word frequency

Count exact words from raw stdin with the pure [Words library](../ail_std_words.ail/README.md).
Build the compiler with `cargo build`, then run the VM or a generated Linux executable:

```sh
target/debug/ail ail-run examples/word-frequency --action Count --stdio < examples/word-frequency/input.txt
target/debug/ail ail-compile examples/word-frequency --action Count --target linux-x86_64-elf --out /tmp/word-frequency
/tmp/word-frequency --stdio < examples/word-frequency/input.txt
```

Count reads bounded UTF-8 stdin through EOF and emits one compact JSON result plus
one LF, including for empty input. Bare `--stdio` grants streams and suppresses
automatic reports. Successful execution has status zero and empty stderr. Multiple
lines form one text; JSON-looking input is ordinary words. The sample includes
Unicode, case, punctuation and tied counts. Its [expected output](expected.json) is:

```json
{"ok":true,"words":9,"unique":6,"top":[{"word":"apple","count":2},{"word":"é","count":2},{"word":"🙂","count":2},{"word":"Apple","count":1},{"word":"apple,","count":1}],"error":""}
```

The exact [STE export](../ste/word-frequency.ail-ste) contains the library and both
actions. It executes through the same VM/native pipeline:

```sh
target/debug/ail ail-vm examples/ste/word-frequency.ail-ste --action Count --stdio < examples/word-frequency/input.txt
target/debug/ail ail-compile examples/ste/word-frequency.ail-ste --action Count --target linux-x86_64-elf --out /tmp/word-frequency-ste
/tmp/word-frequency-ste --stdio < examples/word-frequency/input.txt
```

Analyze accepts a JSON request through `data.request: Text` and selects only
`out.result: Json`. Embedded NUL uses a JSON string escape in the argument:

```sh
target/debug/ail ail-run examples/word-frequency --action Analyze 'data.request={"text":"b a a"}'
target/debug/ail ail-compile examples/word-frequency --action Analyze --target linux-x86_64-elf --out /tmp/word-frequency-analyze
/tmp/word-frequency-analyze 'data.request={"text":"b a a"}'
```

Selected result:
`out.result={"ok":true,"words":3,"unique":2,"top":[{"word":"a","count":2},{"word":"b","count":1}],"error":""}`.
Analyze records `Analyzed`; Count records `Counted`. Ordinary VM reports use stdout;
native action and trace reports use stderr. The stdin commands above suppress them.

Words are maximal nonempty runs outside the pinned 29 whitespace scalars. Case,
punctuation, NUL, BOM, zero-width space and Unicode spelling survive. Ranking uses
numeric count descending and Unicode scalar text order ascending, taking five
entries. The request wrapper requires one unambiguous string `text`; malformed,
missing, wrong-type or duplicate selected members produce `REQUEST`. Unrelated
members are ignored after full JSON/Unicode validation.

Fresh acceptance covers 32,768-scalar ASCII/Unicode words, 32,768 separators,
1,024 repeated words and 128 distinct words with ties/repeats. Exactly 1 MiB of
ASCII spaces succeeds empty; 1 MiB+1 raw bytes fail `TEXT-LIMIT`. Missing `--stdio`
fails `CAPABILITY`; invalid UTF-8 fails `TEXT`. Valid input may hit ordinary value,
allocation, depth or fuel limits before that input bound: 174,763 NUL bytes fail
`LIST-LIMIT` when their escaped list representation exceeds 1 MiB. Fatal failures
emit no result stdout. This is a bounded whole-input action. Immutable values
copy; the useful-size checks do not establish linear-time or unbounded behavior.

Run independent request, token, raw I/O and semantic mutation acceptance with:

```sh
python3 -B scripts/ail_word_frequency_differential.py --artifact-dir /tmp/word-frequency-acceptance
```

The artifact directory must initially be empty. See the
[reference](../../docs/ail/62-word-frequency.md) for the API, exact separator set,
request semantics and ordinary runtime budgets.
