# AIL Capability Ladder And Autonomous Improvement Loop

Status: proposed
Supersedes as primary objective: `31-v03-learning-and-authoring-gate.md`
Date: 2026-07-28

## 0. Purpose

This document specifies a goal function and an autonomous loop that drive AIL
toward one stated outcome:

> AIL is a usable programming language, published to the world, able to express
> any kind of program from low-level IoT firmware to high-level SaaS platforms.

It replaces the current promotion mechanism, in which a capability gap is
closed by emitting a document that describes the gap. It defines instead a
single scalar score, computed by executing programs, that cannot be moved
without changing what the language can compute.

## 1. Measured Starting Position

All figures below were measured on commit `e1e75e7`, 2026-07-28, not inferred
from documentation.

### 1.1 What works

- `cargo test`: 432 passed, 0 failed, 21.7s.
- `python3 scripts/run_v03_release_audit.py`: 23 steps, all `ok`, exit 0,
  **105.7 seconds**.
- `ail-compile --target linux-x86_64-elf` emits a genuine static ELF64 that
  executes and returns correct exit codes.
- `ail-run examples/stateful_counter.ail --action IncrementCounter
  counter.value=5` correctly yields `counter.value=6`.

### 1.2 What does not work

**The language cannot add two numbers.**

```text
spec:      - the system increments the sum total by the sum a
bytecode:  WRITE_FIELD {key: "sum.total", text: "increments the sum total by the sum a"}
run:       sum.a=3  sum.b=4  sum.total=0        (unchanged)
check:     ail diagnostics: none
```

`ADD_INT_FIELD` is the only arithmetic opcode in the instruction set, and its
`delta` operand must be a compile-time literal. With a variable operand the
parser degrades to `WRITE_FIELD`, which stores the English sentence as a text
payload. No diagnostic is emitted.

**The parser silently discards what it does not recognize.** Six bullets in,
four dropped, `ail diagnostics: none`:

| Bullet | Result |
|---|---|
| `the system increments the counter value by 1` | `ADD_INT_FIELD delta=1` |
| `the system frobnicates the widget` | dropped, no diagnostic |
| `The system increments the counter value by 5` | dropped (capital `T`) |
| `the  system increments the counter value by 7` | dropped (double space) |
| `the system launches the missiles` | dropped, no diagnostic |
| `the system records a trace event named CounterIncremented` | `EMIT_TRACE` |

`parse_action_bullet` (`src/ail.rs:20076`) is a `strip_prefix` cascade with no
terminal `else` branch.

**AIL-Core stores English prose where an AST belongs.** Lowering
`examples/recursive_factorial.ail` produces:

```json
{"opcode":"FUNCTION_BRANCH","operands":{"condition":"n is 0"}}
{"opcode":"FUNCTION_RETURN","operands":{"value":"n multiplied by the recursive result"}}
```

`FUNCTION_BRANCH`, `FUNCTION_CALL`, and `FUNCTION_RETURN` push a trace line and
fall through (`src/ail.rs:11038-11056`). `factorial(5)` returns nothing.

**Control flow is unreachable from source.** `LABEL`, `JUMP`, and
`BRANCH_FIELD_EQUALS` exist in the VM and native backend but are never emitted
by any compiler path; they appear only in hand-authored `.ailbc.json`. The
x86-64 emitter explicitly rejects backward jumps (`src/ail.rs:6789`, `:6825`),
so every native AIL binary is loop-free by construction. `REPEAT_ACTION` is
unrolled at compile time.

**The trusted checker validates by substring match.** The component the entire
architecture rests on — "the LLM is an untrusted proposal mechanism, the
checker is the trusted boundary" — decides termination like this
(`src/ail.rs:16140`):

```rust
fn recursive_call_looks_decreasing(call_text: &str) -> bool {
    let call_text = call_text.to_ascii_lowercase();
    call_text.contains(" minus ") || call_text.contains(" - ")
        || call_text.contains("decrement") || call_text.contains("predecessor")
        || call_text.contains("smaller") || call_text.contains("less")
}
```

`AIL-STATE-001` similarly searches for the word `retry` in English prose.

