# Expression Calculator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Track evidence here.

**Goal:** Implement a reusable expression interpreter entirely in AIL and execute
agent-friendly request streams through STE, VM and standalone native code.

**Architecture:** Pure lexer → postfix parser → checked stack evaluator; separate
request/stream adapters import the library. Independent Python recursive descent
supplies expected results and the new capability rung.

**Tech Stack:** Existing AIL Application packages, no new runtime dependency;
Python standard library for independent test tooling only.

**Spec:** [Expression calculator](../specs/2026-10-11-expression-calculator-design.md).

## Global constraints

- Preserve current workspace; .git read-only; no commits/worktrees.
- No host evaluator, eval opcode, external tool or compiler special case.
- i64 checked operations, truncating division; exact grammar/errors from spec.
- Expression at most 4096 Unicode scalars, at most 512 tokens.
- Structured result order: ok, value, error; failed value 0, successful error empty.
- Keep all old benchmark inputs/fingerprints unchanged; never --bless-ladder.
- Workers own disjoint files and never spawn subagents.

## Review focus

1. Parentheses/unary precedence and left associativity: nested unary and mixed
   division/remainder must agree with a separately structured oracle.
2. Malformed suffix versus early arithmetic error: complete syntax wins; lexing
   wins over syntax; request/length checks precede lexing.
3. Minimum i64 and non-integral/duplicate variables: exact error classes and
   intermediate overflow, including minimum remainder -1.
4. Reused library inputs/results and later bad stream records: no state leakage,
   correct errors and continuation, no accidental request data in stderr.
5. Exact limits and deep nesting: 512 tokens/4096 scalars accepted, next unit
   rejected; iterative parser must not exhaust call depth on legal nesting.

## Task 1: Independent oracle and failing capability rung

