# AIL Language Evolution Protocol

## Purpose

The evolution protocol defines how AIL changes without breaking its core
invariants. It separates stable principles from versioned decisions and
experiments.

## Stable Invariants

Stable invariants include English-first authoring, deterministic artifacts,
AIL-Core as semantic source of truth, human-reviewable projections, round-trip
equivalence, untrusted AI agents, explainable features, and the self-sovereign
toolchain direction.

## Versioned Decisions

Versioned decisions define the current forms of AIL-Spec, AIL-Core, AIL-Flow,
AIL-Agent, AIL-System, AIL-Meta, packages, conformance tests, and bootstrap
behavior. They may change only through explicit proposals.

## Experimental Surfaces

Experimental surfaces may include alternate English forms, prompt templates,
visual renderers, embedding-based drift signals, debugging interactions,
lowering strategies, and editor workflows. They must not become compiler
authority until accepted.

## Proposal Requirements

Each language proposal must include:

- human-readable motivation
- non-engineer explanation
- AIL-Spec example
- AIL-Core example
- no-code rendering expectation
- round-trip expectation
- LLM prompt compatibility note
- diagnostic examples
- trace/debugging example
- implementation feasibility note
- compatibility impact

## Readability Gate

The Readability Gate requires non-engineers to understand the generated spec,
view, trace, and explanation for the feature.

## LLM Teachability Gate

The LLM Teachability Gate requires a compact rule, canonical form, valid
example, invalid example, diagnostic example, round-trip expectation, no-code
rendering expectation, and trace expectation.

## Compiler Checkability Gate

The Compiler Checkability Gate requires deterministic AIL-Core representation,
checker rules, diagnostics, conformance tests, and equivalence rules.

## Acceptance Process

A proposal is accepted when it passes the gates, preserves stable invariants,
has conformance tests, and identifies migration behavior for existing packages.

## Open Decisions For Later Specification

The current recommended default is to specify the semantic contract before
freezing surface syntax. Later proposals should decide exact package file
formats, textual grammar, binary graph format, standard library shape, backend
targets, and the first self-hosting subset.
