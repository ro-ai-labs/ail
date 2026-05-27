# AIL Execution Semantics

## Purpose

This document defines how checked AIL-Core executes. Surface English,
AIL-Flow, prompts, and friendly explanations may describe execution, but only
the normalized AIL-Core graph accepted by the checker is executable.

## Executable Core

The minimal executable core is the subset of AIL-Core that contains:

- `Action` and `Function` declarations
- `Call` edges from actions or functions to actions, functions, tools, or
  external bindings
- `Return` values
- `Branch` nodes with exactly one condition and two or more ordered outcomes
- `Loop`, `ForEach`, and `While` nodes
- `Match` nodes over finite variants or pattern constructors
- `Let`, `Assign`, `Read`, and `Write` effects over scoped values or state
- `Emit` trace events
- `Fail` and `HandleFailure` edges
- `GuaranteeCheck` nodes
- `ExternalCall` nodes for tools, C interop, OS calls, and package APIs
- `Allocate`, `Borrow`, `Release`, and `Move` nodes for system profiles

Every higher-level profile lowers into this subset before bytecode, VM, native
backend, or system lowering.

## Evaluation Order

Execution is deterministic.

1. The runtime loads a checked AIL package and verifies the package hash,
   schema version, profile, imports, and capability grants.
2. The runtime resolves the selected action or function by stable ID.
3. Inputs are decoded, type checked, and bound to immutable input values.
4. Required rules are evaluated in canonical edge order.
5. Steps execute in declared order unless a branch, loop, match, failure, or
   return changes control flow.
6. Effects execute only after their permission, capability, secrecy, and
   profile rules pass.
7. Guarantees run at their declared boundary: entry, before effect, after
   effect, exit, compensation, or trace finalization.
8. The runtime emits a trace event for action entry, rule check, branch
   decision, loop iteration, call, external call, failure, compensation,
   guarantee check, and action exit.

Canonical edge order is the stable serialization order defined in
`18-ail-core-schema.md`.

The AIL VM's initial executable control-flow subset includes `LABEL`,
`BRANCH_FIELD_EQUALS`, and `JUMP`. `BRANCH_FIELD_EQUALS` compares a runtime
state field with a literal value, records whether the branch label was taken or
skipped, and transfers control to a verified label when it is taken. `JUMP`
records the target label and transfers control unconditionally. The bytecode
verifier rejects missing branch or jump labels before execution.

The initial action-invocation subset includes `CALL_ACTION`. It records the
target action, executes the target with the caller's current runtime state,
merges the callee trace into the caller trace, and resumes the caller with the
callee's final state when the callee succeeds. If the callee fails, the caller
returns the same failure and merged trace. The bytecode verifier rejects calls
to unknown actions before execution.
Structured AIL-Spec bullets of the form `the system calls <ActionName>` lower
to Core `calls` edges from the caller action to the callee action, then to
`CALL_ACTION` bytecode. The source-level interpreter follows the same failure
propagation and trace merge behavior.

The initial integer state-mutation subset includes `ADD_INT_FIELD`. It reads a
runtime state field as an integer, adds the signed integer `delta`, writes the
result back to the same field, and records the new value in the trace. The
bytecode verifier rejects non-integer `delta` literals before execution, and
the VM rejects missing fields or non-integer field values at runtime. Together
with `LABEL`, `BRANCH_FIELD_EQUALS`, and `JUMP`, this gives AIL-Bytecode a
finite loop counter primitive without introducing host-language execution.

## Calls And Returns

An `Action` may call another `Action`, a `Function`, an `AgentTool`, an
external binding, or a profile lowering primitive. A `Function` may call only
pure functions unless it is explicitly annotated with effects and capability
requirements.

Call semantics:

- arguments are evaluated left to right in canonical parameter order
- mutable references require an exclusive borrow edge
- owned values move unless the type is copyable
- return values are bound through `Return` nodes or named output edges
- recursive calls are allowed only in profiles that permit unbounded execution
  or declare a termination policy

## Branching And Matching

A `Branch` evaluates one condition and selects the first matching outcome in
canonical outcome order. If no outcome matches and no `else` outcome exists,
the checker rejects the graph with `AIL-CONTROL-001`.

A `Match` evaluates a value and selects exactly one pattern. Exhaustiveness is
required for finite variants such as `Option<T>`, `Result<T, E>`, and declared
`State<...>` values. Non-exhaustive matches are rejected with
`AIL-CONTROL-002`.

## Loops

AIL supports three loop forms:

- `ForEach`: iterates over a finite collection in stable collection order.
- `While`: repeats while a checked condition remains true.
- `Loop`: repeats until `Break`, `Return`, or `Fail`.

Each loop emits `LoopEntered`, `LoopIteration`, and `LoopExited` trace events
with the loop node ID and iteration count. Profiles may declare a termination
policy:

- `must_terminate`: checker requires a structural decrease, finite collection,
  bounded iteration, or explicit proof artifact.
- `may_diverge`: accepted for Turing-complete general execution and system
  loops, but the runtime must preserve cancellation and trace visibility.
- `bounded`: runtime enforces an iteration budget declared by the package.

## Recursion

Direct and mutual recursion are valid in the Turing Core. Each recursive
function declares:

- call graph edge
- input and output types
- termination policy
- stack or continuation strategy
- trace event name for recursive entry

An explicit stack bound is written in the function body as:

```ail
- the function has a maximum recursion depth of 64
```

Lowering records this as a `TerminationBound` node connected by
`has_termination_bound`. The value must include a numeric bound and mention a
recursion, stack, or termination limit.

