# AIL Specification

AIL means Agentic Intent Language.

AIL is a semantic programming language and toolchain for humans and AI agents.
Humans begin in English, AI agents help clarify and structure intent, the
toolchain normalizes accepted programs into canonical AIL-Core IR, and checked
artifacts can render back into structured English, no-code views, diagnostics,
traces, bytecode, native executables, and low-level explanations.

## Reference Conventions

Read `28-language-reference-style.md` as the conventions document for the
numbered reference. It defines authority labels, rule identifiers, notation,
version blocks, implementation-note boundaries, and conformance links used by
the rest of the suite.

## Read Order

1. `00-foundation.md`
2. `01-language-architecture.md`
3. `02-structured-spec.md`
4. `03-semantic-ir.md`
5. `04-no-code-views.md`
6. `05-agent-protocol.md`
7. `06-agent-tools.md`
8. `07-types-values-effects.md`
9. `08-failures-guarantees-traces.md`
10. `09-system-profile.md`
11. `10-meta-profile.md`
12. `11-round-trip-equivalence.md`
13. `12-training-corpus.md`
14. `13-bootstrap-self-hosting.md`
15. `14-evolution-protocol.md`
16. `15-toolchain-implementation-guide.md`
17. `16-implementation-readiness-checklist.md`
18. `17-execution-semantics.md`
19. `18-ail-core-schema.md`
20. `19-agent-prompt-pack.md`
21. `20-standard-library-and-packages.md`
22. `21-c-interop-abi.md`
23. `22-backend-portability.md`
24. `23-ui-profile.md`
25. `24-diagnostics-catalog.md`
26. `25-example-inventory.md`
27. `26-semantic-safety-model.md`
28. `27-desired-outcome-traceability.md`
29. `28-language-reference-style.md`
30. `29-first-version-completion-gate.md`
31. `30-next-version-completion-gate.md`
32. `31-v03-learning-and-authoring-gate.md`
33. `32-capability-ladder-and-improvement-loop.md`
34. `33-ste-intermediate.md`

## Specification Contract

```text
human English
  -> AI-assisted interview
  -> AIL-Requirements
  -> AIL-Spec Canonical
  -> AIL-Core canonical semantic graph
  -> checked program artifact
  -> AIL-STE executable controlled English
  -> AIL bytecode, VM/native/Wasm/interoperability artifacts, and projections
```

The compiler accepts checked deterministic artifacts, not free-form
conversation. The AI agent may draft, repair, and explain those artifacts, but
the trusted checker is the authority for acceptance.

## Reference Status / Versions

| Surface | Current status |
| --- | --- |
| Language reference | draft `ail-reference.draft` |
| AIL-Core schema | target `ail-core.schema.v0`; stage-0 text artifact is normative for the bootstrap compiler |
| Prompt pack | draft prompt-pack with JSON envelope, `AIL-PROMPT-001` protocol checks, and offline stored-output corpus verification |
| Bytecode | stage-0 VM JSON plus native Linux x86_64 ELF target |
| STE intermediate | `AIL-STE 1`, executable programming profile based on ASD-STE100 Issue 9; full standard conformity is not claimed |
| Standard library | local package imports support exact version checks; standard library packages and range resolution are not yet versioned |
| Conformance suite | `first-slice` package fixtures and profile fixtures |

This table is the draft version heading for the active reference. Any
normative feature added after `28-language-reference-style.md` must either fit
one of these version surfaces or add a versioned surface through
`14-evolution-protocol.md`.

## Examples

