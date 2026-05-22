# AIL Language Foundation Design

## Status

This is the first foundation design for AIL, formerly explored in this
repository under the EIGL prototype name. It is not a frozen language
standard. It is a language constitution: it defines the durable goals,
invariants, architecture, and evolution protocol that later detailed
specifications must obey.

## Name

AIL means Agentic Intent Language.

The name reflects three design commitments:

- Agentic: AI agents are first-class authors, explainers, debuggers, tool users,
  and tool builders.
- Intent: humans begin from desired outcomes and constraints, not implementation
  syntax.
- Language: AIL is intended to become a complete programming language, not a
  workflow DSL, prompt format, or no-code product.

The existing Rust prototype and older documents may still use EIGL names while
the repository migrates. New language design work should use AIL names.

## Goal

AIL is a semantic programming language and toolchain for humans and AI agents.

Humans describe the application, tool, system, compiler pass, or runtime
behavior they want in ordinary English. An AI Agent helps clarify missing
details, writes deterministic structured specifications, converts those
specifications into a canonical semantic intermediate representation, and
renders that representation back into English, no-code views, traces, and
lower-level forms. The toolchain checks the deterministic artifacts and compiles
accepted programs into safe, fast executable artifacts.

Long term, AIL should be powerful enough to define its own compiler, runtime,
standard library, package system, debugger, agent protocol, and build system.
After bootstrap, the AIL toolchain must not depend on legacy programming
languages as its source of truth.

## Non-Goals For The First Specification

The first specification should not attempt to freeze every grammar detail,
backend detail, optimization rule, or user interface interaction.

It also should not pretend that unrestricted natural language can be compiled
directly. Free-form English is an authoring input. The accepted program is the
checked semantic representation plus deterministic human-readable projections.

## Core Thesis

AIL should make programming understandable at the level of intent while still
remaining precise enough for compiler-grade checking and optimization.

The central contract is:

```text
English idea
  -> AI-assisted clarification
  -> deterministic structured specification
  -> canonical semantic IR
  -> checked program
  -> executable artifact

canonical semantic IR
  -> structured English
  -> no-code views
  -> traces
  -> lower-level explanations
```

Every accepted executable behavior must be traceable back to human-confirmed
intent.

## Non-Negotiable Invariants

### English Starts The Programming Loop

Humans begin by describing the thing they want. The AI Agent interviews the
human until all required semantic elements are captured.

### Deterministic Artifacts Are Required

The compiler must not compile vague conversation. It compiles checked
deterministic artifacts derived from the conversation.

### The Canonical Source Of Truth Is Semantic

AIL-Core, the canonical semantic IR, is the source of truth for accepted
programs. Text, structured English, graphical/no-code views, traces, and
low-level forms are deterministic projections of that IR.

### Human Trust Comes From Projections

Non-engineers should not need to inspect Rust-like code. They should be able to
review structured English, cards, flows, data tables, rule views, permission
views, failure views, and traces.

### Round-Trip Equivalence Is Required

The system must support semantic round trips:

```text
structured English -> AIL-Core -> structured English
AIL-Core -> structured English -> AIL-Core
AIL-Core -> no-code view -> AIL-Core patch
```

The trusted equivalence check is normalized semantic IR equivalence, not raw
text similarity. Embedding distance may be used as an additional regression
signal, but it must not be the compiler authority.

### The AI Agent Is Official But Untrusted

The AI Agent is part of the toolchain. It asks questions, writes specs,
proposes patches, explains programs, generates tests, and debugs traces.

The AI Agent is not the trusted compiler core. The checker validates every
accepted deterministic artifact.

### Features Must Be Explainable And Teachable

AIL features must be understandable by humans, teachable to LLMs, checkable by
the compiler, traceable at runtime, and compilable to efficient artifacts.

### The Toolchain Must Become Self-Sovereign

Legacy languages may bootstrap AIL. They must not own AIL. Once the first AIL
compiler generations are viable, every required part of the toolchain should be
defined in AIL itself.

