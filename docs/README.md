# EIGL Specification Pack

**EIGL** stands for **Executable Intent Graph Language**.

This package captures the design direction discussed so far: a requirements-first, graph-native, compiled programming language intended to be understandable by humans with little or no software-development background, useful for LLM-assisted authoring, visualizable as meaningful diagrams, and compilable into safe, performant machine code.

The current design target is:

```text
As concise as Python at the requirement surface.
As explainable as structured English in the intermediate layer.
As safe as Rust in the compiler core.
As optimizable as a static IR in the backend.
```

The central architectural idea is that **the human-facing text is not the final source of truth**. Humans write compact controlled requirements. Those requirements are elaborated into a still-human-readable intermediate format, which is then lowered into a typed executable semantic graph and finally into compiler/runtime IR.

```text
RSL       Requirement Surface Language
          Concise controlled requirements written by humans.

RIF       Readable Intent Format
          Normalized, explicit, explainable human-readable meaning.

EIG-Core  Executable Intent Graph
          Canonical typed semantic graph with permissions, effects, contracts, failures, and provenance.

EIG-IR    Compiler/runtime intermediate representation
          Lower-level representation for native code, Wasm, bytecode, workflow runtimes, or accelerators.

EIG-Meta  Language/compiler specification layer
          The layer used to describe the language, compiler passes, diagnostics, visualizations, and lowering rules.
```

## Files

Read in this order:

1. [`00-codex-handoff.md`](00-codex-handoff.md) — ready-to-paste handoff prompt for Codex or another coding agent.
2. [`01-vision-and-principles.md`](01-vision-and-principles.md) — core goals and design principles.
3. [`02-language-architecture.md`](02-language-architecture.md) — full pipeline from requirements to executable code.
4. [`03-rsl-requirement-surface-language.md`](03-rsl-requirement-surface-language.md) — the compact human-facing language.
5. [`04-rif-readable-intent-format.md`](04-rif-readable-intent-format.md) — the explicit intermediate format.
6. [`05-eig-core-semantic-graph.md`](05-eig-core-semantic-graph.md) — canonical typed graph model.
7. [`06-safety-permissions-effects.md`](06-safety-permissions-effects.md) — Rust-like safety model in human terms.
8. [`07-visualization-and-views.md`](07-visualization-and-views.md) — meaningful visual projections of programs.
9. [`08-compiler-generation.md`](08-compiler-generation.md) — how to generate the compiler from language specs.
10. [`09-self-hosting-bootstrap.md`](09-self-hosting-bootstrap.md) — path toward writing the compiler in the language itself.
11. [`10-prototype-roadmap.md`](10-prototype-roadmap.md) — practical implementation plan.
12. [`11-examples.md`](11-examples.md) — end-to-end examples.
13. [`12-open-questions.md`](12-open-questions.md) — unresolved design choices.
14. [`EIGL_FULL_SPEC.md`](EIGL_FULL_SPEC.md) — all files concatenated into one long document.

## Intended use

This is not yet a finalized language standard. It is a structured design package that can be given to Codex to begin implementing a prototype.

A sensible first prototype should **not** attempt the full natural-language front end. It should start with:

```text
RIF parser
EIG-Core graph schema
basic type checking
basic permission/effect/failure checking
flow/failure/permission view generation
small bytecode interpreter or Wasm backend
```

Only after that should the compact RSL layer and compiler-generation layer be implemented.
