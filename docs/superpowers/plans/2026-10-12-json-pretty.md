# Pure AIL JSON pretty-printer implementation plan

> For agentic workers: use superpowers:subagent-driven-development for sequential
> implementation, independent task reviews and a fresh whole-milestone review.
> Steps use checkboxes; standing autonomous execution authority remains in force.

Goal: preserve JSON's ordered pairs, duplicate keys, exact number tokens and decoded
strings while producing deterministic two-space layout in a pure AIL library/tool.
Architecture: existing Json validation/compact formatting plus ordinary AIL layout
operations; only the application owns stdin/stdout. No new host pretty operation.
Stack: existing dependency-free Rust compiler, AIL source and Python stdlib oracle.
Spec: [JSON pretty-printer design](../specs/2026-10-12-json-pretty-design.md).

## Global constraints

- No git mutations, worktrees, dependencies, publication or unrelated refactors.
- Preserve completed word-frequency milestone and all prior rung inputs/programs,
  compiler/runtime, tests and old scripts. Reproduced compiler defects need separate
  scope/review; do not silently change the runtime to get this library passing.
- Preserve ordered object pairs, duplicate names, exact number tokens and decoded
  strings. Exactly two-space indentation, LF layout, compact empty containers,
  colon plus space, no trailing spaces/comma/LF in library results.
- Freeze the independently reviewed new oracle/cases before AIL implementation;
  actual missing-program RED, exact fingerprints and no --bless-ladder.
- All functions/helpers pure; no host pretty-printer, new opcode, I/O, external/
  process call, arbitrary small semantic cap, alternate fuel or budget padding.
- Actual outputs versus independent expectations decide passing. No Python/NLP/
  alpha-completion or arbitrary-size/linear-time/reclamation claim.
- Controller owns baseline, integration, scoring/JOURNAL/LADDER and checkpoints.
  Use /tmp/ail-pretty-* artifacts/briefs/reports and preserve evidence.

## Review focus

1. Duplicate and escaped-equivalent keys at any depth must survive in encounter
   order; no dictionary or ambiguous JSON Pointer traversal may erase/reject them.
2. Lossless numeric spelling must survive negative zero, trailing fractional zeros,
   exponent case/sign/zeros, huge exponents and 5,000-digit integer tokens.
3. JSON-looking punctuation inside strings and odd/even backslash runs must never
   affect layout; all control and Unicode encoding must follow the pinned transport.
4. Wrapper errors take precedence over malformed selected documents; full Unicode
   validation covers ignored wrapper/document content, and fatal bounds propagate.
5. Large plain keys/values and 64 open containers must work, with exact raw stdout
   on success and no partial result when input/output/resource limits fail.

## Task 1: Independent oracle and frozen R023 inputs

Create only `/tmp/ail-pretty-stage/bench/R023-json-pretty/` files:
`intent.md`, `reference.py`, `cases/*.args`.
Consumes: binding spec and documented Json transport, no AIL implementation.
Produces: independent request-to-result oracle reading explicit data.request argv,
printing `out.result=hex:<compact result Json UTF-8 bytes>`; manual/generated checks and frozen cases.

- [x] Parse objects into ordered member pairs distinct from arrays and numbers into
  opaque non-string tokens. Fully validate Unicode, preserving duplicate names and
  number spellings; implement wrapper REQUEST versus selected-document JSON errors.
- [x] Render the parsed tree recursively using exact two-space layout and an
  independent canonical string encoder. Public oracle helpers should expose parse,
  render, render_text, evaluate_request and encode for later read-only acceptance.
- [x] Pin hand assertions for every root kind, empty/nested containers and the
  duplicate-a example in spec. Compare parsed input/output ordered-pair/token trees;
  include seeded recursive trees, string punctuation/escapes and numeric spellings.