**No recursive data structures.** `List<T>` is declared as
`Empty | Items(values: T)` — no tail. Trees, ASTs, and linked structures are
not expressible, which independently rules out self-hosting. `Option.map` is a
dedicated VM opcode (`OPTION_MAP`, `src/ail.rs:11148`) that reads a
pre-computed answer from `mapper.result`, because the language cannot express
`map`.

### 1.3 What the evidence system measures

A mutation test settles this. Flipping `ADD_INT_FIELD` from `current + delta`
to `current - delta` (`src/ail.rs:11138`) — corrupting the only arithmetic
operation in the language — produces:

```text
test result: FAILED. 429 passed; 3 failed
```

Three of 432 tests caught it. **The artifact, fingerprint, manifest, and audit
layer caught zero.** It would have regenerated the corrupted traces, hashed
them, compared them against hashes computed from the same bytes in the same
run, and passed.

`verify_artifact_dir` (`scripts/run_v02_release_audit.py:277`) computes
`fnv64(manifest_bytes)` and compares it to `manifest.fingerprint.txt`, written
by the same run from the same bytes. This is a tamper seal, not a correctness
check.

Corpus arithmetic:

| Claim | Measured |
|---|---|
| 128 replayable end-to-end examples | 16 distinct programs; `diff example-40 example-45` = 2 lines, differing by a serial number |
| prompt portability across models | 124/128 entries are `codex-skill-agent`; 4 touched a real endpoint, all one model |
| 17 v0.3 learning signals | 17 promoted, 0 deferred; 11 closed by citing `ail-examples --release-evidence`, i.e. by emitting a `.txt` describing the same entries |
| largest program in corpus | 195 spec lines; median 27 lines, median 1 action |

Last 40 commits: 81% of added lines outside `src/`. Of 39 functions added to
`src/`, 12 are `render_ail_e2e_*_text` report renderers and ~5 are new
checkers.

### 1.4 Honest summary

AIL is a well-engineered **contract and specification language for
agent-authored systems**, with a real package system, a real rejection engine
(52 diagnostics with repair suggestions), real provenance, and a real ELF
writer. It is not yet a programming language: it has no expression
evaluator, no value model beyond `BTreeMap<String, String>`, no calls that
return values, no loops reachable from source, and no recursive data.

The contract half is genuinely built. The computation half is absent.

## 2. Why The Current Loop Cannot Reach The Goal

The existing loop is:

```text
ail-examples --release-evidence
  -> emits v03-roadmap.txt signals (gaps)
  -> a renderer is written that emits <gap-name>-review.txt about the same entries
  -> the signal is marked promoted in v03-roadmap-signal-status.md
  -> the release audit passes
```

This loop is closed and self-satisfying. Its terminating condition is "every
signal is promoted", and a signal can be promoted by writing prose. **Nothing
in the loop can generate a demand for a language feature the language does not
have.** The observed result is 17/17 signals promoted while the language cannot
add two numbers.

The failure is not diligence — the execution has been disciplined and the
artifacts are internally consistent. The failure is that the objective function
is satisfiable without touching the object being improved.

## 3. The New Goal Function: The AIL Capability Ladder

### 3.1 Definition

`bench/` contains numbered **rungs**. Each rung is a program AIL must be able
to express, compile, and execute correctly.

```text
bench/
  LADDER.md              # ordered rung index, frozen
  SCORE.lock             # highest score ever achieved + passing rung set
  R001-add/
    intent.md            # English statement of required behavior
    reference.py         # independent oracle, not written in AIL
    cases/
      case-01.args       # command-line arguments
      case-01.stdin      # optional
    program.ail/         # the AIL implementation (absent until it can exist)
```

Expected outputs are **not stored**. They are regenerated from `reference.py`
on every run. This makes it impossible to move the score by editing expectations.

### 3.2 Scoring procedure

`ail-bench` executes, for each rung:

1. `program.ail/` exists, else rung fails.
2. `ail-check` reports zero diagnostics.
3. Compile to bytecode; run each case through the VM.
4. Compile to native ELF for the rung's declared target; run each case.
5. Compute `reference.py` output for each case.
6. **Pass requires: VM output == native output == reference output**, for every
   case, byte for byte.

