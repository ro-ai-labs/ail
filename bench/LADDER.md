# The AIL Capability Ladder

The goal function for AIL. See
[`docs/ail/32-capability-ladder-and-improvement-loop.md`](../docs/ail/32-capability-ladder-and-improvement-loop.md).

```bash
python3 scripts/ail_bench.py                  # score every rung
python3 scripts/ail_bench.py --rung R001-add  # one rung
python3 scripts/ail_bench.py --update-lock    # record a new high score
```

## What a rung is

```text
bench/RNNN-name/
  intent.md      metadata header + English statement of required behavior
  reference.py   independent oracle, Python, never AIL
  cases/*.args   'key=value' runtime state, one assignment per line
  program.ail/   the AIL implementation — absent until the language can express it
```

A rung passes only when, for every case:

```text
reference output == VM final state == native final state
```

Expected outputs are recomputed from `reference.py` on every run and never
stored. Rung inputs (everything except `program.ail/`) are fingerprinted into
`SCORE.lock`.

## Rules

1. The oracle is independent and written in Python. It is not derived from AIL.
2. Rungs are frozen once committed. Editing one requires `--bless-ladder`, a
   deliberate human act.
3. A new rung is committed in a failing state and demonstrated failing before
   the feature work that fixes it.
4. VM and native must agree. A report that says the right thing while the binary
   does something else fails.
5. Score is the only promotion currency. Emitting a document closes nothing.
6. The ratchet is mechanical: a rung in `SCORE.lock` marked `pass` may never
   regress, and a rung recorded in the lock may never disappear from `bench/`.

The runner enforces all six. Verified against four attacks: weakening an oracle
(exit 3), adding a softer case (exit 3), deleting a failing rung (exit 3), and
breaking the language while leaving the ladder untouched (exit 4).

## Tier 0 — computation

Without this tier nothing above it is meaningful.

| Rung | Program | Blocked on |
|---|---|---|
| R000-increment | `counter.value += 1` | — calibration, passes today |
| R001-add | `a + b` | expression IR, runtime operands |
| R002-arithmetic | `-`, `*`, `/`, `%` | arithmetic opcodes |
| R003-max | `max(a, b)` | comparison operators |
| R004-abs-sign | `abs`, `sign` | branching on a computed value |
| R005-factorial | `factorial(n)` | calls that return values |
| R006-fibonacci | `fib(n)` | multiple recursive calls per frame |
| R007-sum-to-n | `sum 1..n` | loops reachable from source |
| R008-gcd | `gcd(a, b)` | loop plus modulo |
| R009-collatz | Collatz step count | data-dependent loop bound |
| R010-is-prime | primality | loop with early exit |

## Tier 1 — data (partly implemented)

R011–R015 list build/append/length, sum/max, map, filter, fold · R016 sort ·
R017 map insert and lookup · R018 nested record update · R019 recursive ADT:
binary tree insert and in-order traversal · R020 strings: length, concat, split,
join, compare.

[R019-binary-tree](R019-binary-tree/intent.md) uses executable recursive `Tree<T>`
variants and concrete Int functions for insertion, duplicate multiplicities,
search, height and in-order traversal. Its independently frozen Python reference
and 45 cases measure VM/native results. See the
[tree application](../examples/binary-tree/README.md).

The remaining Tier 1 rungs still need independent frozen measurements. Executable
custom variants now provide recursive tails, while the historical descriptive
`List<T>` declaration remains unchanged. First-class functions and exhaustive
pattern matching remain open; legacy `Option.map` still uses its dedicated opcode.

## Tier 2 — real programs (partly implemented)

R021 word frequency top-5 · R022 CSV parse and aggregate · R023 JSON
pretty-printer · R024 LRU cache · R025 Conway's Life 20×20×50 · R026 expression
calculator: tokenizer, parser, evaluator · R027 run-length encode/decode ·
R028 binary search · R029 matrix multiply · R030 stack-machine interpreter.

