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

## Tier 1 — data (planned)

R011–R015 list build/append/length, sum/max, map, filter, fold · R016 sort ·
R017 map insert and lookup · R018 nested record update · R019 recursive ADT:
binary tree insert and in-order traversal · R020 strings: length, concat, split,
join, compare.

Blocked on recursive ADTs with a real tail (`List<T>` is currently
`Empty | Items(values: T)` — no tail), first-class function types, and pattern
matching. `Option.map` is presently a dedicated VM opcode because the language
cannot express `map`.

## Tier 2 — real programs (planned)

R021 word frequency top-5 · R022 CSV parse and aggregate · R023 JSON
pretty-printer · R024 LRU cache · R025 Conway's Life 20×20×50 · R026 expression
calculator: tokenizer, parser, evaluator · R027 run-length encode/decode ·
R028 binary search · R029 matrix multiply · R030 stack-machine interpreter.

**Passing this tier is the alpha-publication gate.** R026 is the decisive rung:
a language that can parse an expression language is a real language.

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
