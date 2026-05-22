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