Score = passing rungs / total rungs. Both a VM/native differential and an
independent oracle must agree. No artifact, report, or fingerprint participates.

### 3.3 Current score

**0 / 65.** Rung R001 is "read two integers from argv, print their sum." As
measured in §1.2, AIL cannot express it.

### 3.4 The ladder

Tier 0 — computation. *Without this tier nothing else is meaningful.*

| Rung | Program | Blocked on |
|---|---|---|
| R001 | `a + b` from argv | expression IR, runtime operands |
| R002 | `a - b`, `a * b`, `a / b`, `a % b` | arithmetic opcodes |
| R003 | `max(a, b)` | comparison operators |
| R004 | `abs`, `sign` | branching on computed values |
| R005 | `factorial(n)` | calls that return values |
| R006 | `fib(n)` | multiple recursive calls, frames |
| R007 | `sum 1..n` | loops from source |
| R008 | `gcd(a, b)` | loop + modulo |
| R009 | `collatz_steps(n)` | data-dependent loop bound |
| R010 | `is_prime(n)` | loop with early exit |

Tier 1 — data.

| Rung | Program |
|---|---|
| R011–R015 | list build/append/length; sum/max; map; filter; fold |
| R016 | sort a list of integers |
| R017 | map/dictionary insert and lookup |
| R018 | nested record update |
| R019 | recursive ADT: binary tree insert + in-order traversal |
| R020 | strings: length, concat, split, join, compare |

Tier 2 — real programs. *Passing this tier is the alpha-publication gate.*

| Rung | Program |
|---|---|
| R021 | word frequency, top 5, from stdin |
| R022 | CSV parse and aggregate |
| R023 | JSON pretty-printer |
| R024 | LRU cache with eviction |
| R025 | Conway's Life, 20×20, 50 generations |
| R026 | expression calculator: tokenizer, parser, evaluator |
| R027 | run-length encode and decode |
| R028 | binary search over a sorted array |
| R029 | matrix multiply |
| R030 | a tiny stack-machine interpreter |

Tier 3 — effects and I/O. *Where the capability model finally gets teeth.*
R031–R040: file read/write, argv/env/exit codes, clock, TCP echo server, raw
HTTP GET, HTTP `/health` responder, file-backed key-value store, two-task
deterministic concurrency.

Tier 4 — systems and IoT. R041–R050: bit manipulation, volatile MMIO register
access, fixed-size no-heap buffers, interrupt handler with critical section,
ring buffer, cross-compile to `thumbv7em-none-eabi`, `no_std` with a static
memory budget, cycle-accurate timing, UART driver, LED blink verified under
QEMU.

Tier 5 — SaaS. R051–R060: HTTP router, JSON request/response, bearer-token auth
middleware, record persistence and query, transaction with rollback, background
job with retry, rate limiter, structured logging with trace propagation,
multi-tenant isolation, single-binary deploy serving on `:8080`.

Tier 6 — self-hosting. R061–R065: AIL tokenizer in AIL; AIL-Spec parser in AIL;
one checker rule in AIL; lowering in AIL; the AIL-authored compiler compiles
R001–R010 with byte-identical output to the Rust toolchain.

## 4. Anti-Gaming Rules

These are the operative part of this document. The previous objective failed
because it was satisfiable by generation; these rules make that impossible.

1. **The oracle is independent.** `reference.py` is Python. Expected outputs
   are regenerated from it on every run and never stored. An agent cannot edit
   an expectation to match a broken program.
2. **Rungs are frozen.** `bench/LADDER.md` and every `intent.md` and `cases/`
   directory are fingerprinted into `bench/SCORE.lock`. The loop may add rungs;
   it may not modify a committed rung. Modification requires human approval and
   is enforced by the harness, which aborts on fingerprint mismatch.
3. **A rung must fail before it counts.** New rungs are committed in a failing
   state and demonstrated failing. This prevents writing rungs to match what
   already works.
4. **VM and native must agree.** Every rung runs both ways. A change that makes
   a report say the right thing while the binary does something else fails.
5. **Score is the only promotion currency.** A capability gap is closed when a
   rung flips from fail to pass. Emitting a `*-review.txt` closes nothing. The
   `promoted`/`deferred` mechanism in `v03-roadmap-signal-status.md` is retired
   as a completion criterion.