- [x] Author named adversarial cases for all five review concerns and exact required
  32,768-scalar values/keys, 5,000-digit integer, 128 array elements/object pairs,
  64 open containers. Valid frozen cases fit existing runtime structural/argv bounds.
  Use literal UTF-8 for long Unicode and escape line-split-sensitive scalars in args.
- [x] Run all oracle CLI cases through unchanged benchmark loading, then actual
  unchanged evaluator missing-program RED. Record commands/statuses with finite
  deadlines, no bytecode caches in the staged rung, exact membership/hashes/fingerprint.
- [x] Independent review and fix material findings before freeze. Root integrates
  approved oracle/input bytes; no AIL source exists before this gate passes.

## Task 2: Pure formatter and Analyze applications

Create exactly eight files:
- `examples/ail_std_json_pretty.ail/{ail-package.md,spec.ail-spec.md,README.md}`
- `examples/json-pretty/{ail-package.md,spec.ail-spec.md,README.md}`
- `bench/R023-json-pretty/program.ail/{ail-package.md,spec.ail-spec.md}`
Consumes: frozen Task1 oracle/inputs and existing compiler/runtime.
Produces `ail.std.json.pretty` 0.1.0, alias Pretty, with
`render(value:Json)->Text`, `renderText(text:Text)->Text`,
`renderRequest(text:Text)->Json`; app and benchmark Analyze/data.request:Text →
compact structured Json UTF-8 Bytes → selected out.result:Bytes,
trace Rendered, importing the same library.

- [x] Record direct-library missing-resolution RED before source for duplicate-key
  layout, string punctuation and large-number spelling. Keep the actual pre-source
  probe bytes; use valid existing source syntax such as application `manages`.
- [x] Implement pure rendering and parse/request adapters with exact layout and
  narrow domain recovery. Use bulk text runs or bounded-window scanning so long
  plain strings do not require a root loop per scalar. Preserve duplicate names.
- [x] Write coherent library/app guides. Library README must already contain seven
  meaningful required sections: Purpose, Concepts Taught, Files To Inspect, Expected
  Replay Artifacts, Rejected Fixtures, Next Example To Read, v0.3 Learning Signal.
- [x] Frontload exact ordinary VM/native report smokes. Execute unchanged R023
  evaluator on VM/native plus independent direct render/renderText probes for
  layouts, string delimiters/escapes, numbers, all useful sizes and fatal depth.
- [x] Check representative Core/STE execution and actual pure IR; freeze source,
  evidence and artifacts with finite deadlines/cleanups and honest failure history.
- [x] Independent task review; fix findings, freeze eight files. No oracle/compiler/
  old rung/script/test edits, runtime padding or weakened size expectations.

## Task 3: Stdin, full independent acceptance and documentation

Extend app manifest/spec/README; create `input.json`, `expected.json`,
`scripts/ail_json_pretty_differential.py`, `examples/ste/json-pretty.ail-ste`,
`docs/ail/63-json-pretty.md`. Narrow navigation/inventory edits only in README.md,
docs/README.md, docs/ail/README.md, docs/ail/25-example-inventory.md,
examples/README.md and examples/support-packages.md. Existing library/benchmark
source and manifests remain frozen; root supplies pretask baseline/preimages.
Consumes frozen Pretty API, Analyze and approved oracle expectations.
Produces bounded Format raw-stdin behavior and standalone acceptance/docs.

- [x] Prove missing Format action RED, then read raw stdin, call renderText, append
  exactly one LF and write under --stdio. Preserve Analyze. Empty/malformed JSON
  fails with no stdout; stdin is one document, not wrapper or JSON-lines protocol.
- [x] Build and frontload ordinary report smokes for VM/native app and direct-render
  probe. The probe encodes returned pretty Text as UTF-8 Bytes to transport all
  layout and Unicode scalars safely; its independent expectation is the oracle string.
- [x] Build 14 app and 14 direct-render routes, including separate execution AND
  native compilation of saved bytecode. Compare every frozen request and direct
  hand/constructed render vector exactly; inspect pure semantic IR.