## Layer Model

AIL is one semantic language with multiple projections and profiles.

### AIL-English

AIL-English is ordinary human conversation and prose. It is useful for idea
capture and clarification, but it is not directly compiled.

Example:

```text
I want an app where support agents can open tickets, assign them, close them,
and see overdue tickets. Customers should get updates, but private internal
notes should never be visible to customers.
```

### AIL-Spec

AIL-Spec is deterministic structured English. It is readable by any English
speaker and precise enough to elaborate into AIL-Core.

Example:

```text
The application manages support tickets.

A ticket has:
- an id
- a title
- a status, which can be New, Open, Assigned, Closed, or Overdue
- an assignee
- public updates
- internal notes

When a support agent closes a ticket:
- the ticket must exist
- the ticket must not already be Closed
- the system changes the ticket status to Closed
- the system records a public update
- the system does not reveal internal notes to the customer
```

### AIL-Flow

AIL-Flow is the no-code projection: cards, flows, tables, rule views, failure
maps, permission views, tool-capability views, and trace views. It is rendered
from AIL-Core and can produce validated graph patches.

### AIL-Core

AIL-Core is the canonical typed semantic graph. It represents things, actions,
events, tools, values, rules, contracts, permissions, effects, failures,
guarantees, provenance, views, traces, and low-level obligations.

AIL-Core is the artifact the trusted checker validates.

### AIL-System

AIL-System is the low-level profile for systems programming. It expresses
ownership, borrowing, regions, memory layout, allocation, ABI, scheduling,
concurrency, device access, runtime primitives, and backend lowering
obligations.

AIL-System must still be explainable:

```text
This buffer is owned by the network driver while the packet is being processed.
The packet parser may read the buffer but may not change it.
The driver releases the buffer when the packet has been handled.
```

### AIL-Meta

AIL-Meta is the profile used to define AIL itself: parsers, checkers, compiler
passes, diagnostics, renderers, agent prompts, lowering rules, optimizers,
tests, package metadata, and language evolution rules.

## Program Profiles

AIL should support multiple profiles over the same semantic substrate.

### Application Profile

Interactive apps, APIs, background jobs, workflows, stateful services, data
systems, events, dashboards, forms, notifications, and integrations.

### Agent Tool Profile

Tools and capabilities that AI agents can request. Each tool must expose its
purpose, inputs, outputs, permissions, effects, failures, secrets, approval
rules, audit events, and guarantees.

### Systems Profile

Low-level systems software, kernels, drivers, runtimes, schedulers, memory
managers, filesystems, network stacks, embedded software, and performance
critical libraries.

### Compiler Profile

Language definitions, compiler passes, checkers, diagnostics, transformations,
lowering rules, optimizers, renderers, build steps, and self-hosting logic.

### Training Profile

Example programs, interview transcripts, structured specs, semantic IR,
patches, traces, explanations, invalid examples, diagnostics, round-trip tests,
and fine-tuning/evaluation data.

## Core Vocabulary

The core vocabulary should remain small and regular so humans and LLMs can learn
it from prompts and examples.

Initial core concepts:

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

These concepts should work across profiles. An API endpoint, AI-agent tool,
compiler pass, and device-driver operation should all expose actions, inputs,
outputs, rules, effects, failures, guarantees, and traces.

## Agent Protocol

The AI Agent protocol is part of the toolchain.

### Responsibilities

The AI Agent should:

- interview the human
- identify missing semantic information
- ask focused clarification questions
- produce AIL-Spec
- elaborate AIL-Spec into AIL-Core
- render AIL-Core back into AIL-Spec and plain English
- propose patches instead of silent rewrites
- explain diagnostics
- debug traces interactively
- generate examples and tests
- preserve provenance for every inferred fact

### Required Interview Coverage

Before an application, tool, or systems behavior is accepted, the agent should
attempt to capture:

- who uses it
- what data it stores
- what actions are possible
- what inputs each action needs
- what outputs each action produces
- what must be true before actions run
- what can fail
- what happens on each failure
- what data is secret
- what external systems can be called
- what data may be read or changed
- what approvals are required
- what guarantees must always hold
- what views or interfaces humans need
- what traces should be explainable

### Patch Discipline

Future changes should be represented as patches:

```text
Change request:
  Allow managers to reopen closed tickets.

Proposed patch:
  Add action "Reopen ticket".
  Add rule "Only managers can reopen tickets".
  Add status transition Closed -> Open.
  Add audit event "Ticket reopened".

Expected effects:
  Ticket status can change from Closed to Open.
  A reopen event is recorded.
  Non-manager users still cannot reopen tickets.
```

The checker validates the patch before it is accepted.

## LLM-Friendly Language Contract

AIL should be prompt-operable. A capable LLM should be able to use the language
from a system prompt, compact schema, and examples.

Every language feature should have:

- one short English rule
- one canonical structured form
- one valid example
- one invalid example
- one diagnostic example
- one round-trip expectation
- one no-code rendering expectation
- one trace/debugging expectation

LLMs should be expected to use small, regular forms. Irregular syntax and hidden
special cases should be treated as language design failures.

## Human-Reviewable Agent Tools

AIL should make AI-agent tools explicit and auditable.

An agent tool must declare:

- purpose
- allowed use
- inputs
- outputs
- rules
- permissions
- effects
- secrets
- approvals
- failures
- guarantees
- audit trace

Example:

```text
Tool: Refund customer payment

The agent may use this tool when:
- the order exists
- the payment was captured
- the refund amount is not greater than the captured amount

The tool needs:
- order id
- refund amount
- reason

The tool can:
- read the order
- read the payment record
- call the payment provider
- write a refund record

The tool must not:
- reveal the payment token
- refund more than the captured amount
- run without manager approval when the refund is over USD 500

If the payment provider rejects the refund:
- the tool records the failure
- the customer is not notified automatically
- a human review task is created
```

The runtime enforces the declared capability. The LLM can request the tool, but
the runtime checks whether the request is permitted.

## Semantic Equivalence

Semantic equivalence should be defined in layers.

### Strong Equivalence

Two artifacts are strongly equivalent when they normalize to the same AIL-Core
graph, allowing stable ordering normalization and approved alias normalization.

### Behavioral Equivalence

Two artifacts are behaviorally equivalent when they produce the same observable
behavior, diagnostics, permissions, effects, failures, guarantees, and traces
for the same inputs.

### Explanation Equivalence

Two English projections are explanation-equivalent when a human reviewer and
the automated semantic checks agree that the same behavior, rules, permissions,
effects, failures, and guarantees are communicated.

Embeddings may help detect drift in explanation equivalence, but they cannot
replace graph or behavioral checks.

## Debugging Model

AIL debugging should be interactive and semantic.

The runtime should produce traces that answer:

- which action ran
- who or what triggered it
- which rules allowed or rejected it
- which data was read
- which data changed
- which external systems were called
- which branch was taken
- which failure occurred
- which guarantee failed
- which lower-level operation corresponds to the behavior
- which human-confirmed spec caused the behavior

The AI Agent should use traces to answer questions in plain English.

Example:

```text
Human: Why did this ticket not close?

Agent:
- The Close ticket action ran.
- It required the ticket status to be Open.
- The actual ticket status was Pending.
- The action stopped before changing the ticket.
- This rule came from "Only open tickets can be closed."
```

Systems debugging should use the same model:

```text
Human: Why is this generated compiler binary allocating here?

Agent:
- The diagnostics list escapes the current region.
- It escapes because Render diagnostics returns the list to its caller.
- The compiler allocated the list on the heap to keep it alive.
- To avoid the allocation, the pass can stream diagnostics instead.
```

## Self-Sovereign Toolchain Principle

