# AIL Failures, Guarantees, And Traces

## Purpose

AIL treats failures, guarantees, and traces as language-level semantics. A
program is incomplete if it can fail in an important way without naming the
failure, handling it, and making the result explainable.

## Declared Failures

Failures are named outcomes such as `NotFound`, `PermissionDenied`,
`ProviderRejected`, `InvalidInput`, `GuaranteeFailed`, or `LoweringFailed`.

Each failure declares:

- trigger condition
- affected action or tool
- data that remains unchanged
- compensation behavior
- user or caller response
- trace event
- related guarantees

## Failure Handling

Failure handling may stop the action, run compensation, retry, request approval,
emit an event, create a review task, return an error response, or choose an
alternate branch. The chosen handling must be explicit.

## Compensation

Compensation describes how the program repairs or limits partial effects. It
can undo a write, create a follow-up task, publish a correction event, mark a
record for review, or preserve a safe intermediate state.

## Guarantees

A guarantee is a condition the program promises after successful execution or
after a named failure path. Guarantees may cover data consistency, permissions,
secret protection, external effects, ordering, trace coverage, memory safety,
or lowering correctness.

## Trace Events

Trace Events record semantic execution:

- action or tool start
- input validation
- rule evaluation
- permission check
- approval check
- data read
- data write
- external call
- branch selection
- failure occurrence
- compensation
- guarantee check
- low-level lowering or allocation decision

## Interactive Debugging

Interactive Debugging uses traces to answer questions in plain English.

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

## Human Diagnosis Requirements

Human Diagnosis requires each important runtime outcome to identify the action,
trigger, rule, data, branch, failure, guarantee, and source paragraph that
caused it. A diagnosis must not invent facts that are absent from the trace.

## Systems-Level Debugging

Systems-level debugging uses the same model for low-level behavior.

Example:

```text
Human: Why is this generated compiler binary allocating here?

Agent:
- The diagnostics list escapes the current region.
- It escapes because Render diagnostics returns the list to its caller.
- The compiler allocated the list on the heap to keep it alive.
- To avoid the allocation, the pass can stream diagnostics instead.
```
