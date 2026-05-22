# AIL-Flow No-Code Views

## Purpose

AIL-Flow is the no-code projection of AIL-Core. It lets humans inspect and edit
applications, tools, system components, rules, permissions, capabilities,
failures, traces, and low-level obligations without reading compiler syntax.

## View Types

Initial view types are:

- Application Map
- Action Cards
- Data Tables
- Rule Lists
- Permission Views
- Failure Maps
- Trace Views
- Tool Capability Views
- System Component Views
- Lowering Views
- Diagnostic Views

## Application Map

The application map shows actors, things, actions, events, external systems,
tools, system components, and views. It is a deterministic projection of
AIL-Core and must render the same graph the same way apart from layout
preferences.

## Action Cards

Action Cards show one action at a time: trigger, inputs, requirements, reads,
writes, calls, failures, approvals, guarantees, and traces.

Editing an action card creates a graph patch. The patch is checked before it is
accepted.

## Data Tables

Data tables show things, fields, types, secrecy, ownership, persistence, and
visibility. They can propose field additions, type changes, visibility changes,
and migrations as graph patches.

## Rule Lists

Rule lists show preconditions, invariants, permission rules, approval rules,
and guarantee rules. Rules must show their provenance and the actions or tools
that depend on them.

## Permission Views

Permission Views show who or what may read, write, call, approve, disclose, or
own a resource. Secret flows and capability boundaries must be visible.

## System Component Views

System Component Views show low-level components, resources, required
capabilities, effects, guarantees, and traces. They make device or OS access
visible before lowering to target code.

## Failure Maps

Failure maps show named failures, triggering conditions, compensation, user
messages, retries, audit events, and affected guarantees.

## Trace Views

Trace Views show runtime execution in semantic terms: action entry, rule checks,
reads, writes, calls, branches, failures, approvals, guarantees, and low-level
obligations.

## Editing Through Views

No-code views do not perform opaque text edits. They produce graph patches
against AIL-Core. Each patch declares the nodes, edges, attributes, and
provenance it adds, changes, or removes.

## Validation Of View Patches

View patches are checked with the same authority as structured-English patches.
The checker validates type flow, permissions, effects, failures, guarantees,
secrets, approvals, trace obligations, profile rules, and round-trip
equivalence before acceptance.