- [x] Check actual Format stdin across all app routes: scalar/empty/nonempty/nested
  documents, duplicate/escaped keys, number tokens, Unicode/control strings,
  already-pretty input, no final LF and useful sizes. An exactly 1 MiB total input
  containing `null` plus JSON whitespace succeeds; 1 MiB+1 fails TEXT-LIMIT.
  Missing grant, invalid UTF-8, malformed JSON/Unicode, excessive Json depth and
  representative formatting expansion/allocation limits fail without output/leaks.
- [x] In a copied library mutate one real layout decision; independent exact bytes
  must reject actual wrong output with successful status on VM/native. Compilation
  failure or mismatched ordinary reporting does not count as semantic detection.
- [x] Document API, layout/preservation, both error layers, stdio commands, samples,
  measured bounds and exact export. Register support-only library and actual32
  suffix packages, preserving all prior registrations and128 catalog/story/request/
  response counts. No caller-file/JSON-lines/indent/sort options added.
- [x] Run exact tests `docs_example_inventory_names_every_package_directory`,
  `example_package_directories_all_have_learning_guides`,
  `cli_ail_e2e_corpus_accepts_declared_support_only_package_directory` in
  `ail_toolchain`, and `checked_example_packages_keep_their_bytecode_through_ste`
  in `ste_pipeline`. Also inspect actual support-loader closure with scoped
  positive/negative/restored probes, local links, exact STE and sample commands.
- [x] Independent task review, then root's final metadata/whole-milestone review.
- [x] Root revalidates prior inputs and saved full scoring, guardedly records22/22
  through unchanged lock writer only after all gates, closes this plan and keeps
  the broader language goal active.

## Preflight and execution ledger

- Baseline: /tmp/ail-pretty-baseline.json is the byte-identical 2,059-file completed
  word-frequency snapshot, SHA256537ed62a1bf4d6dafe58236d99df0a6fcba22f5846533a026990e1a6f7365603.
  Relevant preimages /tmp/ail-pretty-baseline. Prior milestone complete at21/21;
  /tmp/ail-words-final-summary.json and post-publication audit pass. Do not repeat
  its completed compiler/acceptance/benchmark gates as preparation.
- Ruling: proceed with written design/plan under standing continuous reversible
  language-development authority. No redundant design/permission pause, git mutation
  or workspace cleanup. Scope mistakes cost local reviewable rework.
- Spec/plan self-review: exact layout, semantic preservation versus original byte
  preservation, wrapper/document error priority, empty stdin, fatal bounds, API,
  output names and routes agree. Required sizes and prior-scope boundaries are
  explicit; no placeholder or implied compiler change remains.

| Tasks/interface | Producer and consumer agreement | Ruling |
| --- | --- | --- |
| 1 ↔ 2 | Explicit data.request wrapper; complete out.result Bytes transport of result Json. | Independent tree-render oracle reviewed/frozen before lexical AIL source. |
| 1 ↔ 3 | Same frozen wrapper cases and read-only oracle functions; direct render values independent. | Test complete pretty bytes through Bytes probe transport and raw stdin separately. |
| 2 ↔ 3 | Same app Analyze later gains Format; library and benchmark frozen. | Sequential ownership, pure library versus action I/O. |
| 1 internal | Ordered pairs/tokens preserve duplicates/numbers; no AIL-derived expected output. | Valid structural/argv sizes in rung; fatal cases remain separate acceptance. |
| 2 internal | Three exact signatures and helpers; mandatory guide headings already required. | Known-valid missing-import probes and exact ordinary report smoke before matrices. |
| 3 internal | Raw JSON document, no final library LF, one app LF; bounded errors before write. | Routes and stdin expectations test this distinction, not backend self-agreement. |
| 3 ↔ root | Agent app/script/docs ownership; controller score/history/final review. | Root metadata edits after Task3 freeze/review. |