**Passing this tier is the alpha-publication gate.** R026 is the decisive rung:
a language that can parse an expression language is a real language.

[R021-word-frequency](R021-word-frequency/intent.md) counts exact words separated
by the pinned 29 AIL whitespace scalars. Its imported pure library ranks the top
five by descending numeric count, then ascending Unicode text. All 75 independently
frozen request cases compare VM/native results. The
[stdin application](../examples/word-frequency/README.md) separately reads one
bounded raw text and emits exact compact JSON plus LF; independent acceptance
covers real stdin and every public execution route.

[R022-csv-aggregate](R022-csv-aggregate/intent.md) parses quoted CSV and aggregates
checked signed integer amounts by exact category in an imported pure AIL library.
Its 129 independently frozen cases compare VM/native results, including syntax,
request and data errors. The [CSV application](../examples/csv-aggregate/README.md)
also processes JSON-lines requests and writes reports to granted files after
successful validation. Useful-size checks do not imply unbounded processing;
ordinary value, arena, depth and fuel limits still apply.

[R023-json-pretty](R023-json-pretty/intent.md) produces deterministic two-space
JSON layout through an imported pure AIL library. All 162 independently frozen
cases compare VM/native results while retaining ordered duplicate pairs, exact
number spellings and decoded Unicode strings. Analyze uses complete UTF-8 Bytes
transport; the [stdin formatter](../examples/json-pretty/README.md) emits ordinary
pretty JSON plus LF. Independent acceptance covers all public execution routes,
actual stdin, fatal bounds and successful wrong-output mutations.

[R024-lru-cache](R024-lru-cache/intent.md) implements a reusable pure AIL cache
with immutable returned state, exact least-to-most-recent entry order and batch
events. All 78 independently frozen requests compare VM/native results, including
Unicode keys, cached null, duplicate-pair payloads and exact number spellings.
The [stdin cache tool](../examples/lru-cache/README.md) reads one request and emits
one compact JSON response. Acceptance covers all five APIs, public execution
routes, stdin, fatal bounds and a detected successful wrong-output mutation.

[R026-expression-calculator](R026-expression-calculator/intent.md) now implements
tokenization, shunting-yard parsing and checked evaluation in a reusable AIL
library, with 73 independently frozen cases on VM and native. The
[JSON-lines calculator](../examples/expression-calculator/README.md) also exposes
structured request errors and continuation.

[R027-run-length-codec](R027-run-length-codec/intent.md) implements binary RLE32
encoding, decoding and size preflight in an imported pure AIL library. Its 146
frozen cases check actual Bytes results and explicit domain errors on VM/native;
the [file tool](../examples/run-length-codec/README.md) processes granted binary
files. The other Tier 2 rungs remain planned; R021, R022, R023, R024, R026 and R027 do not satisfy
the complete alpha-publication gate.

## Tier 3 — effects and I/O (planned)

R031–R040 file read/write, argv/env/exit codes, clock, TCP echo server, raw HTTP
GET, HTTP `/health` responder, file-backed key-value store, two-task
deterministic concurrency.

Blocked on effect rows and effect polymorphism — without them there is no
generic `retry`, `map`, or middleware.

## Tier 4 — systems and IoT (planned)

R041–R050 bit manipulation, volatile MMIO register access, fixed-size no-heap
buffers, interrupt handler with critical section, ring buffer, cross-compile to
`thumbv7em-none-eabi`, `no_std` with a static memory budget, cycle timing, UART
driver, LED blink under QEMU.

## Tier 5 — SaaS (planned)

R051–R060 HTTP router, JSON request/response, bearer-token auth middleware,
record persistence and query, transaction with rollback, background job with
retry, rate limiter, structured logging with trace propagation, multi-tenant
isolation, single-binary deploy on `:8080`.

## Tier 6 — self-hosting (planned)

R061–R065 AIL tokenizer in AIL · AIL-Spec parser in AIL · one checker rule in
AIL · lowering in AIL · the AIL-authored compiler compiles R001–R010 with
byte-identical output to the Rust toolchain.
