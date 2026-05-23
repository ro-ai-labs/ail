# AIL Documentation

AIL means Agentic Intent Language.

This directory is the active documentation root for the language, compiler, VM,
native Linux ELF backend, LLM-assisted agent workflow, and conformance suite.

## Read Order

1. [AIL Specification](ail/README.md)
2. [AIL Desired Outcome Traceability](ail/27-desired-outcome-traceability.md)
3. [AIL Implementation Readiness Checklist](ail/16-implementation-readiness-checklist.md)

## Current Direction

AIL lets humans begin in English while an AI agent asks clarifying questions,
captures sufficient requirements, drafts deterministic AIL-Spec, normalizes the
accepted program into AIL-Core IR, and invokes the compiler pipeline.

Checked AIL artifacts can render back into structured English, no-code views,
diagnostics, traces, manifests, fingerprints, AIL bytecode, and low-level
explanations. For Linux, the executable machine-level bytecode target is a
native ELF64 x86_64 executable emitted by the compiler.

## Active Specification Suite

The [ail](ail) directory defines the current language framework: foundation,
architecture, structured English, semantic IR, no-code views, agent protocol,
agent tools, type/effect model, failure and trace model, systems profile,
meta/compiler profile, round-trip equivalence, training corpus, bootstrap
self-hosting path, language evolution protocol, toolchain implementation guide,
implementation-readiness checklist, formal execution semantics, AIL-Core schema,
agent prompt pack, standard library and package model, C interop and ABI
profile, backend portability, UI profile, diagnostic catalog, example
inventory, semantic safety model, and desired-outcome traceability matrix.