**Owner:** independent test worker.
**Create:** scripts/ail_calculator_differential.py;
bench/R026-expression-calculator/{intent.md,reference.py,cases/*.args}.
**Consumes:** spec only; do not derive expectations from AIL implementation.
**Produces:** independent expected results and executable bounded test runner.

- [x] Implement recursive-descent/AST Python reference with exact grammar, error
  priority and checked i64 semantics. Author fixed independent boundary cases.
- [x] Run `python3 scripts/ail_bench.py --rung R026-expression-calculator`; record
  unimplemented RED before program.ail exists; freeze input hashes in /tmp.
- [x] Add generated/boundary/all-route checks, streams, duplicate variables and
  deliberate evaluator mutation sensitivity. All subprocesses bounded.
- [x] Run the final oracle after Task 2; report counts, routes and exact hashes.

## Task 2: Pure AIL library and executable adapters

**Owner:** AIL implementation worker.
**Create:** examples/ail_std_expression.ail/{ail-package.md,spec.ail-spec.md};
examples/expression-calculator/{ail-package.md,spec.ail-spec.md,input.jsonl};
bench/R026-expression-calculator/program.ail/{ail-package.md,spec.ail-spec.md}.
**Consumes:** Task 1's independently frozen rung and spec public interfaces.
**Produces:** Calc.evaluate / Calc.evaluateRequest; Evaluate and Stream actions.

- [x] Wait for Task 1 RED before product code; read existing value-function,
  list, Boolean, JSON and recovery syntax, then implement lexer/postfix/evaluator.
- [x] Add importing Evaluate/Stream adapters and mixed sample, plus importing
  benchmark Evaluate action; helpers preserve exact structured error semantics.
- [x] Check all packages; compile/run fixed cases on VM/native. Exercise the
  five review-focus classes; report any compiler defects instead of workaround
  or silent contract reduction. Root will separately triage compiler changes.
- [x] Run the frozen rung and the independent full oracle; fix legitimate
  mismatches without modifying oracle inputs. Report file hashes and evidence.

## Task 3: Review, documentation and final verification

**Owner:** root; independent review by a worker who did not implement the library.
**Create:** examples/expression-calculator/README.md;
examples/ail_std_expression.ail/README.md;
examples/ste/expression-calculator.ail-ste;
docs/ail/55-expression-calculator.md.
**Modify:** README.md, docs/ail/README.md, examples/README.md,
docs/ail/25-example-inventory.md, examples/support-packages.md,
tests/ail_toolchain.rs (inventory counts), bench/LADDER.md, bench/SCORE.lock.

- [x] Independently review oracle logic and AIL implementation, run selected
  adversarial probes and resolve findings with reproducible evidence.
- [x] Document grammar, examples, errors, bounds, integer differences from Python,
  imports and exact commands; export STE from the actual checked implementation.
- [x] Verify frozen new rung inputs and all prior compiler/runtime/test/oracle
  hashes; run independent calculator oracle, all benchmark rungs and exact STE
  export/link checks. Run only additionally justified regression tests.
- [x] Record new passing rung via --update-lock, preserving all prior entries;
  final diff/format checks, artifact hashes and evidence. Goal remains active.

## Ledger

Previous goal turn: progress, verified directory milestone completed.
Baseline: /tmp/ail-calculator-baseline.json verifies all 112 inputs against final
directory evidence. No live jobs are being assumed or restarted.

Ruling: proceed with reversible design and implementation under the active user
continuation; no extra approval gate is needed. Preserve shared workspace and
use file hashes because .git is read-only. This changes no external service.

| Tasks | Shared contract / conflict scan |
| --- | --- |
| 1 ↔ 2 | Task 1 owns oracle and inputs; Task 2 only adds program.ail after RED. Public result/grammar shared via spec. |
| 1 ↔ 3 | Root reviews oracle, freezes exact inputs before score update; never weakens them. |
| 2 ↔ 3 | Root generates docs/STE after stable implementation; reviews use exact file hashes. |
| 1 | Independent recursive descent differs from AIL shunting yard; tests and spec agree. |
| 2 | Library and both callers share exact public interfaces; implementation may expose compiler bugs. |
| 3 | Unchanged implementation hashes avoid redundant broad tests; actual changes require scoped validation. |

Task states: 1 complete; 2 complete; 3 complete.

Inventory integration: adding the reusable `.ail` library changes the historical
top-level package count from 26 to 27. Existing inventory test failed for this
exact change, /tmp/ail-calculator-inventory-red.log. Update the counts and declare
the library support-only, retaining 128 replay entries. Run the inventory/learning
guide/closure checks and the toolchain target after integration. This is a test
metadata change, not a compiler implementation change.

Independent preflight complete: /tmp/ail-calculator-preflight.md, with 15 focused
case reports at /tmp/ail-calculator-preflight-probe.log. Existing imports, typed
JSON lookup and arithmetic semantics support the design. Compact helper loops
can hit fuel at 4096 iterations; implement long-zero normalization using existing
bounded search operations and explicitly test limits. Blank detection must use
a copy, preserving strict JSON parsing of original nonblank lines. Spec clarified
this intended behavior without changing the grammar or error contract.

Task 1 RED: /tmp/ail-calculator-rung-red.log, exit 1, unimplemented score 0/1.
Frozen 75 files including 73 cases: /tmp/ail-calculator-rung-frozen.json,
fingerprint sha256:1933508153ecb4338f90515c8b317322. Root independently checked
32 manually calculated expressions and 6 request shapes; all agreed with the
reference, and all frozen inputs were unchanged (/tmp/ail-calculator-reference-review.json).
Task 2 began only after this RED. Previous rung definitions remain frozen in
/tmp/ail-calculator-old-bench.json; no --bless-ladder is authorized.

Inventory GREEN: /tmp/ail-calculator-inventory-green.log, three tests including
package inventory, learning guides and support-only closure. Independent docs
pre-review approved after clarifying assignment/exponent punctuation error codes
and local call syntax: /tmp/ail-calculator-docs-pre-review.md. Final example/STE
verification follows the stable AIL implementation.

Task 2 released all seven inputs, /tmp/ail-calculator-task2-hashes.json and
/tmp/ail-calculator-task-2-report.md. Initial library smoke correctly failed on
an unsupported empty separator in TEXT_SPLIT; using existing indexed slices
fixed the lexer. This required no compiler change. The frozen rung subsequently
passed all 73 cases on VM/native, /tmp/ail-calculator-rung-second.log. Twenty
additional boundary executions and 64 streamed outcomes passed. Root independently
verified the documented 23 result and all 11 sample outputs on both backends;
the native sample also runs with empty environment/cwd and has no ELF dynamic
loader or interpreter segment.

Task 1 full oracle passed, /tmp/ail-calculator-oracle-first.log: seed0xca1c026,
12 routes,1,044 Evaluate checks,252 stream batches/4,212 result records,60 limit
checks and two deliberate mutation rejections,1,379 bounded commands. Frozen
fingerprint unchanged. Author report /tmp/ail-calculator-task-1-report.md and
hashes /tmp/ail-calculator-task-1-hashes.json. Root independent oracle review:
/tmp/ail-calculator-review-oracle.md, approved.

Final implementation review approved with no remaining findings:
/tmp/ail-calculator-review-implementation.md. Sixteen independent external-import
probe outcomes verify function-to-function use of both APIs, retained prior
results, aliasing and error precedence. Fresh STE export is byte-identical,
27,192 bytes, SHA256 27cfbeb2c89f5138c63548bede29148581bc1c8b7b2997d3527c34f5cbba20ae.
All seven reviewed product inputs and all 56 compiler/runtime/build baseline
inputs remain unchanged. Only four existing test inventory counts changed.

Fresh complete toolchain target:434 tests pass, no failures/ignored,
/tmp/ail-calculator-toolchain-tests.log. Target Clippy with -D warnings passes,
/tmp/ail-calculator-clippy.log. Eleven documentation files have88 valid local
links; Python syntax and exact STE export checks pass,
/tmp/ail-calculator-doc-checks.json. Full benchmark and final lock/hash checks
remain the final checkpoint.

Final full benchmark passed and recorded17/17 using --update-lock,
/tmp/ail-calculator-final-bench.log. All16 prior rungs still pass and preserve
their frozen inputs/fingerprints; the only added entry is R026 with its original
pre-implementation fingerprint. No --bless-ladder was used. Final relevant
artifact hashes and gate summary are in /tmp/ail-calculator-final-inputs.json and
/tmp/ail-calculator-final-gates.json. No review findings remain.

The expression-interpreter milestone is complete: reusable pure AIL library,
agent-friendly request stream, exact STE export, VM/native behavior and independent
correctness evidence. The full Specs → STE → Binary Python-replacement/AI-agent
goal remains active. Passing this rung does not complete all data/program tiers,
the alpha-publication gate or general Python compatibility.
