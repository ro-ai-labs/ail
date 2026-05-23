# AIL Language Architecture

## Pipeline

AIL uses a staged pipeline:

```text
AIL-English conversation
  -> AI-assisted interview
  -> AIL-Spec structured English
  -> AIL-Core canonical semantic graph
  -> checker
  -> executable artifact
  -> projections, diagnostics, traces, and explanations
```

English conversation is not compiled directly. It is an input to the interview
and clarification process. The compiler accepts only deterministic artifacts.

## Accepted Program Artifact

The accepted program artifact is AIL-Core plus validated metadata. The metadata
records provenance, package identity, version, profile, source spans or
paragraph IDs, trust decisions, approvals, and conformance expectations.

AIL-Core is the source of truth for accepted behavior. If a projection and
AIL-Core disagree, AIL-Core wins and the projection is a compiler bug.

## Projection Model

Every human-facing surface is a projection:

- AIL-Spec is structured English for review and authoring.
- AIL-Flow is a no-code view model for cards, flows, rules, permissions,
  failures, and traces.
- Diagnostics explain checker failures and missing semantic information.
- Runtime traces explain what happened and why.
- Lower-level explanations connect high-level intent to generated code,
  runtime effects, memory behavior, and backend obligations.

Projections may create edits only as validated patches against AIL-Core. They
must not silently rewrite the accepted program.

## Trusted And Untrusted Components

The trusted checker validates AIL-Core, metadata, imports, package boundaries,
types, permissions, effects, failures, guarantees, trace obligations, and
profile-specific rules.

The AI Agent, LLM calls, prompt templates, no-code editors, formatters, and
renderers are untrusted. They may propose artifacts or patches, but they do not
decide acceptance.

## Profiles Over One Semantic Substrate

AIL profiles specialize the same graph model rather than creating separate
languages:

- Application programs add routes, forms, background jobs, collections, events,
  integrations, and user-facing views.
- Agent tools add purpose, capability, approval, audit, secret, and runtime
  enforcement contracts.
- Systems programs add ownership, regions, layout, scheduling, device access,
  and lowering obligations.
- Compiler programs add parser rules, checker rules, diagnostics, renderers,
  passes, optimizers, and fixed-point checks.
- Training programs add paired examples, invalid examples, diagnostics, traces,
  and evaluation cases.

## Compiler Boundary

The compiler boundary begins at validated AIL-Core. Source conversation,
draft AIL-Spec, proposed patches, generated examples, and no-code edits are
outside the boundary until they normalize into checked AIL-Core.

The checker must reject:

- unresolved questions
- missing required inputs or outputs
- unknown references
- ambiguous action targets
- invalid type, permission, effect, or failure flow
- secret leaks
- unmet approval rules
- profile obligations that cannot be explained or lowered

## Runtime Boundary

The runtime executes accepted artifacts and enforces declared capabilities. It
must produce trace events for action entry, rule checks, reads, writes, calls,
branches, failures, approvals, guarantees, and low-level obligations that matter
to the selected profile.

The runtime may optimize, cache, inline, or lower behavior only when the
observable behavior, permissions, effects, failures, guarantees, and traces
remain equivalent under the active conformance rules.

## Bootstrap Boundary

Stage-0 implementation code may host the first parser, checker, VM, and native
backend while the language matures. The accepted language surface remains
AIL-Spec, AIL-Core, AIL bytecode, native executable artifacts, manifests,
reports, and deterministic projections.
