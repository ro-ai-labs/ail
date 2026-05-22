# AIL Foundation

## Name

AIL means Agentic Intent Language.

The name carries three requirements. AIL is agentic because AI agents are
first-class authors, explainers, debuggers, tool users, and tool builders. AIL
is intent-oriented because humans begin from outcomes and constraints instead
of implementation syntax. AIL is a language because it is intended to become a
complete programming language, not a prompt format, workflow DSL, or no-code
product.

Older files in this repository may still use EIGL names. New specifications use
AIL names.

## Goal

AIL is a semantic programming language and toolchain for humans and AI agents.

Humans describe the application, tool, compiler pass, runtime behavior, or
system they want in ordinary English. An AI Agent asks focused questions until
the missing semantics are explicit. The accepted result is not the
conversation. The accepted result is AIL-Core plus validated metadata, rendered
through deterministic human-readable projections.

Long term, AIL must be powerful enough to define its own compiler, runtime,
standard library, package system, debugger, agent protocol, and build system.

## Core Thesis

AIL makes programming understandable at the level of intent while remaining
precise enough for compiler-grade checking, optimization, tracing, and
execution.

```text
English idea
  -> AI-assisted clarification
  -> AIL-Spec structured English
  -> AIL-Core canonical semantic graph
  -> checked program
  -> executable artifact

AIL-Core
  -> structured English
  -> no-code views
  -> traces
  -> low-level explanations
```

Every accepted executable behavior must be traceable back to human-confirmed
intent.

## Non-Negotiable Invariants

### English Starts The Programming Loop

English starts the authoring loop. Humans describe what they want in ordinary
English, and the AI Agent interviews them until required semantic details are
captured.

### Deterministic Artifacts Are Required

The compiler must not compile vague conversation. It compiles checked
deterministic artifacts derived from the conversation.

### The Canonical Source Of Truth Is Semantic

AIL-Core is the canonical semantic source of truth for accepted programs.
Structured English, graphical views, traces, diagnostics, and low-level forms
are deterministic projections of AIL-Core.

### Human Trust Comes From Projections

Non-engineers should not need to inspect Rust-like code. They must be able to
review structured English, cards, flows, data tables, rule views, permission
views, failure views, and traces.

### Round-Trip Equivalence Is Required

AIL must support Round-Trip checks:

```text
AIL-Spec -> AIL-Core -> AIL-Spec
AIL-Core -> AIL-Spec -> AIL-Core
AIL-Core -> AIL-Flow view -> AIL-Core patch
```

The authority is normalized semantic IR equivalence, not text similarity.

### The AI Agent Is Official But Untrusted

The AI Agent is part of the toolchain. It asks questions, writes AIL-Spec,
proposes patches, explains diagnostics, and debugs traces. It is not the
trusted compiler core. The checker validates every accepted deterministic
artifact.

### Features Must Be Explainable And Teachable

AIL features must be understandable by humans, teachable to LLMs, checkable by
the compiler, traceable at runtime, and compilable to efficient artifacts.

### The Toolchain Must Become Self-Sovereign

Legacy languages may bootstrap AIL, but they must not own AIL. Required
toolchain components should eventually be defined in AIL itself.

## Layer Model

AIL is one semantic language with multiple projections and profiles.

### AIL-English

AIL-English is ordinary conversation and prose. It is used for idea capture and
clarification, but it is not directly compiled.

### AIL-Spec

AIL-Spec is deterministic structured English. It is readable by English
speakers and precise enough to elaborate into AIL-Core.

### AIL-Flow

AIL-Flow is the no-code projection: cards, flows, tables, rule views, failure
maps, permission views, tool-capability views, and trace views. It is rendered
from AIL-Core and can produce validated graph patches.

### AIL-Core

AIL-Core is the canonical typed semantic graph. It represents applications,
things, fields, actions, events, tools, values, rules, contracts, permissions,
effects, failures, guarantees, provenance, views, traces, and low-level
obligations.

### AIL-System

AIL-System is the low-level profile for systems programming. It expresses
ownership, borrowing, regions, memory layout, allocation, ABI, scheduling,
concurrency, device access, runtime primitives, and backend lowering
obligations while remaining explainable in English.

### AIL-Meta

AIL-Meta is the profile used to define AIL itself: parsers, checkers, compiler
passes, diagnostics, renderers, agent prompts, lowering rules, optimizers,
tests, package metadata, and language evolution rules.

## Program Profiles

AIL supports multiple profiles over the same semantic substrate:

- Application Profile for apps, APIs, background jobs, workflows, services,
  dashboards, notifications, and integrations.
- Agent Tool Profile for tools AI agents can request under explicit permission,
  effect, approval, audit, and guarantee rules.
- Systems Profile for kernels, drivers, runtimes, schedulers, memory managers,
  filesystems, network stacks, embedded software, and performance-critical
  libraries.
- Compiler Profile for language definitions, checkers, diagnostics,
  transformations, lowering rules, optimizers, renderers, and build steps.
- Training Profile for examples, interviews, specs, IR, invalid examples,
  traces, diagnostics, round-trip tests, and evaluation data.

## Core Vocabulary

The initial vocabulary is intentionally small and regular:

- Application
- Thing
- Field
- Value
- Action
- Step
- Rule
- Event
- Tool
- View
- Input
- Output
- Permission
- Effect
- Failure
- Guarantee
- Secret
- Approval
- Trace
- Provenance
- Capability
- Region
- Layout
- Lowering
- Diagnostic

The same concepts must work across profiles. An API endpoint, AI-agent tool,
compiler pass, and device-driver operation all expose actions, inputs, outputs,
rules, effects, failures, guarantees, and traces.

## Self-Sovereign Toolchain Principle

The Self-Sovereign Toolchain principle says AIL may use existing languages and
systems as bootstrap scaffolding, but no required part of the mature AIL
toolchain may depend on a legacy language as its source of truth.

The long-term AIL-defined components include the language specification,
semantic IR schema, parser rules, checker rules, diagnostic rules, renderers,
no-code projections, debugger, agent protocol, compiler passes, optimizer
rules, lowering rules, runtime primitives, standard library, package system,
build system, and conformance suite.

## Readability Gate

A feature is not accepted into core AIL unless a non-engineer can inspect the
generated structured spec, no-code view, trace, and explanation and understand:

- what the program does
- what data it uses
- what it can change
- what can fail
- what happens on failure
- what secrets are protected
- what external systems are called
- why the behavior is allowed

## Flexibility Rule

Stable invariants define what AIL must always preserve. Versioned decisions
define the current shape of AIL-Spec, AIL-Core, AIL-Flow, AIL-System,
AIL-Meta, and the bootstrap toolchain. Experiments may explore alternative
English forms, agent prompts, renderers, equivalence scoring, and debugging
interactions without becoming core language rules.
