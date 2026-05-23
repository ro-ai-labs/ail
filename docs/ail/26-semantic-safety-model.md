# AIL Semantic Safety Model

## Purpose

AIL lets non-engineers author executable behavior through English, agent
assistance, and visual views. The safety model defines which operations require
confirmation, approval, expert review, capability grants, trace evidence, or
agent refusal before the checker accepts the artifact.

## Safety Classes

| Class | Examples | Required review |
| --- | --- | --- |
| Low | local validation, pure transformations, read-only non-secret views | normal human review |
| Medium | writes to application state, notifications, scheduled jobs | explicit confirmation |
| High | money movement, permission changes, secret access, external calls | approval plus audit trace |
| Expert | C interop, kernel/system operations, raw pointers, device access, unsafe memory | expert-mode capability plus conformance fixtures |
| Prohibited | hidden external calls, secret disclosure without reveal policy, bypassing checker | rejected |

## Confirmation Rules

The checker requires explicit confirmation for inferred rules that affect:

- permissions
- effects
- secrets
- money
- safety
- external calls
- C interop
- OS or device resources
- destructive UI actions
- backend lowering obligations

An AI Agent may ask for confirmation, but it may not mark a semantic change as
confirmed unless the artifact includes human confirmation provenance.

## Approval Rules

High-risk operations require approval declarations:

```text
The action requires approval:

- manager approval before refund over USD 500
```

Approval declarations must include:

- approver role or capability
- triggering condition
- action blocked until approval
- trace event for requested, granted, denied, and expired approval
- compensation when approval is denied after partial progress

## Expert-Mode Rules

Expert-mode capability is required for:

- `unsafe c interop`
- raw pointers
- pointer ownership transfer
- interrupt handlers
- device register access
- DMA
- custom allocators
- scheduler manipulation
- kernel/runtime self-hosting
- native backend changes

Expert-mode packages must include one accepted fixture and one rejected fixture
for the unsafe feature they introduce.

## Agent Refusal And Escalation

The agent must refuse to produce a deterministic artifact when the user request
requires semantics that are missing or unsafe. It should return questions or an
escalation instead.

Required refusals:

- user asks to reveal a secret without permission
- user asks to add an external call without target, permission, and failure
  mapping
- user asks to bypass approval
- user asks for C pointer behavior without ownership and release semantics
- user asks for kernel/device behavior without expert-mode capability

## UI Review Requirements

AIL-Flow and UI profile views must display:

- risk class
- affected permissions
- affected secrets
- external calls
- money movement
- destructive writes
- approval requirements
- trace events
- changed guarantees

High and expert class changes cannot be accepted from a collapsed summary
view. The reviewer must see the affected graph nodes and edges or canonical
structured English sections.

## Audit Trace

Every medium, high, and expert class operation records:

- actor
- action
- package hash
- AIL-Core hash
- safety class
- approval state
- external binding if any
- secret redaction status
- result or failure

Audit traces are semantic runtime artifacts and participate in behavioral
equivalence.

## Safety Diagnostics

Safety diagnostics are blocking when the safety class is high or expert and a
required confirmation, approval, capability, trace, or fixture is missing.

Example:

```text
AIL-SAFETY-SECRET-001 blocking
condition: inferred secret read has no human confirmation
repair: ask who may read the secret and add permission, redaction, and trace
```
