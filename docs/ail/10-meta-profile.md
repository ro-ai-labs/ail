# AIL-Meta Profile

## Purpose

AIL-Meta is the profile used to define AIL itself. It represents language
definition packages, parser rules, checker rules, diagnostics, renderers,
agent prompts, compiler passes, lowering rules, optimizer rules, tests, package
metadata, and evolution proposals.

## Language Definition Packages

A language definition package contains versioned AIL-Meta declarations for a
language feature or profile. It includes syntax or projection rules, AIL-Core
node and edge mappings, checker rules, diagnostics, examples, conformance
tests, and migration notes.

## Complete Feature Package Example: Option<T>

An AIL-Meta package for `Option<T>` contains:

- AIL-Spec form:
  `Type: Option<T>. Option has variants Some(value: T) and None.`
- AIL-Core mapping:
  `Type Option<T>`, `Variant Some`, `Variant None`, and payload edge from
  `Some` to `T`
- checker rule:
  every `Match` over `Option<T>` must handle `Some` and `None`
- diagnostic rule:
  `AIL-CONTROL-002` when a finite variant match is non-exhaustive
- renderer rule:
  render `Some` and `None` as canonical structured English branches
- AIL-Flow block rule:
  Option Match block has two required sockets
- prompt rule:
  ask for absent-value behavior when user intent mentions optional data
- valid example:
  `Option.map` with `Some` and `None` branches
- invalid example:
  match with only `Some`
- round-trip fixture:
  AIL-Core match -> AIL-Spec -> AIL-Core preserves semantic hash
- trace fixture:
  `OptionMapEvaluated`
- migration notes:
  initial package version has no migration

Definition of acceptance: the bootstrap compiler consumes this AIL-Meta package
and generates at least one checker or renderer component recorded in the
conformance report.

The current executable model for this pattern is
`examples/compiler_pass.ail`, which defines the `InferReadPermissions`
compiler pass. The bootstrap toolchain lowers it to AIL-Core and bytecode,
executes it against a checked package graph, records diagnostics and traces,
and is covered by `cargo test --test ail_toolchain compiler_pass`.

## Compiler Passes As Actions

Compiler Passes are actions over compiler artifacts. A pass declares inputs,
outputs, reads, writes, failures, guarantees, traces, and lowering obligations.

## Checker Rules

Checker Rules are deterministic constraints over AIL-Core. A checker rule names
the graph pattern it accepts or rejects, the diagnostic it emits, and the
provenance it reports.

## Diagnostic Rules

Diagnostic Rules define how checker failures become human-readable messages,
repair suggestions, no-code highlights, and agent clarification questions.

## Renderer Rules

Renderer rules define how AIL-Core projects to AIL-Spec, AIL-Flow, traces,
diagnostics, and lower-level explanations. Renderers must preserve round-trip
equivalence.

## Agent Prompt Rules

Agent prompt rules define how the AI Agent asks questions, proposes patches,
uses tools, explains diagnostics, and debugs traces. Prompt rules are
untrusted, but versioned and testable.

## Lowering Rules

Lowering rules map checked AIL-Core into executable targets. They declare
preconditions, target obligations, failure modes, trace preservation, and
equivalence checks.

## Self-Hosting Role

The Self-Hosting role of AIL-Meta is to move required toolchain definitions
from bootstrap implementation languages into AIL-defined artifacts until AIL
can rebuild its own compiler, runtime, standard library, package system, agent
protocol, and build system.