6. **The ratchet is mechanical.** `bench/SCORE.lock` records the highest score
   ever achieved and the exact set of passing rungs. The loop may not commit a
   change that drops any rung out of that set.
7. **Sensitivity is tested.** A weekly mutation run applies 50 mechanical
   mutations to semantic code in `src/`. Required: ≥40 killed by `cargo test`,
   and ≥10 killed by `ail-bench` with `cargo test` removed from the plan. Today
   the second figure is 0.

## 5. Loop Mechanics

```text
loop:
  1. HEALTH        run_v03_release_audit.py           (105s, must be green)
  2. SCORE         ail-bench --all                    (record score, compare to SCORE.lock)
  3. SELECT        lowest-numbered failing rung
  4. DIAGNOSE      why does it fail? name the missing language capability
  5. DESIGN        smallest change unblocking it (often unblocks several rungs)
  6. IMPLEMENT     in a git worktree, never in the main tree
  7. VERIFY        a. target rung passes: VM == native == oracle
                   b. every rung in SCORE.lock still passes   (ratchet)
                   c. full audit still green
                   d. cargo fmt / clippy -D warnings clean
  8. DECIDE        all green -> commit, update SCORE.lock
                   any red   -> discard the worktree, record the attempt
  9. RECORD        append to bench/JOURNAL.md: rung, capability added, score delta
```

Step 8 is deterministic code, not a model judgment. The loop cannot talk itself
into a commit.

Cycle-time budget, measured: audit 105s + bench (projected <60s at 65 rungs) —
a full verification pass under three minutes. This is fast enough to verify
every candidate change with no sampling.

### 5.1 The local model: measured capability

`ornith-1.0-35b` was benchmarked live at `http://192.168.60.171:8091` on
2026-07-28. These are measurements, not estimates.

| Metric | Value |
|---|---|
| Generation | **34.1 tok/s**, stable across 16+ calls |
| Prefill | 378 tok/s aggregate; 497 tok/s at 1266-token prompts |
| Trivial round-trip | 0.80 s |
| Determinism at temp 0 | **bit-identical across 3/3 runs** |
| Serial daily budget | ~2.9M generated tokens/day |

A checker-clean 92-line AIL-Spec costs ~700 tokens ≈ 21 s, so roughly 4,000
spec-sized artifacts per day on a single stream. That is a real corpus engine.

**What it does well, measured:**

- Envelope protocol compliance: 11/11 correct `artifact_kind` and correct
  artifact-vs-questions outcome, unprompted.
- **Grammar transfer from an exemplar.** Given a grammar sheet plus one full
  reference spec in context, it produced a checker-clean spec for a novel
  domain (library lending), lowering to 211 nodes+edges — structurally
  identical to the support-ticket reference.
- Interview and blocking-question generation: correctly asked rather than
  invented on all 3 question-expected prompts.

**What it cannot do, measured:**

- **Repair from a diagnostic: 0/1.** Given a real `AIL003` and the real
  rejected fixture, it invented rollback and notification behavior — the exact
  `hallucinated_field` failure the prompt forbids — and the checker still
  returned `AIL003`. Zero progress.
- **Edit-preserving transformation: 0/3.** It regenerates from scratch rather
  than editing. See §5.2.
- **Grammar without an exemplar: hard reject.** The repo's own harness probe
  produces a spec the checker rejects outright.
- **No resampling path.** `src/llm.rs` hardcodes `temperature: 0.0` with no
  `seed`. Combined with measured bit-determinism, **a failed generation fails
  identically forever.** Retry is a no-op.

Verdict: a competent pattern-completion engine and an incompetent
reasoning-under-constraint engine. **Suitable as a throughput component inside
a loop whose correctness comes from elsewhere; not suitable as the loop's
driver**, because it cannot close a propose→check→repair cycle.

### 5.2 The blocking safety defect

Running the production `ail-draft` path live against 8091 on the real 92-line
`examples/support_ticket.ail`, 2 of 3 runs returned a **26-line** spec that
dropped the entire user model, 6 of 9 `Ticket` fields including
`internal notes: Secret<List<Text>>`, 4 of 5 actions, both `Failure` sections,
and every guarantee — replacing them with an invented stub. The verdict:

```text
ail-draft diagnostics: none
```

**A secret-bearing, permission-guarded application was silently reduced to a
toy, and the trusted boundary reported clean.** The third run failed with
`AIL-PROMPT-001 prompt envelope cannot contain both artifact_text and
questions` — a shape the prompt pack's own envelope template invites.

The root cause is that `ail-check` validates an artifact **in isolation**. It
never compares a candidate to the artifact it replaces. So "diagnostics: none"
is not a safety property under edit. An unattended loop using this path will
produce a large corpus of confidently-wrong, checker-blessed artifacts.

A related false green: `scripts/run_v03_prompt_llm_harness.py` reported
`prompt-envelope-valid-count 11`, `review-result accepted` for all 11 prompts —
while the spec-draft artifact inside that accepted run is the one the checker
hard-rejects. **The harness never calls `ail-check`.** It certifies envelope
shape as if it were artifact validity.

### 5.3 Division of labor

| Loop role | Owner |
|---|---|
| Language design: IR, type system, effects | human + frontier |
| Compiler/checker implementation in Rust | frontier |
| `llm.rs` hardening, harness gating fix | frontier |
| Semantic-equivalence gate, package-preservation guard | deterministic code (**missing today**) |
| Verification, scoring, commit/revert | deterministic code only |
| Grammar sheet + exemplar authoring, per prompt (once) | frontier |
| Bulk corpus authoring from requirements + exemplar | **Ornith** |
| Bench rung authoring, batched, execution-gated | **Ornith** |
| Paraphrase/variant generation for robustness corpora | **Ornith** |
| Rejected-fixture generation (mutate a valid spec to trip a code) | **Ornith** |
| Interview / blocking-question generation | **Ornith** |
| `core-to-summary` friendly renderings | **Ornith** |
| Diagnostic-driven repair | frontier — Ornith measured 0/1 |
| Any edit preserving an existing artifact | frontier — Ornith measured 0/3 |
| Promotion into `./examples` | human, until the equivalence gate exists |

On the user-facing Agent specifically: Ornith is a viable interviewer and
**greenfield** spec drafter, provided every prompt ships a grammar sheet and a
full exemplar. It is **not** viable for amending an existing spec until §5.2 is
fixed, because that is precisely the operation it was measured destroying.

### 5.4 Endpoint correction

`http://inteligentia-pro-1:8080/v1/chat/completions` is **down** (HTTP 000, no
listener) and is hardcoded in **33 files** — including `src/main.rs:1516` as
the default, all four live harness scripts, `README.md`, and the
`base_llm_endpoint` baked into package metadata, which `ail-check` prints on
every invocation. Every documented live command in the repo is currently
broken. The live server is `http://192.168.60.171:8091`.

## 6. Phases And Exit Criteria

### Phase 0 — Make the loop trustworthy (days)

Nothing else may start until the measurement apparatus is honest.

Status as of 2026-07-28 — done: the ladder and `ail-bench` with the ratchet and
tamper checks; `AIL-PARSE-001`; `AIL-EXPR-001`; VM/native differential testing;
`llm.rs` timeouts, retries, resampling, and response parsing; the prompt-harness
checker gate; both new gates wired into the release audit (25 steps, 127s);
`LICENSE`; version; the endpoint default and `AIL_LLM_ENDPOINT`; the seed-corpus
guard. Outstanding: the semantic-equivalence gate and package-preservation guard
(§5.2), the mutation-sensitivity check (§4.7), and grammar sheets plus exemplars
for each prompt. Until the equivalence gate lands, no model may write into
`examples/`.

**Correctness of measurement**

- Add a terminal `else` to `parse_action_bullet`; emit `AIL-PARSE-001` for any
  bullet no rule consumed. **This is the single highest-value fix in the repo.**
- Reject, rather than silently degrade, an arithmetic bullet whose operand is
  not a literal (`AIL-EXPR-001`), until Phase 1 lands.
- Build `bench/`, `ail-bench`, `SCORE.lock`, `JOURNAL.md`.
- Add VM↔native differential testing across the existing corpus.
- Add the mutation-sensitivity check (§4.7).