- `../../examples/README.md`
- `../../examples/examples.md`
- `../../examples/support_ticket.ail/spec.ail-spec.md`
- `../../examples/support_ticket.ail/checked.ail-core.md`
- `../../examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- `../../examples/refund_tool.ail/spec.ail-spec.md`
- `../../examples/refund_tool.ail/checked.ail-core.md`
- `../../examples/compiler_pass.ail/spec.ail-spec.md`
- `../../examples/compiler_pass.ail/checked.ail-core.md`
- `../../examples/network_driver.ail/spec.ail-spec.md`
- `../../examples/network_driver.ail/checked.ail-core.md`
- `../../examples/recursive_factorial.ail/spec.ail-spec.md`
- `../../examples/option_map.ail/spec.ail-spec.md`
- `../../examples/stateful_counter.ail/spec.ail-spec.md`
- `../../examples/repeated_task.ail/spec.ail-spec.md`
- `25-example-inventory.md`

## Manual

- `manual/README.md`: interactive manual index and deterministic runner for
  authoring chapters.
- `manual/01-user-story-mode.md`: story-first authoring with `ail-story`,
  checked requirements, accepted spec, checked Core, bytecode, agent trace, and
  native runtime-trace evidence, plus live llama.cpp harness evidence,
  including story-promotion import-demo evidence for corpus-copy promotion.
- `manual/02-examples-release.md`: full examples replay, release evidence,
  model/executor manifest, and learning artifact review.
- `manual/03-prompt-interaction.md`: prompt corpus, prompt-surface replay,
  transcript capture help, and hosted prompt harness review.
- `manual/04-agent-entrypoint.md`: Codex role contracts and the AIL-authored
  toolchain-agent build entrypoint.
- `manual/05-v03-roadmap.md`: direct `ail-v03-roadmap` backlog view over the
  examples-derived learning signals.
- `manual/06-v03-authoring-gate.md`: one deterministic audit over User Story
  mode, examples replay, roadmap, prompt, agent, bootstrap self-hosting,
  Systems profile, stateful runtime, Application baseline, repair-promotion,
  UI patch import, and AgentTool policy import checks.
- `manual/07-repair-promotion.md`: deterministic review and batch audit of
  rejected-example repair tutorials, corrected specs, checked Core, bytecode,
  repair evidence, diffs, and promotion reviews before proposing repaired
  artifacts for accepted-corpus promotion.
- `manual/08-ui-patch-import.md`: deterministic review of UI patch plans before
  importing a human-approved `ail-flow-edit` candidate into a replayed corpus
  copy, then writing visual-regression and runtime UI-state evidence for the
  imported patch.
- `manual/09-agent-policy-import.md`: deterministic review of AgentTool policy
  handoff artifacts before importing a human-approved policy trace amendment
  into a replayed corpus copy, writing a role-separated handoff witness, and
  reviewing optional hosted AgentTool reviewer evidence.
- `manual/10-bootstrap-self-hosting.md`: deterministic bootstrap bundle for
  the AIL-authored toolchain agent and AIL-Meta compiler pass, including
  fixed-point, native-bytecode, host-boundary, dependency, handoff, and manifest
  evidence.
- `manual/14-turing-core.md`: deterministic recursive Turing Core conformance
  evidence for base-case recursion, explicit stack bounds, well-founded
  termination measures, and `AIL-CONTROL-003` rejected fixtures.
- `manual/11-systems-profile.md`: deterministic Systems profile evidence for
  `network_driver.ail`, including scheduler and interrupt conformance fixtures,
  Linux x86_64 native target artifacts, receive/transmit/interrupt-handler
  runtime traces, and unsupported-target migration guidance.
- `manual/12-application-baseline.md`: deterministic Application baseline
  evidence for `support_ticket.ail`, including package-local accepted and
  rejected conformance fixtures for support-ticket authoring boundaries.
- `manual/13-stateful-runtime.md`: deterministic stateful Application runtime
  evidence for `stateful_counter.ail`, including persistence, idempotency,
  shared-state serialization, replay-recovery fixtures, and counter VM trace
  output.

## Versioned Assets

- `prompts/`: agent prompt pack artifacts
- `manual/`: runnable manual chapters for authoring workflows
- `corpus/`: conformance and training fixtures
- `24-diagnostics-catalog.md`: stable diagnostic IDs
- `27-desired-outcome-traceability.md`: outcome-to-artifact matrix
- `28-language-reference-style.md`: normative rule style, grammar notation,
  versioning, and conformance-link rules
- `29-first-version-completion-gate.md`: v0.1 completion definition, evidence
  gates, release audit commands, and required release artifacts
- `30-next-version-completion-gate.md`: v0.2 completed release evidence,
  package and host-boundary portability definition, evidence gates, release
  audit commands, and required release artifacts
- `31-v03-learning-and-authoring-gate.md`: v0.3 release completion, learning,
  authoring, interactive manual, example-usefulness, and
  `scripts/run_v03_release_audit.py` audit bar that builds on the v0.2 release
  evidence
- `v03-roadmap-signal-status.md`: machine-checkable promoted/deferred status
  registry for high-count v0.3 roadmap signals

## Implementation Start

Use `15-toolchain-implementation-guide.md` as the implementation reference and
`16-implementation-readiness-checklist.md` as the readiness gate. The first
vertical slice is the support-ticket package, followed by agent-tool, systems,
compiler-pass, conformance, and native Linux ELF workflows.
Use `29-first-version-completion-gate.md` before claiming AIL v0.1 complete.
Use `30-next-version-completion-gate.md` before claiming AIL v0.2 complete.
Use `31-v03-learning-and-authoring-gate.md` before claiming AIL v0.3 complete.

- [`34-integer-expressions.md`](34-integer-expressions.md): typed integer computation, ordered calls, result selection, and current capability limits.
- [`35-control-flow.md`](35-control-flow.md): typed comparisons, source branches and loops, Core validation and per-action execution limits.

- [`36-integer-functions.md`](36-integer-functions.md): named arguments, isolated frames, recursion and early returns through STE and native code.
- [`37-text-operations.md`](37-text-operations.md): Unicode joins, character counts, slices and searches with checked types and runtime limits.
- [`38-typed-lists.md`](38-typed-lists.md): integer and text list creation, length, indexing, append and replacement through Core, STE and both runtimes.
- [`39-typed-literals.md`](39-typed-literals.md): source constants checked against declared types, with opaque JSON data through Core and STE.
- [`40-file-resources.md`](40-file-resources.md): named UTF-8 file resources, read/write grants, limits and VM/native behavior.
- [`41-json-conversion.md`](41-json-conversion.md): strict typed JSON parsing, canonical formatting, allocation accounting and file computation.
- [`42-recursive-json.md`](42-recursive-json.md): nested records, mixed arrays, lossless numbers and typed path lookup.
- [`43-json-mutation.md`](43-json-mutation.md): JSON set/remove, typed replacements, array append and record-to-report file transformations.

- [`44-value-functions.md`](44-value-functions.md): typed parameters/results, local transformations, recursive frames and a reusable JSON file report.
- [`45-float-computation.md`](45-float-computation.md): binary64 arithmetic and controls, explicit conversions, finite JSON boundaries and a numeric file report.
- [`46-floating-lists.md`](46-floating-lists.md): floating lists through source/Core/STE, borrowed function arguments, JSON conversion and a statistics file report.
- [`47-json-inspection.md`](47-json-inspection.md): JSON kind inspection, ordered key enumeration and generic recursive file traversal.
- [`48-boolean-predicates.md`](48-boolean-predicates.md): lazy Boolean logic, typed scalar comparisons and reusable record-filter predicates.
- [`49-text-transformations.md`](49-text-transformations.md): split, list join, replace and Unicode trim with a reusable TSV file report.
- [`50-list-transformations.md`](50-list-transformations.md): slicing, concatenation, reversal and stable sorting with a file statistics report.
- [`51-runtime-recovery.md`](51-runtime-recovery.md): explicit data-error handlers, call unwinding and a recovering JSON-lines batch report.
- [`52-standard-streams.md`](52-standard-streams.md): explicit stdin/stdout/stderr, quiet execution and a composable JSON-lines filter.

- [`53-process-resources.md`](53-process-resources.md): caller-bound external tools, bounded capture, timeouts and an AIL tool coordinator.

- [`54-directory-resources.md`](54-directory-resources.md): separate directory read/write grants, checked relative paths, sorted listing and discovered-file reports.

- [`55-expression-calculator.md`](55-expression-calculator.md): an AIL-authored tokenizer, parser and evaluator, reusable imports, structured errors and JSON-lines requests.

- [`56-bytes.md`](56-bytes.md): immutable byte values, UTF-8/hex conversions, typed functions and binary file operations.

- [`57-explicit-errors.md`](57-explicit-errors.md): explicit errors in actions and typed functions through checked Core, STE and both runtimes.
- [`58-run-length-codec.md`](58-run-length-codec.md): RLE32 encoding/decoding and size preflight in pure AIL, plus a binary-file application.
- [`59-executable-variants.md`](59-executable-variants.md): executable recursive variants, schema-checked payloads, generic data and typed function boundaries.

- [Trustworthy edits](60-trustworthy-edits.md): preserve-by-default authoring, explicit expected targets, structural proof limits and independent edit verification; this authoring-correctness milestone adds no ladder rung.

- [CSV aggregation](61-csv-aggregation.md): pure CSV parsing and checked category totals, JSON-lines requests, caller-granted file reports and independent VM/native acceptance.

- [Word frequency](62-word-frequency.md): pure exact tokenization/counting, deterministic ranking and bounded raw stdin.

- [JSON pretty printing](63-json-pretty.md): pure lossless layout, request errors, Bytes transport and bounded raw stdin.

- [LRU cache](64-lru-cache.md): immutable pure cache state, exact events and bounded raw stdin.

- [Conway Life](65-conway-life.md): immutable finite grids, simultaneous B3/S23 and bounded raw stdin.
