# Vision and Principles

## Goal

EIGL is a language architecture for a future where much software is authored with AI assistance, but humans still need to understand, review, govern, and trust the software.

The goal is not to invent another Python-like syntax. The goal is to replace traditional source code as the primary artifact with an executable, explainable, visualizable semantic model.

EIGL should be:

```text
simple enough for non-programmers to read,
compact enough to feel closer to Python than Java,
structured enough for LLMs to edit safely,
precise enough for a compiler to verify,
static enough to compile to performant code,
and explicit enough to visualize and explain.
```

## Core thesis

```text
Programs should be written as requirements,
elaborated into readable intent,
compiled from semantic graphs,
and verified through permissions, effects, contracts, and failures.
```

EIGL is therefore not just a language syntax. It is a layered system:

```text
Requirement Surface Language  ->  Readable Intent Format
Readable Intent Format        ->  Executable Intent Graph
Executable Intent Graph       ->  Compiler/runtime IR
Compiler/runtime IR           ->  machine code / Wasm / bytecode / specialized runtime
```

## Design target

```text
As concise as Python at the requirement surface.
As explainable as structured English in the intermediate layer.
As safe as Rust in the compiler core.
As optimizable as a static IR in the backend.
```

## What humans should write

Humans should write compact requirement-like statements:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

The user should not need to explicitly write every type, permission, effect, failure path, allocation, dispatch rule, or graph edge.

## What the system should infer

The system may infer:

```text
types
subjects
state transitions
step inputs
step outputs
permissions
effects
failure cases
compensations
guarantees
visual layout
cost-relevant facts
```

However, inferred facts must be visible in RIF and must have provenance.

## Sparse at the top, explicit underneath

The primary principle is:

```text
Humans may omit details.
The compiler may infer details.
But every inferred detail must become visible in the intermediate representation.
```

This gives Python-like brevity without Python-like runtime ambiguity.

## Controlled English, not arbitrary English

EIGL should read like English, but it cannot be unrestricted English.

Unrestricted English is ambiguous. EIGL uses controlled English and domain dictionaries. Sentences are allowed to be short and natural, but every valid sentence must map to a formal semantic object.

Example valid sentence forms:

```text
A <Thing> has <Property>.
A <Thing> can be <State>.
<Action> changes <Thing>.
<Action> does <Step>.
If <Failure> happens, do <Recovery>.
After <Action>, <Guarantee> must be true.
```

## Semantic graph as source of truth

The canonical program is not raw text.

```text
program = typed semantic graph + permission graph + effect graph + view graph
```

Text, diagrams, tables, explanations, and traces are projections of the same semantic object.

## Human concepts

The human-facing language should expose only a small set of concepts:

```text
Thing
Rule
Action
Step
State
Event
View
```

Everything else is compiler machinery.

## Compiler concepts

The compiler-facing core uses:

```text
Type
Value
Operation
Region
Edge
Permission
Effect
Contract
View
```

Mapping:

```text
Thing   -> Type
Rule    -> Contract
Action  -> Operation with an effect/process region
Step    -> Operation or operation invocation
State   -> Type + state-machine region
Event   -> Type + emission effect
View    -> View query/projection
```

## Explainability rule

The system must be able to answer:

```text
What does this action do?
Why does this action need this permission?
What can fail?
What happens if it fails?
What state changes?
What external systems are called?
What secrets are touched?
What is the cost model?
Which requirements caused this behavior?
```

These answers must be generated from RIF/EIG-Core, not guessed from raw prose.

## Safety rule

Safe EIGL should aim to prevent:

```text
use-after-free
double-free
data races
unchecked null access
ignored declared failures
unauthorized external calls
secret leakage through forbidden paths
hidden unsafe code
mutation while another live operation is reading the same mutable resource
```

The human does not write these rules each time. They are inherited from policies such as `SafeDefaults`.

## Performance rule

The high-level language must not force slow execution.

The compiler should avoid mandatory:

```text
garbage collection
dynamic dispatch
runtime reflection
implicit boxing
stringly typed lookup
implicit exceptions
runtime type guessing
```

The compiler should support:

```text
static types
static dispatch
ownership-based memory management
region allocation
stack allocation
move semantics
copy elision
inlining
partial-order scheduling
SIMD/vectorization
native code
Wasm
workflow runtimes
accelerator backends
```

## LLM role

LLMs should help elaborate, explain, and edit semantic models. They should not be trusted as the final compiler authority.

Good LLM roles:

```text
convert loose requirements into RSL
suggest RIF expansions
explain RIF in plain English
propose graph patches
detect missing cases
suggest domain phrase meanings
generate tests from specs
```

Bad LLM roles:

```text
silently decide ambiguous semantics
bypass compiler checks
generate unchecked low-level code
invent effects or permissions without provenance
```

## Long-term self-hosting goal

Eventually:

```text
EIGL is specified in EIGL.
The EIGL compiler is generated from that specification.
The generated compiler can compile the compiler specification itself.
```

This is the self-hosting target.