- Ruling: after Task2 review freezes executable library/benchmark inputs, root may
  run full22-rung scoring once concurrently with Task3. Use unchanged main without
  lock flags and preserve each result plus exact compiler/program/import/oracle
  dependency hashes. Only non-executable LADDER/JOURNAL status edits may differ
  before final lock publication. Original SCORE must match until writer executes.
  Changed semantic dependencies or incomplete scoring require discarding saved
  results and rerunning; no --bless-ladder. A mistaken scope assumption costs a
  full rerun and review, not acceptance of an unbound score.
- Models/roles: standard gpt-6.1-sol/high implementers for prose-defined integration;
  gpt-6-astra/high independent task and whole-milestone reviews. Fresh contexts,
  sequential product owners, scoped SDD fix loops. Root artifacts remain under
  /tmp/ail-pretty-*; no cleanup or git mutation despite generic skill defaults.

- Ruling: preimplementation /tmp/ail-pretty-transport-probe.py reproduces truncation
  of literal U+2028/U+2029 by unchanged bench.state_lines (Python splitlines).
  Analyze therefore uses existing UTF-8 Bytes transport of its compact structured
  Json result, and direct-render probes use UTF-8 Bytes for returned pretty Text.
  This preserves all Unicode cases and compares complete result bytes without
  modifying the old runner or Json encoder. Public library result types and raw
  Format behavior remain as specified. Cost: Analyze state is hex-encoded and
  needs decoding for direct human reading; the raw Format interface stays ordinary
  pretty JSON. This choice is made before any oracle or AIL implementation.

- Task1 dispatched to /root/pretty_oracle with /tmp/ail-pretty-task1-brief.md.
  Stage-only independent tree-render oracle and Bytes transport expectations;
  no AIL source, compiler build or repository edits authorized for this task.

- Task 1: complete (no commits; exact staged inputs integrated after independent
  spec PASS / quality APPROVE, no findings). 164 inputs / 162 CLI cases:
  82 success, 45 JSON, 35 REQUEST; 19 manual vectors, 400 seeded recursive
  identity properties and 40 direct invalid-Unicode/whitespace checks. Actual
  unchanged-evaluator RED was missing program.ail, before any AIL source.
  Fingerprint sha256:611860d22426e9cba9d97d4ba0dcde8d. Manifest
  /tmp/ail-pretty-task1-hashes.json SHA256
  491cdc40489320fbccc0a441a381ff267443b4ea62dc2d59dd5e0441f4b0d6fd;
  review /tmp/ail-pretty-task1-review.md SHA256
  d3951d20670aac4cb55daebbe8600ea8975dfd87ddb0f584605e171041ca997d.
  Root verified exact membership, all evidence and 2,059 unchanged prior files.
  The manifest records the plan dependency before these root ledger updates.
  Preserved harness corrections: trailing U+2003 stripped by loader moved before
  root; literal NUL impossible in argv replaced by U+0001 with direct NUL check.

- Task 2: complete (no commits; spec PASS / quality APPROVE, no findings).
  Pure five-function library and identical Analyze specifications frozen across
  eight new files. Accepted evidence: post-smoke 162-case VM/native evaluator,
  344 direct comparisons, 22 representative routes, 6 app + 4 direct exact report
  smokes, 18 fatal depth/fuel/Text-allocation checks, pure IR and guide headings.
  Report /tmp/ail-pretty-task2-report.md and manifest
  /tmp/ail-pretty-task2-hashes.json SHA256
  9282aff123e78f6049eac60a1962a18deb42d3bc3a7ce3fdfc0e9ee24263b28f;
  review /tmp/ail-pretty-task2-review.md SHA256
  ce0c32f1ff6ee0b28dd3bae8df71b62929e289150807197f577d99a3421d0dd3.
  Root verified 8 owned, 33 evidence, 60 artifact hashes, 164 frozen oracle inputs,
  unchanged compiler and all 2,059 baseline files. Early evaluator ran before
  passing report smokes; its result is retained separately, followed by the
  accepted post-smoke run. Report preserves all report/IR/allocation probe
  corrections; no pristine chronology or guessed fatal category is claimed.