A well-founded measure is written in the function body as:

```ail
- the function has a well-founded termination measure n that decreases to 0 on every recursive call
```

Lowering records this as a `TerminationMeasure` node connected by
`has_termination_measure`. The value must state that the measure decreases and
must name a visible lower bound or well-founded ordering.

Profiles that require termination reject unproven recursion with
`AIL-CONTROL-003`.

## State Mutation

State mutation is explicit. A write is accepted only when the graph contains:

- writer action or function
- target value, field, resource, or external state
- required permission or capability
- effect class
- failure behavior
- trace event
- guarantee boundary when a guarantee depends on the mutation

Writes are sequenced in canonical step order. Concurrent writes require the
concurrency rules in this document and the ownership rules in
`07-types-values-effects.md`.

## Failures And Compensation

`Fail` raises a declared `Failure`. The runtime searches the current action,
caller action, package policy, and profile policy for a `HandleFailure` edge.
If no handler exists and the failure is blocking, execution stops and the
trace records `UnhandledFailure`.

A compensation action runs after the failure trace and before user-visible
output. Compensation may not hide the original failure. It emits its own trace
events and guarantee checks.

## Tool Calls And External Calls

Tool calls, OS calls, C interop calls, network calls, file calls, process
calls, random calls, and clock calls are `ExternalCall` nodes. An external
call must declare:

- binding target
- input and output types
- effect class
- permission or capability
- sandbox or ABI contract
- failure mapping into AIL `Failure`
- secret redaction policy
- trace event

The AI Agent may propose an external call, but the checker accepts only a
declared binding with matching permissions.

## Concurrency Boundaries

Concurrent execution is opt-in. An AIL-Core graph may declare:

- `Task` nodes
- `Await` nodes
- channels or messages
- locks and guards
- ownership transfer edges
- scheduler constraints
- cancellation points

Concurrent actions must preserve deterministic trace ordering through logical
sequence numbers. If a profile cannot preserve deterministic behavior, the
checker must reject the graph or require a behavioral-equivalence proof.

## Turing Completeness

AIL-Core is Turing complete through the Turing Core subset:

- unbounded recursion or unbounded `While`
- conditional branching
- construction and inspection of values
- function or action invocation
- readable and writable state

Proof sketch: encode a two-counter Minsky machine. Counters are `Int` values.
Increment and decrement are `Assign` operations. Zero tests are `Branch`
conditions. Instruction dispatch is a recursive `Function` over a program
counter. Because a two-counter Minsky machine is Turing complete and the
Turing Core can encode its transition function, checked AIL-Core can encode a
known Turing-complete model.

Non-engineer friendliness does not weaken this formal requirement. Friendly
English and visual blocks are projections of this executable semantic core.

## Accepted Example: Recursive Factorial

Canonical AIL-Spec:

```text
Function: factorial.

The function needs:

- n: Int

The function produces:

- result: Int

When factorial runs:

- if n is 0, the function returns 1
- otherwise the function calls factorial with n minus 1
- the function returns n multiplied by the recursive result
- the function records a trace event named FactorialCalled
```

AIL-Core rendering:

```text
node Function factorial [label=factorial]
node Input factorial.n : Int
node Output factorial.result : Int
node Branch factorial.n is 0 [condition=n is 0]
node Call factorial.factorial with n minus 1 [target=factorial]
node Return factorial.1 [value=1]
node Return factorial.n multiplied by the recursive result [value=n multiplied by the recursive result]
edge contains Function:factorial -> Branch:factorial.n is 0
edge calls Function:factorial -> Call:factorial.factorial with n minus 1
edge records_trace Function:factorial -> Trace:FactorialCalled
```

Current bootstrap VM bytecode trace for this function surface records the
checked structure that will drive full Turing Core execution:

```text
function factorial started
function input n:Int
function output result:Int
function branch n is 0
function call factorial
function return 1
function return n multiplied by the recursive result
trace FactorialCalled
```

Target recursive execution trace for `factorial(3)` after recursive arithmetic
semantics are enabled:

```text
trace FunctionEntered factorial n=3
trace BranchSelected factorial.base_case outcome=otherwise
trace FunctionEntered factorial n=2
trace FunctionEntered factorial n=1
trace FunctionEntered factorial n=0
trace Return factorial.return_one value=1
trace Return factorial.return_product value=1
trace Return factorial.return_product value=2
trace Return factorial.return_product value=6
```

AIL-Flow rendering:

```text
Action Card: factorial
  Inputs: n: Int
  Branch: n is 0 -> return 1
  Otherwise: call factorial(n - 1)
  Return: n * recursive result
  Trace: FactorialCalled
```

## Rejected Example: Required Termination Without Proof

```text
Function: wait forever.

When wait forever runs:

- while true remains true, the function repeats
- the profile requires termination
```

Diagnostic:

```text
AIL-CONTROL-003 blocking
condition: unbounded loop in must_terminate profile has no structural decrease
repair: add a bounded iteration policy, prove structural decrease, or change
the profile to may_diverge
```

## Additional Turing Core Fixtures

The conformance suite includes these executable patterns:

- recursive factorial
- recursive countdown with an explicit well-founded termination measure
- `List.map`, `List.filter`, and `List.reduce`
- stateful counter with `Read`, `Assign`, and `Write`
- event loop with `Loop`, `Await`, `Branch`, `Emit`, and cancellation
- compiler pass over an AIL-Core graph using graph traversal and graph patch
  construction
