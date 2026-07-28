# The AIL Bootstrap Chain

This directory holds the self-hosting bootstrap chain for AIL: the sequence of
compiler generations that ends with AIL compiling itself. It is scaffolding,
built ahead of the capability that fills it, so self-hosting is *measured*
from day one instead of retrofitted once someone remembers to check.

Run the chain with:

```bash
python3 scripts/ail_bootstrap_chain.py --status   # report how far it gets today
python3 scripts/ail_bootstrap_chain.py --run       # attempt the full chain
```

See also `docs/ail/13-bootstrap-self-hosting.md` (the stage-0..5 bootstrap
model this chain implements a slice of) and
`docs/ail/32-capability-ladder-and-improvement-loop.md` (the capability ladder
and rungs referenced below).

## The stage model

This is the same shape Rust bootstraps in:

```text
stage0     target/debug/ail                    Rust-written, exists today
stage1     stage0 compiles compiler.ail   ->    ail-stage1
stage2     stage1 compiles compiler.ail   ->    ail-stage2
FIXPOINT   stage1 output == stage2 output, byte for byte
```

- **stage0** is the bootstrap compiler: today's Rust-implemented `ail` binary
  (`target/debug/ail`). It is the one and only stage allowed to be
  non-AIL-authored — the bootstrap allowance in
  `docs/ail/13-bootstrap-self-hosting.md` ("bootstrap implementation languages
  may start AIL but must not own AIL").
- **stage1** is what you get when stage0 compiles `compiler.ail` — an AIL
  program that is itself a compiler for AIL. `compiler.ail` does not exist
  yet; see "What must land before stage1" below.
- **stage2** is what you get when the stage1 *binary* — no longer stage0,
  no Rust involved — compiles the same `compiler.ail` source again.
- **fixpoint**: stage1 and stage2 must produce byte-identical output. This is
  the actual definition of self-hosting, not a nice-to-have: it is the same
  check `rustc` runs (`x.py build --stage 2` compares the stage1-built and
  stage2-built compilers), and it's what distinguishes "a compiler happens to
  exist" from "the language can reproduce its own toolchain."

Why the fixpoint, specifically, is the real test: any single stage can be an
accident — a lucky one-off translation, hand-patched until it produces
correct-looking output for the cases someone tried. A fixpoint cannot be an
accident. If stage1 (built by Rust) and stage2 (built by stage1, with zero
Rust in the loop) agree byte for byte, then stage1 is a faithful, total
implementation of whatever `compiler.ail` specifies — good enough that
applying it to its own definition a second time changes nothing. That is the
property that lets stage0 be deleted from the trust boundary.

## What must land before stage1 can even be attempted

`scripts/ail_bootstrap_chain.py --status` reports this list live; it is
reproduced here for readers who want the argument in one place. Every item is
measured, not projected — see
`docs/ail/32-capability-ladder-and-improvement-loop.md` §1.2 for the
evidence, and `bench/` for the executable rungs that will flip from `fail` to
`pass` as each capability lands.

AIL today cannot express a compiler. Concretely, before `compiler.ail` (a
lexer, a parser, an AST, a checker, and a code generator, all written in AIL)
can exist:

1. **A typed expression IR with runtime (non-literal) operands, plus general
   arithmetic/comparison opcodes.** The language cannot add two runtime
   values today: `ADD_INT_FIELD` is the only arithmetic opcode and its
   `delta` operand must be a compile-time literal. A variable operand
   silently degrades to `WRITE_FIELD`, storing English prose as a text
   payload — no diagnostic. (`bench/R001-add`, `R002-arithmetic`,
   `R003-max`, `R004-abs-sign`)
2. **Function calls that return values, with real call frames.** A compiler
   is built from functions that consume input and produce a result
   (tokenize, parse, check, lower). Today `FUNCTION_CALL` and
   `FUNCTION_RETURN` push a trace line and fall through; `factorial(5)`
   returns nothing. (`bench/R005-factorial`, `R006-fibonacci`)
3. **Loops reachable from source.** Scanning source text and walking a token
   or AST list needs unbounded iteration. `LABEL`/`JUMP`/`BRANCH_FIELD_EQUALS`
   exist in the VM and native backend but no compiler path ever emits them;
   the x86-64 emitter explicitly rejects backward jumps, and `REPEAT_ACTION`
   is unrolled at compile time — every native AIL binary is loop-free by
   construction. (`bench/R007-sum-to-n`, `R008-gcd`, `R009-collatz`,
   `R010-is-prime`)
4. **Recursive algebraic data types with a real tail.** A token stream and an
   AST are both exactly this shape (`Cons(head, tail)` / `Node(left, right)`).
   `List<T>` today is declared `Empty | Items(values: T)` — no tail — which
   independently rules out self-hosting. (Tier 1, rung R019 in the capability
   ladder; not yet materialized under `bench/`.)
5. **String processing:** indexing, slicing, comparison, concatenation,
   character classification. A lexer *is* a string-processing loop. No
   string opcode exists today beyond storing prose as an opaque payload.
   (Tier 1, rung R020 in the capability ladder; not yet materialized under
   `bench/`.)
6. **File I/O from AIL source itself:** read `compiler.ail`'s own source
   text; write the bytecode/ELF bytes of the next stage. No file-read/write
   effect exists in the language yet. (Tier 3 of the capability ladder,
   rungs R031 onward.)
7. **An AIL-expressible code-generation target.** Lowering to bytecode and
   emitting native machine code must themselves be expressible as AIL
   computation over AIL data — not merely invoked by AIL on Rust's behalf.
   This is the Stage 3 requirement in `docs/ail/13-bootstrap-self-hosting.md`:
   the reviewable output is deterministic AIL-authored bytecode/ELF bytes
   plus a host-boundary report proving no Rust/C/other host-language backend
   source was generated. This is the capstone item — everything above is a
   precondition for writing `compiler.ail` at all; this one is the
   precondition for `compiler.ail`'s output to count as self-hosted rather
   than cross-compiled by a hidden Rust step.

Items 1–3 are Phase 1 of the capability ladder ("the computational core"),
items 4–5 are Phase 2 ("data"), item 6 is part of Phase 4 ("effects and
I/O"), and item 7 is the Stage 3 boundary condition from the bootstrap doc.
That is also the order in which the improvement loop described in
`docs/ail/32-capability-ladder-and-improvement-loop.md` §5 is planned to
close them, which is why the list above is ordered that way rather than by
which capability *feels* most fundamental to a compiler-shaped program.

## Layout

```text
bootstrap/
  README.md          this file
  compiler.ail/       (does not exist yet) the AIL source of a compiler for AIL,
                       once the language can express one; a package directory in
                       the same shape as examples/*.ail or bench/*/program.ail
  build/              (created on demand by --run) stage1/stage2 output binaries
```

Nothing in `compiler.ail/` is fabricated ahead of the language being able to
express it — see `scripts/ail_bootstrap_chain.py` for why: fabricating a
`compiler.ail` that merely runs but doesn't compile would satisfy this
scaffolding's checks by doing something trivial, which is exactly the failure
mode `docs/ail/32-capability-ladder-and-improvement-loop.md` §4 (anti-gaming
rules) is written against.