- Task 3: complete; independent spec PASS / quality APPROVE, no findings.
  Manifest /tmp/ail-pretty-task3-hashes.json SHA256
  65f1d838568c729df01fb87a6359c57ee305147a6ecf1c3d6e825e8d722eac3d;
  review /tmp/ail-pretty-task3-review.md SHA256
  e371f31dd4680226242177351b7cec3580bb1a58b46f39133630c6af82b2c820.
  Root verified 14 owned files, 2,224 unchanged files, 26 evidence inputs and
  18,596 artifacts; 164 oracle inputs, five library/benchmark files and compiler
  remain frozen. Accepted complete behavioral invocation: 5,044 checks across
  5,088 commands in 933.5551 seconds; 14 app and 14 direct-render routes.
  Counts: 2,268 frozen requests, 1,246 direct renders, 1,310 raw successes,
  208 fatal/input failures, ten report/raw smokes and two successful semantic
  mutations rejected. Nine actual IR inspections are separately counted.
  Four exact Rust tests, sample/export commands and real support closure
  positive/negative/restored probes pass; 163 local links at the task boundary.
  All 32 suffix packages and 128 catalog/story/request/response entries retained.
  Historical attempts remain explicit: initial wrong-type diagnostic expectation;
  reconstructed first-attempt source; controller-interrupted partial attempt with
  1,038 command records and unwritten final output files; its fuel assertion
  never executed. Only the final complete run supplies behavioral assertions.
  Initial missing literal guide-index entry was corrected before passing gates.

- Ruling: accept one complete behavioral invocation plus a separate executed
  identity-only gate on the final permanent script. After the complete run, the
  sole source change adds runtime/ and build.rs to dependency roots. Behavioral
  predecessor SHA25658dde84d598e515cd284d0acadc7d5075baa4b2961397ee0e665362e123de04a;
  final SHA256c40d2f4313a1ffe44e277c7e18b39b8146ce039ea567f581814e3e22f5660699.
  /tmp/ail-pretty-task3-identity-gate.json and the exact source diff bind both.
  The final function was executed: precisely build.rs and runtime/float_runtime.rs
  were added, their hashes match the pretask baseline, and all old dependencies
  except the harness's own hash, compiler and frozen inputs remain unchanged.
  Task3 independent review approves this composition. No claim that one invocation
  ran every final assertion is made. Risk of misclassifying a behavioral change
  is controlled by the exact one-line diff and task/whole-milestone reviews.
  Repeating the unchanged full matrix is not required for this identity-only delta.

- Full unchanged benchmark: 22/22 passed in 1,059.65 seconds, with 1,152 pinned
  dependencies. Results /tmp/ail-pretty-full-benchmark-results.json; root checked
  dependency identities and original 21-rung score lock after completion. No lock
  flag was used; score recording awaits the final whole-milestone review.

- Whole-milestone review: spec PASS and quality APPROVE; see
  /tmp/ail-pretty-final-review.md and the frozen validation index. All required
  task, acceptance, documentation, package and scope gates passed. The index
  and individual reports retain actual commands, identities and harness history.
- Score recording: unchanged ail_bench.write_lock records the saved complete
  22/22 results only after required reviews/gates and fresh dependency checks.
  All 21 prior fingerprints/pass entries remain, and the R023 fingerprint is
  sha256:611860d22426e9cba9d97d4ba0dcde8d. No --bless-ladder, repeated full
  benchmark, git mutation, dependency change or external publication occurred.
- JSON pretty-printer milestone complete. /tmp/ail-pretty-final-summary.json,
  /tmp/ail-pretty-score-update.json and /tmp/ail-pretty-validation-index.md retain
  the measured scope and evidence. Sources and documentation remain in the
  workspace. The broader Python-replacement and AI-agent language goal stays active.