AIL may use Rust, C, C++, Python, JavaScript, TypeScript, Go, LLVM, Wasm, or
other existing systems as bootstrap scaffolding.

After the first viable compiler generations, no required part of the AIL
toolchain should depend on a legacy language as its source of truth.

The long-term required AIL-defined components include:

- language specification
- parser and elaborator rules
- checker rules
- diagnostic rules
- semantic IR schema
- renderers
- no-code projections
- debugger
- agent protocol and prompts
- compiler passes
- optimizer rules
- lowering rules
- runtime primitives
- standard library
- package system
- build system
- conformance suite

Legacy-language artifacts may remain as historical bootstrap artifacts,
optional backends, or interoperability targets. They must not be required for
AIL to define, explain, evolve, or rebuild itself.

## Bootstrap Stages

### Stage 0: Bootstrap Prototype

Use the current Rust prototype to explore parsing, checking, semantic graph
construction, views, traces, and execution.

### Stage 1: AIL Foundation Specs

Write the AIL foundation, vocabulary, structured specification, semantic IR,
agent protocol, no-code projection, debugging, and self-hosting specs.

### Stage 2: AIL-Defined Compiler Rules

Represent parser rules, checker rules, diagnostics, renderers, examples,
round-trip rules, and lowering obligations in AIL-Meta.

### Stage 3: Generated AIL Compiler

Use the bootstrap compiler to compile the AIL-defined compiler rules into a new
compiler.

### Stage 4: Self-Hosted Fixed Point

Compiler N compiles the AIL toolchain spec into Compiler N+1. Compiler N+1
compiles the same spec into Compiler N+2. The outputs are equivalent under the
defined fixed-point check.

### Stage 5: Legacy Independence

AIL can rebuild its required compiler, runtime, standard library, agent tooling,
and build system from AIL sources.

## Training Corpus Strategy

AIL should be developed with a training and evaluation corpus from the start.

Each accepted language feature should include paired artifacts:

- vague human request
- agent interview transcript
- structured English spec
- semantic IR
- no-code view model
- valid examples
- invalid examples
- diagnostics
- runtime traces
- debugging conversations
- patch examples
- round-trip examples
- conformance expectations

This corpus supports:

- system prompts
- few-shot examples
- fine-tuning
- model evaluation
- regression tests
- semantic equivalence tests
- human-readability tests
- compiler conformance tests

## Language Evolution Protocol

AIL should evolve through explicit proposals.

Each proposal must include:

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

The proposal is accepted only when it preserves the non-negotiable invariants.

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

The foundation spec intentionally separates stable invariants from versioned
decisions and experiments.

Stable invariants define what AIL must always preserve.

Versioned decisions define the current shape of AIL-Spec, AIL-Core, AIL-Flow,
AIL-System, AIL-Meta, and the bootstrap toolchain.

Experiments may explore alternative English forms, agent prompts, visual
renderers, equivalence scoring, and debugging interactions without becoming
core language rules.

## Initial Specification Set

The first complete AIL specification suite should include:

- AIL vision and invariants
- AIL language architecture
- AIL-Spec structured English
- AIL-Core semantic IR
- AIL-Flow no-code projections
- AIL-Agent protocol
- AIL tool capability model
- AIL type and value model
- AIL permission, effect, and capability model
- AIL failure and guarantee model
- AIL debugging and trace model
- AIL-System low-level profile
- AIL-Meta compiler-definition profile
- AIL round-trip and equivalence model
- AIL training corpus and conformance model
- AIL bootstrap and self-hosting plan
- AIL language evolution protocol

## Open Design Pressure

AIL must balance three forces:

- English-like enough for non-engineers and LLMs.
- Formal enough for deterministic checking and compilation.
- Low-level enough to define compilers, runtimes, kernels, and optimized
  binaries.

The first implementation should not try to solve every profile at once. It
should validate the semantic architecture with small vertical slices that prove
round-tripping, human review, no-code rendering, checking, tracing, and
compilation can all share one source of truth.
