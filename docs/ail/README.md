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

## Specification Contract

```text
human English
  -> AI-assisted interview
  -> AIL-Requirements
  -> AIL-Spec Canonical
  -> AIL-Core canonical semantic graph
  -> checked program artifact
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
  live llama.cpp harness evidence, including story-promotion import-demo
  evidence for corpus-copy promotion.
- `manual/02-examples-release.md`: full examples replay, release evidence,
  model/executor manifest, and learning artifact review.
- `manual/03-prompt-interaction.md`: prompt corpus, prompt-surface replay,
  transcript capture help, and hosted prompt harness review.
- `manual/04-agent-entrypoint.md`: Codex role contracts and the AIL-authored
  toolchain-agent build entrypoint.
- `manual/05-v03-roadmap.md`: direct `ail-v03-roadmap` backlog view over the
  examples-derived learning signals.
- `manual/06-v03-authoring-gate.md`: one deterministic audit over User Story
  mode, examples replay, roadmap, prompt, agent, repair-promotion, UI patch
  import, and AgentTool policy import checks.
- `manual/07-repair-promotion.md`: deterministic review of rejected-example
  repair evidence before proposing a repaired artifact for accepted-corpus
  promotion.
- `manual/08-ui-patch-import.md`: deterministic review of UI patch plans before
  importing a human-approved `ail-flow-edit` candidate into a replayed corpus
  copy.
- `manual/09-agent-policy-import.md`: deterministic review of AgentTool policy
  handoff artifacts before importing a human-approved policy trace amendment
  into a replayed corpus copy, writing a role-separated handoff witness, and
  reviewing optional hosted AgentTool reviewer evidence.

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
- `30-next-version-completion-gate.md`: v0.2 package and host-boundary
  portability definition, evidence gates, release audit commands, and required
  release artifacts
- `31-v03-learning-and-authoring-gate.md`: v0.3 learning, authoring, and
  example-usefulness bar that builds on the v0.2 release evidence

## Implementation Start

Use `15-toolchain-implementation-guide.md` as the implementation reference and
`16-implementation-readiness-checklist.md` as the readiness gate. The first
vertical slice is the support-ticket package, followed by agent-tool, systems,
compiler-pass, conformance, and native Linux ELF workflows.
Use `29-first-version-completion-gate.md` before claiming AIL v0.1 complete.
Use `30-next-version-completion-gate.md` before claiming AIL v0.2 complete.
Use `31-v03-learning-and-authoring-gate.md` to decide how examples should move
the bar for AIL v0.3.