**Safety of unattended operation** — from §5.2; without these the loop
actively destroys work.

- **Semantic-equivalence gate.** `ail-check` gains a mode asserting that a
  candidate lowers to the same AIL-Core as the artifact it replaces, modulo
  declared intent. Until this exists, "diagnostics: none" is not a safety
  property under edit, and no model may write into `examples/`.
- **Package-preservation guard** for `ail-draft` / `ail-patch`: reject a
  candidate that drops declared things, fields, actions, failures, guarantees,
  or secrets without an explicit intent to remove them.
- **`llm.rs` hardening**: `connect_timeout` and `set_read_timeout` (there are
  none today — a hung server hangs the CLI forever), bounded retries with
  backoff, a `seed`/`temperature` knob so retry is not a no-op, and a real JSON
  parser. Fix the `\uXXXX` decoding bug at `src/llm.rs:534`, where
  `other => other` turns `A` into the literal `u0041`.
- **Guard against empty-content success**: if `content` is empty and
  `reasoning_content` is populated, fail loudly rather than returning `""` as a
  valid artifact.
- **Harness must call the checker.** `run_v03_prompt_llm_harness.py` must pipe
  every artifact-producing probe through `ail-check` before writing
  `review-result accepted`. Best effort-to-value ratio in the repo.
- **Ship a grammar sheet and one full exemplar with every prompt.** Measured to
  move spec-draft from hard-reject to `diagnostics: none`.

**Hygiene**

- Replace the dead `:8080` endpoint across all 33 files with configuration.
- Add `LICENSE`; reconcile `Cargo.toml` `0.1.0` against the v0.3 docs.
- Guard `generate_e2e_seed_corpus.py`, which defaults its `--output-dir` to the
  live `examples/` tree.
- Re-label corpus provenance: 119/128 stored captures are
  `codex-subagent-default`, i.e. frontier-authored. The repo currently reads as
  though a local model produced them.

Exit: `ail-bench` reports an honest score (expected 0/65), the parser can no
longer discard input silently, and no unattended path can overwrite a package
with a stub.

### Phase 1 — The computational core (weeks)

The largest single piece of work in the project, and unavoidable.

- A typed expression IR: constants, variables, operators, calls, with a
  canonical English projection. Replaces prose in `condition=` / `value=`.
- An expression parser producing that IR, plus a terse non-English surface for
  cases where controlled English is a poor fit.
- A value model beyond `BTreeMap<String, String>`: typed values, locals, an
  operand stack, call frames with parameters and return values.
- Bytecode: real arithmetic and comparison opcodes; `CALL` with arguments and
  return values.
- Native backend: a writable segment, a stack frame, virtual registers with a
  linear-scan allocator, and backward jumps — lifting the ban at
  `src/ail.rs:6789`.
- Replace `recursive_call_looks_decreasing` and `branch_looks_like_base_case`
  with structural analysis over the new IR.

Exit: R001–R010 pass on both VM and native. **AIL can compute.**

### Phase 2 — Data (weeks)

Recursive ADTs with a real tail; exhaustive pattern matching; first-class
function types; collection and string operations as AIL programs rather than
VM opcodes; delete the `OPTION_MAP` special case.

Exit: R011–R020 pass.

### Phase 3 — Real programs (weeks)

No new language features expected; this tier proves generality. R026 (a
tokenizer, parser, and evaluator written in AIL) is the decisive rung — a
language that can parse an expression language is a real language.

Exit: R021–R030 pass. **This is the alpha-publication gate.**

### Phase 4 — Effects and I/O

Effect rows and effect polymorphism in the Koka sense — without them there is
no generic `retry`, `map`, or middleware, which blocks both stated target
domains. Real syscall effects behind the existing capability grants.

Exit: R031–R040 pass.

### Phase 5 — IoT and SaaS

New backends (`thumbv7em-none-eabi`, `aarch64`), `no_std`, volatile/MMIO,
interrupts with a real ISR ABI; then the SaaS tier on top of Phase 4 effects.
Note that `deadline 10 ms, budget 2 ms` is currently stored text — discharging
it requires a bounded-time cost model that does not yet exist.

