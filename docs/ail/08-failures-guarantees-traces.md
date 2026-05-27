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
lowering correctness, or temporal behavior. Application actions that claim
scheduler behavior for repeated work must also name a temporal policy so the
checker can distinguish deterministic repeated-action lowering from scheduler
declaration errors.

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
- loop entry, iteration, and exit
- recursive call entry and return
- failure occurrence
- compensation
- guarantee check
- async task spawn, await, cancellation, and join
- low-level lowering or allocation decision

## Control-Flow Failure Semantics

Loops and recursion can fail through:

- termination policy violation
- iteration budget exhaustion
- stack or continuation limit
- guarantee failure inside an iteration
- cancellation
- failure propagated from a called action or function

Profiles that require termination must reject unproven recursion or unbounded
loops before runtime. Profiles that allow divergence must still declare
cancellation and trace behavior.

## Async And Concurrent Failure Semantics

Concurrent actions declare spawn, await, cancellation, join, and compensation
behavior. If one task fails, the graph must say whether sibling tasks continue,
cancel, compensate, or join before returning. Concurrent writes require
ownership, lock, or deterministic join rules.

## External And C Interop Failures

External calls map provider errors, timeouts, rejected responses, C return
codes, `errno`, null pointers, callback failures, memory faults, and ABI
violations into declared AIL `Failure` nodes. Unmapped external failures are
blocking for safe profiles.

## Backend Lowering Failures

Backend failures such as unsupported effect, unsupported target type, missing
symbol, verifier mismatch, trace mapping loss, or native artifact validation
failure map into `LoweringFailed` subdiagnostics. A backend failure blocks
artifact emission but must preserve the checked AIL-Core artifact for review.

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
