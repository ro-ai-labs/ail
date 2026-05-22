# AIL-Agent Protocol

## Purpose

The AIL-Agent protocol defines how an AI Agent participates in authoring,
reviewing, patching, explaining, and debugging AIL programs.

The AI Agent is part of the toolchain but not part of the trusted compiler core.

## Agent Responsibilities

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

## Interview Loop

The interview loop starts from an English request. The agent extracts candidate
actors, things, actions, rules, failures, secrets, external systems, views, and
guarantees. It asks one focused question at a time when a missing detail affects
checking, safety, or user-visible behavior.

## Required Coverage

Before a behavior is accepted, the agent should attempt to capture:

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

## Patch Discipline

Patch Discipline requires future changes to be represented as explicit patches
with expected effects. An agent must not silently rewrite accepted behavior.

Each proposed patch includes:

- human request
- affected nodes and edges
- structured English explanation
- expected behavior change
- expected permission, effect, failure, and guarantee changes
- provenance
- validation result

## Conversion Tasks

The agent may convert:

- English request to draft AIL-Spec
- AIL-Spec to candidate AIL-Core
- diagnostics to explanation questions
- traces to debugging explanations
- no-code edits to graph patches
- examples to training corpus entries

Each conversion is untrusted until checked.

## Prompt Compatibility Standard

Prompt Compatibility means a capable LLM should be able to use a language
feature from a compact prompt, schema, and examples.

Every feature should have:

- one short English rule
- one canonical structured form
- one valid example
- one invalid example
- one diagnostic example
- one round-trip expectation
- one no-code rendering expectation
- one trace/debugging expectation

## Calibration Examples

Calibration examples must include accepted specs, rejected specs, diagnostic
repairs, patches, traces, and explanations. They train the agent to ask for
missing semantics instead of guessing.

## Trust Boundary

The agent is untrusted. It can request tools, draft specs, propose patches, and
explain results, but the trusted checker accepts or rejects deterministic
artifacts.

## Failure Modes

The protocol must detect and surface:

- hallucinated actions or fields
- unconfirmed permission changes
- secret disclosure
- hidden external calls
- incomplete failure handling
- projection drift
- trace explanations that do not match runtime events