Exit: R041–R060 pass.

### Phase 6 — Self-hosting

Exit: R061–R065 pass; the AIL-authored stage replaces its Rust counterpart in
CI with byte-identical output.

## 7. Publication Gate

AIL may be published to the world when all of the following hold. Each is
falsifiable and none is satisfiable by generating a document.

1. `ail-bench` score ≥ 30/65, with all of Tiers 0–2 passing.
2. **Cold-start authoring.** Three programmers who have never seen AIL, given
   only the published documentation and a fresh checkout, each independently
   write and run a word-frequency program. Pass: ≥2 of 3 succeed within 90
   minutes with no help from the author. Every question they must ask is
   recorded as a documentation defect.
3. **Unseen-domain program.** A program in none of the corpus templates
   (LRU cache, Life, JSON printer) is written and passes its cases without
   adding a prompt file or a catalog entry.
4. **Adversarial soundness.** Someone motivated to break the checker writes 20
   specs that are semantically wrong but pass every check. Pass: ≤3 of 20 slip
   through. This directly attacks the substring-matching checkers, which is
   where the real risk lives.
5. **Model portability, actually measured.** ≥30 rungs authored by three
   different models using only the published prompt pack, no repair loop.
   Pass: ≥80% check clean on first attempt.
6. Mutation-kill rate meets §4.7.
7. The minimum public-release checklist: `LICENSE`, consistent version, CI
   running `fmt`/`clippy`/`test`/`bench`, a tagged release, readable per-command
   `--help`, a verified quickstart, `CONTRIBUTING.md`, no internal hostnames in
   public docs.
8. A naming decision, made deliberately. **AILANG** (ailang.sunholo.com)
   already exists and positions itself as "an AI-First Programming Language —
   deterministic, effect-typed, a safe target for AI-generated programs." That
   is nearly identical positioning and a phonetically adjacent name.
   **IntentLang** is a second near-collision.

## 8. What To Keep

This work is not a rewrite. The following are genuinely good, ahead of common
practice in places, and must survive:

- **Provenance on every node and edge**, distinguishing human-confirmed from
  agent-inferred (`03-semantic-ir.md:112`). The single best idea in the
  project; no mainstream language has it, and it is the correct trust primitive
  for agent-authored code.
- **IR-as-source-of-truth with projections** — "if a projection and AIL-Core
  disagree, AIL-Core wins and the projection is a compiler bug"
  (`01-language-architecture.md:53`).
- **Capability grants that survive into the bytecode artifact**
  (`03-semantic-ir.md:176`), and the non-ambient authority model for imports.
- **Graph patches gated on `base_hash`** (`18-ail-core-schema.md:228`) —
  optimistic concurrency for program edits.
- **`Secret<T>` and the safety-class ladder**, with paired accept/reject
  fixtures required per unsafe feature — better discipline than Rust has.
- **52 diagnostics with `repair_suggestion` and source provenance.** Genuinely
  actionable error messages, better than most young languages.
- **The hand-written ELF64 writer and `X64Code` assembler** with label patching
  and argv-parsing runtime routines (`src/ail.rs:6652`, `:7973`, `:8116`).
  Real machine-level engineering; the foundation Phase 1 builds on.
- **The 105-second, 23-step release audit.** Fast, deterministic,
  reproducible from a clean checkout. Keep it, and add `ail-bench` to it as
  step 24.
- **The package system**: version ranges, registry index resolution, alias
  conflict rejection, capability grants checked before lowering.

The evidence apparatus should be **demoted, not deleted**. Reproducibility,
provenance, and tamper-evidence are real engineering goods. They stop being
useful the moment they are read as proof of capability.

## 9. Risk Note

At the observed rate — roughly 5 of the last 40 commits added checker or
language behavior — Tiers 0–2 are a multi-quarter effort even with the loop
running well. The estimate is dominated by Phase 1, which touches the parser,
the IR, the VM, and the native backend simultaneously, i.e. the three things
that make a compiler plus the one that makes it fast.

The honest framing for planning: this is not "finish v0.3 and publish." It is
"the specification and contract layer is built; now build the language
underneath it." The existing work is a runway, and it is a good runway. The
plane has not been built yet.
