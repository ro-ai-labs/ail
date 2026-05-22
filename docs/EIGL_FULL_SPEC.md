<!-- Source: README.md -->

# EIGL Specification Pack

**EIGL** stands for **Executable Intent Graph Language**.

This package captures the design direction discussed so far: a requirements-first, graph-native, compiled programming language intended to be understandable by humans with little or no software-development background, useful for LLM-assisted authoring, visualizable as meaningful diagrams, and compilable into safe, performant machine code.

The current design target is:

```text
As concise as Python at the requirement surface.
As explainable as structured English in the intermediate layer.
As safe as Rust in the compiler core.
As optimizable as a static IR in the backend.
```

The central architectural idea is that **the human-facing text is not the final source of truth**. Humans write compact controlled requirements. Those requirements are elaborated into a still-human-readable intermediate format, which is then lowered into a typed executable semantic graph and finally into compiler/runtime IR.

```text
RSL       Requirement Surface Language
          Concise controlled requirements written by humans.

RIF       Readable Intent Format
          Normalized, explicit, explainable human-readable meaning.

EIG-Core  Executable Intent Graph
          Canonical typed semantic graph with permissions, effects, contracts, failures, and provenance.

EIG-IR    Compiler/runtime intermediate representation
          Lower-level representation for native code, Wasm, bytecode, workflow runtimes, or accelerators.

EIG-Meta  Language/compiler specification layer
          The layer used to describe the language, compiler passes, diagnostics, visualizations, and lowering rules.
```

## Files

Read in this order:

1. [`00-codex-handoff.md`](00-codex-handoff.md) — ready-to-paste handoff prompt for Codex or another coding agent.
2. [`01-vision-and-principles.md`](01-vision-and-principles.md) — core goals and design principles.
3. [`02-language-architecture.md`](02-language-architecture.md) — full pipeline from requirements to executable code.
4. [`03-rsl-requirement-surface-language.md`](03-rsl-requirement-surface-language.md) — the compact human-facing language.
5. [`04-rif-readable-intent-format.md`](04-rif-readable-intent-format.md) — the explicit intermediate format.
6. [`05-eig-core-semantic-graph.md`](05-eig-core-semantic-graph.md) — canonical typed graph model.
7. [`06-safety-permissions-effects.md`](06-safety-permissions-effects.md) — Rust-like safety model in human terms.
8. [`07-visualization-and-views.md`](07-visualization-and-views.md) — meaningful visual projections of programs.
9. [`08-compiler-generation.md`](08-compiler-generation.md) — how to generate the compiler from language specs.
10. [`09-self-hosting-bootstrap.md`](09-self-hosting-bootstrap.md) — path toward writing the compiler in the language itself.
11. [`10-prototype-roadmap.md`](10-prototype-roadmap.md) — practical implementation plan.
12. [`11-examples.md`](11-examples.md) — end-to-end examples.
13. [`12-open-questions.md`](12-open-questions.md) — unresolved design choices.
14. [`EIGL_FULL_SPEC.md`](EIGL_FULL_SPEC.md) — all files concatenated into one long document.

## Intended use

This is not yet a finalized language standard. It is a structured design package that can be given to Codex to begin implementing a prototype.

A sensible first prototype should **not** attempt the full natural-language front end. It should start with:

```text
RIF parser
EIG-Core graph schema
basic type checking
basic permission/effect/failure checking
flow/failure/permission view generation
small bytecode interpreter or Wasm backend
```

Only after that should the compact RSL layer and compiler-generation layer be implemented.



---

<!-- Source: 00-codex-handoff.md -->

# Codex Handoff: EIGL Prototype

You are implementing an early prototype of **EIGL: Executable Intent Graph Language**.

This is a requirements-first, graph-native compiled language. Humans write compact controlled requirements. The system elaborates them into a readable normalized intermediate format called RIF. RIF lowers into EIG-Core, a typed executable semantic graph. EIG-Core is checked for types, permissions, effects, failures, and contracts, then lowered to an executable form.

## Do not implement everything at once

The first prototype should target a narrow subset:

```text
1. Parse explicit RIF documents.
2. Build EIG-Core graph objects.
3. Validate references, types, permissions, effects, and failures.
4. Generate readable explanations from RIF/EIG-Core.
5. Generate simple graph visualizations.
6. Execute or simulate simple process graphs.
```

The first prototype should **not** attempt unrestricted natural language. RSL should come later.

## Recommended repository layout

```text
eigl/
  README.md
  pyproject.toml or Cargo.toml
  docs/
    spec/
      copy these markdown files here
  src/
    eigl/
      __init__.py
      rif_parser.py
      rif_model.py
      core_model.py
      graph_builder.py
      checker.py
      diagnostics.py
      explanations.py
      views.py
      interpreter.py
      examples.py
  examples/
    confirm_order.rif.md
    password_reset.rif.md
    bad_parallel_update.rif.md
  tests/
    test_rif_parser.py
    test_graph_builder.py
    test_checker.py
    test_explanations.py
    test_views.py
    test_interpreter.py
```

Python is acceptable for the first prototype because the goal is language exploration, not final runtime performance. A later implementation can move the trusted kernel and backend to Rust, Zig, C++, OCaml, or a custom self-hosted compiler.

## Minimum viable data model

Implement these core classes or equivalent structures:

```text
Program
Module
Intent
Thing
Field
StateSet
Operation
Step
FailureCase
Guarantee
Permission
Effect
Diagnostic
Graph
Node
Edge
View
```

The core graph should model:

```text
nodes:
  thing
  intent
  operation
  step
  state
  value
  failure
  guarantee
  permission
  effect

edges:
  contains
  has_field
  has_state
  reads
  changes
  calls
  produces
  requires
  ensures
  may_fail
  handles_failure
  compensates
  transitions_to
```

## Minimum viable RIF syntax

Start with indentation-based RIF, for example:

```text
intent ConfirmOrder

subject:
  order: Order

requires:
  order.status is Draft
  order.items.count > 0

steps:
  1. Reserve inventory
     call: Inventory.reserve(order.items)
     output: reservation: InventoryReservation
     changes: Inventory
     compensation: Inventory.release(reservation)

  2. Capture payment
     call: PaymentGateway.capture(order.payment_method, order.total)
     output: payment: PaymentCapture
     changes: PaymentLedger
     external call: PaymentGateway
     may fail with: PaymentFailed

  3. Create shipment request
     call: Shipping.create_request(order, reservation)
     output: shipment: ShipmentRequest
     changes: ShipmentQueue
     external call: Shipping
     may fail with: ShipmentFailed

  4. Mark order confirmed
     set: order.status = Confirmed
     changes: order.status

failure behavior:
  if payment capture fails:
    release reservation
    stop with PaymentFailed

  if shipment creation fails:
    set order.status = PaidAwaitingFulfillmentRetry
    stop with FulfillmentRetryNeeded

guarantees:
  if this intent succeeds:
    order.status is Confirmed
```

The parser does not need to support all wording immediately. It may parse this as structured sections and preserve unknown lines as semantic text for later elaboration.

## Checks to implement first

Implement these compiler checks:

1. Every step reference resolves to a known step or operation.
2. Every `call:` has a target operation name.
3. Every step output has a type.
4. Every declared failure from a step is handled, returned, or explicitly ignored.
5. Every `changes:` target creates a `Change` permission requirement.
6. Every `reads:` or input dependency creates a `Read` permission requirement.
7. Two unordered steps may not both change the same target.
8. A secret value may not flow to logs or unauthorized external calls.
9. A guarantee must refer to known subjects, fields, states, or outputs.
10. A compensation may only refer to values that are available on that failure path.

## Views to implement first

Generate plain-text views first. SVG/Graphviz can come later.

### Flow view

```text
[Draft Order]
    ↓
[Reserve inventory]
    ↓
[Capture payment]
    ↓
[Create shipment request]
    ↓
[Mark order confirmed]
    ↓
[Confirmed Order]
```

### Failure view

```text
Capture payment fails
    ↓
Release inventory reservation
    ↓
Stop with PaymentFailed
```

### Permission view

```text
ConfirmOrder
  reads:
    order.status
    order.items
    order.total
    order.payment_method

  changes:
    Inventory
    PaymentLedger
    ShipmentQueue
    order.status
```

## Prototype acceptance criteria

The prototype is useful when it can:

```text
1. Load examples/confirm_order.rif.md.
2. Produce an EIG-Core graph object.
3. Emit a JSON serialization of the graph.
4. Run all safety checks and report no errors for the valid example.
5. Generate flow, failure, permission, and explanation views.
6. Load examples/bad_parallel_update.rif.md.
7. Report a clear diagnostic for conflicting change permissions.
```

## Important design rule

Do not let the human-facing language become the compiler truth. The accepted source of truth should be RIF/EIG-Core.

```text
RSL is pleasant and sparse.
RIF is explicit and reviewable.
EIG-Core is canonical and checkable.
EIG-IR is executable.
```



---

<!-- Source: 01-vision-and-principles.md -->

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



---

<!-- Source: 02-language-architecture.md -->

# Language Architecture

## Overview

EIGL is a layered language system.

```text
RSL        Requirement Surface Language
           Concise controlled requirements written by humans.

RIF        Readable Intent Format
           Explicit, normalized, explainable intermediate representation.

EIG-Core   Executable Intent Graph
           Canonical typed semantic graph.

EIG-IR     Compiler/runtime intermediate representation
           Low-level representation used for execution.

EIG-Meta   Language/compiler specification layer
           Rules for generating compilers, diagnostics, visualizations, and lowerings.
```

## Pipeline

```text
Human requirement text
        ↓
RSL parsing / LLM-assisted interpretation / phrase matching
        ↓
Domain dictionary lookup
        ↓
Template expansion and inference
        ↓
Ambiguity detection
        ↓
RIF candidate
        ↓
RIF review / explanation / visualization
        ↓
EIG-Core graph construction
        ↓
Trusted checking:
  types
  permissions
  effects
  failures
  contracts
  secrets
        ↓
EIG-IR lowering
        ↓
Optimization
        ↓
Backend:
  native code
  Wasm
  bytecode VM
  workflow runtime
  accelerator runtime
```

## Layer 1: RSL

RSL is the human-facing requirement surface.

Example:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

RSL is sparse and compact. It is allowed to omit details that can be inferred from domain dictionaries, policies, templates, and operation contracts.

RSL should feel more like requirements than code.

## Layer 2: RIF

RIF is the normalized readable intent format. It makes implicit meaning visible.

Example:

```text
intent ConfirmOrder

subject:
  order: Order

state transition:
  order.status: Draft -> Confirmed

steps:
  reservation = Inventory.reserve(order.items)
  payment = PaymentGateway.capture(order.payment_method, order.total)
  shipment = Shipping.create_request(order, reservation)
  order.status = Confirmed

failure behavior:
  if PaymentGateway.capture fails:
    Inventory.release(reservation)
    stop with PaymentFailed

  if Shipping.create_request fails:
    order.status = PaidAwaitingFulfillmentRetry
    stop with FulfillmentRetryNeeded

permissions:
  read order.items
  read order.total
  read order.payment_method
  change Inventory
  change PaymentLedger
  change ShipmentQueue
  change order.status

effects:
  write Inventory
  call PaymentGateway
  write PaymentLedger
  call Shipping
  write ShipmentQueue
  write Order

guarantees:
  on success order.status is Confirmed
  on payment failure reservation is released
  on shipping failure order.status is PaidAwaitingFulfillmentRetry
```

RIF is not intended to be as short as RSL. It is intended to be reviewable and compilable.

## Layer 3: EIG-Core

EIG-Core is the canonical graph representation.

```text
Program = Modules + Types + Operations + Graph + Contracts + Views + Metadata
```

Core graph elements:

```text
Node
Edge
Type
Operation
Region
Permission
Effect
Contract
View
```

EIG-Core is the source of truth after elaboration.

Raw RSL is not the source of truth. RSL is a way to produce or edit RIF/EIG-Core.

## Layer 4: EIG-IR

EIG-IR is the compiler/runtime representation. It can be lowered into:

```text
native machine code
WebAssembly
portable bytecode
workflow state machines
GPU/accelerator kernels
embedded code
```

EIG-IR is not necessarily human-friendly. Its purpose is efficient and safe execution.

## Layer 5: EIG-Meta

EIG-Meta describes the language itself.

It defines:

```text
vocabulary
phrase patterns
syntax rules
elaboration rules
type rules
permission rules
effect rules
failure rules
contract rules
rewrite rules
lowering rules
diagnostic templates
explanation templates
view templates
compiler passes
backend contracts
```

EIG-Meta enables compiler generation and self-hosting.

## Domain dictionaries

RSL is concise because meaning is inherited from a domain dictionary.

Example:

```text
domain Commerce uses SafeDefaults.

thing Order:
  has status.
  has items.
  has total.
  has payment method.

phrase "reserve inventory":
  means Inventory.reserve(order.items)
  produces reservation
  compensation is Inventory.release(reservation)

phrase "capture payment":
  means PaymentGateway.capture(order.payment_method, order.total)
  produces payment
  may fail with PaymentFailed

phrase "create shipment":
  means Shipping.create_request(order, reservation)
  produces shipment
  may fail with ShipmentFailed
```

The human can write `reserve inventory` because the dictionary defines the actual operation, inputs, outputs, effects, and compensation.

## Policies and defaults

Policies supply inherited behavior.

Example:

```text
domain Commerce uses:
  SafeDefaults
  NoSecretLogging
  ExplicitFailureHandling
  SagaForExternalCalls
  TransactionalState
```

These policies imply rules such as:

```text
External calls may fail.
Secrets may not be logged.
State changes require change permission.
Failures must be handled, returned, retried, compensated, or marked impossible.
Compensations are registered for reversible side effects.
```

## Inference rule

The compiler may infer an omitted fact only if there is exactly one valid interpretation in the current domain context.

```text
Omitted fact -> inferred only when unique and justified.
Multiple valid interpretations -> ambiguity diagnostic.
No valid interpretation -> unknown phrase diagnostic.
```

## Provenance

Every inferred fact must carry provenance.

Example:

```text
change Inventory
  source: inferred from phrase "reserve inventory"
  because: Inventory.reserve declares write<Inventory>

release reservation on payment failure
  source: inherited from Inventory.reserve compensation contract
```

Provenance is essential for trust, explanation, and debugging.

## RIF as review artifact

RIF should support multiple display modes:

```text
collapsed summary
expanded requirements
expanded steps
expanded permissions
expanded effects
expanded failures
expanded guarantees
expanded cost model
expanded security model
expanded provenance
```

The same RIF object should also generate visual views.

## Compiler trust model

The front end may be smart and LLM-assisted, but the accepted program must pass the trusted checker.

```text
RSL / LLM elaborator -> untrusted or semi-trusted
RIF candidate        -> reviewable
EIG-Core candidate   -> checked
Trusted kernel       -> validates correctness obligations
Backend              -> must preserve checked semantics
```

## Executability

EIGL should not normally transpile to Python, JavaScript, Rust, or C++.

It should lower to an executable representation directly:

```text
EIG-Core -> EIG-IR -> native / Wasm / bytecode / workflow runtime / accelerator backend
```

Existing languages may be used for early implementation, but they are not part of the language's conceptual execution path.



---

<!-- Source: 03-rsl-requirement-surface-language.md -->

# RSL: Requirement Surface Language

RSL is the human-facing layer of EIGL.

It is intended to be compact, readable, and requirement-like. It should feel closer to writing business rules or system behavior than writing code.

## Purpose

RSL lets humans write sparse requirements such as:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

RSL is not the compiler's final source of truth. It is elaborated into RIF.

## Human-facing concepts

RSL should expose seven core concepts:

```text
Thing
Rule
Action
Step
State
Event
View
```

### Thing

A thing is a domain object.

```text
A Customer has an email address.
An Order has items.
A Payment has an amount.
```

### Rule

A rule is a required truth.

```text
Every order must have at least one item.
A confirmed order must have a captured payment.
A refund amount must not be greater than the captured payment amount.
```

### Action

An action is something the system can do.

```text
Confirm order:
  A draft order can be confirmed when inventory is available and payment succeeds.
```

### Step

A step is part of an action.

```text
Confirming an order reserves inventory.
Confirming an order captures payment.
Confirming an order creates a shipment request.
```

### State

A state is a named condition.

```text
An Order can be Draft, Confirmed, Cancelled, or Completed.
```

### Event

An event is something that happened.

```text
Payment captured has order, payment, and time.
```

### View

A view describes how to show the program.

```text
Show Confirm order as a flow.
Show Confirm order failures.
Show Confirm order permissions.
```

## Compact syntax

RSL supports compact action definitions:

```text
<Action>:
  <FromState> -> <ToState>
  by <step>, <step>, <step>
  on <failure> -> <outcome>
```

Example:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on payment failure -> PaymentFailed
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

## Full readable syntax

RSL also supports a more verbose requirement style:

```text
Confirm order:
  A draft order can be confirmed when inventory is available and payment succeeds.

  Confirming the order:
    reserves inventory,
    captures payment,
    creates a shipment request,
    leaves the order confirmed.

  If payment capture fails:
    release the inventory reservation.

  If shipment creation fails:
    the order waits for fulfillment retry.
```

## Allowed sentence forms

Initial RSL should use a constrained grammar.

```text
A <Thing> has <Property>.
A <Thing> can be <StateList>.
A <Thing> in <State> can become <State> when <Condition>.
<Action> does <StepList>.
<Action> changes <Thing>.
<Action> reads <Thing>.
If <Failure> happens, do <Recovery>.
If <Step> fails, do <Recovery>.
After <Action>, <Guarantee> must be true.
When <Event> happens, do <Action>.
```

## Domain phrases

RSL relies on domain phrases.

Example:

```text
phrase "reserve inventory":
  means Inventory.reserve(order.items)
  produces reservation
  changes Inventory
  compensation is Inventory.release(reservation)
```

The human writes:

```text
by reserve inventory
```

The system expands it to:

```text
call Inventory.reserve(order.items)
output reservation: InventoryReservation
changes Inventory
compensation Inventory.release(reservation)
```

## Inheritance and defaults

RSL stays concise by inheriting from:

```text
domain policies
thing templates
action templates
phrase definitions
operation contracts
library contracts
```

Example:

```text
domain Commerce uses SafeDefaults.
```

This may imply:

```text
changes require exclusive permission
external calls may fail
secrets cannot be logged
failures must be handled or returned
compensations are used for reversible effects
```

## Ambiguity

The system must not silently guess.

If this is written:

```text
Cancel order:
  refund payment.
```

and the domain has multiple refund methods, the compiler should report:

```text
Ambiguous phrase: "refund payment".

Possible meanings:
  1. refund full captured payment through PaymentGateway
  2. issue store credit
  3. create manual refund task

Choose a phrase meaning or define a default.
```

## RSL to RIF elaboration

RSL is elaborated into RIF through:

```text
phrase matching
vocabulary resolution
domain dictionary lookup
template expansion
inference
ambiguity detection
provenance recording
```

Human RSL:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

Generated RIF:

```text
intent ConfirmOrder
subject: order: Order
state transition: order.status Draft -> Confirmed
steps:
  Inventory.reserve(order.items)
  PaymentGateway.capture(order.payment_method, order.total)
  Shipping.create_request(order, reservation)
failure behavior:
  Shipping.create_request fails -> order.status = PaidAwaitingFulfillmentRetry
permissions: inferred
permissions provenance: recorded
```

## RSL is not arbitrary English

RSL should read naturally, but it must be parseable and deterministic.

Bad RSL:

```text
Make sure things work nicely after payment unless the service acts weird.
```

Good RSL:

```text
If payment capture fails because the provider is unavailable:
  retry up to 3 times.
  then stop with PaymentProviderUnavailable.
```

## RSL editing experience

A good editor should not require users to memorize syntax.

Users should be able to choose from actions such as:

```text
Create Thing
Create Rule
Create Action
Add Step
Add Failure Case
Add Guarantee
Show RIF
Show Flow View
Show Safety View
Show Cost View
```

The editor should guide users into valid RSL forms.

## RSL design rule

No human-facing sentence should contain more than one hidden compiler concept.

Bad:

```text
Borrow the order mutably while capturing payment and release the reservation on unwind.
```

Good:

```text
This action may change the order.
If payment capture fails, release the inventory reservation.
```



---

<!-- Source: 04-rif-readable-intent-format.md -->

# RIF: Readable Intent Format

RIF is the normalized, explicit, human-readable intermediate representation of EIGL.

RIF is the bridge between sparse human requirements and executable compiler graphs.

## Purpose

RIF should be:

```text
human-readable
explicit
reviewable
explainable
visualizable
compilable
```

RIF is where hidden meaning becomes visible.

## Core rule

```text
Humans may omit details in RSL.
RIF must show the details the system inferred.
```

RIF should not be as terse as RSL. It should be precise enough to build EIG-Core.

## RIF object structure

An RIF intent should contain:

```text
intent name
subject
inputs
outputs
required state
conditions
steps
outputs produced by steps
state changes
failure behavior
permissions
effects
guarantees
inferred facts
provenance
unresolved questions
```

## Example: ConfirmOrder

```text
intent ConfirmOrder

subject:
  order: Order

requires:
  order.status is Draft
  order.items.count > 0

state transition:
  order.status: Draft -> Confirmed

steps:
  1. Reserve inventory
     call: Inventory.reserve(order.items)
     output: reservation: InventoryReservation
     changes: Inventory
     compensation: Inventory.release(reservation)

  2. Capture payment
     call: PaymentGateway.capture(order.payment_method, order.total)
     output: payment: PaymentCapture
     changes: PaymentLedger
     external call: PaymentGateway
     may fail with: PaymentFailed

  3. Create shipment request
     call: Shipping.create_request(order, reservation)
     output: shipment: ShipmentRequest
     changes: ShipmentQueue
     external call: Shipping
     may fail with: ShipmentFailed

  4. Mark order confirmed
     set: order.status = Confirmed
     changes: order.status

failure behavior:
  if inventory reservation fails:
    stop with InventoryUnavailable

  if payment capture fails:
    release reservation
    stop with PaymentFailed

  if shipment creation fails:
    set order.status = PaidAwaitingFulfillmentRetry
    stop with FulfillmentRetryNeeded

guarantees:
  if this intent succeeds:
    order.status is Confirmed
    payment exists
    shipment request exists

  if payment capture fails after reservation:
    reservation is released

  if shipment creation fails:
    order.status is PaidAwaitingFulfillmentRetry

inferred permissions:
  reads:
    order.status
    order.items
    order.total
    order.payment_method

  changes:
    Inventory
    PaymentLedger
    ShipmentQueue
    order.status

inferred effects:
  writes Inventory
  calls PaymentGateway
  writes PaymentLedger
  calls Shipping
  writes ShipmentQueue
  writes Order
```

## RIF to English explanation

RIF can be translated back into plain English.

Example explanation:

```text
To confirm an order, the system first checks that the order is in Draft status and has at least one item. It then reserves inventory for the order items. If inventory cannot be reserved, the process stops.

After inventory is reserved, the system captures payment using the order payment method and total. If payment capture fails, the system releases the inventory reservation and stops with a payment failure.

If payment succeeds, the system creates a shipment request. If shipment creation fails, the order is marked as paid but waiting for fulfillment retry.

When all steps succeed, the order is marked Confirmed.
```

This explanation is generated from RIF, not guessed from source code.

## RIF syntax principles

RIF should be:

```text
section-based
indentation-friendly
regular
unambiguous
machine-parseable
pleasant to read
```

It should avoid dense symbolic syntax where possible.

## Required RIF sections

For a process/action intent, minimum sections should be:

```text
intent
subject or inputs
steps
guarantees
```

Recommended sections:

```text
requires
failure behavior
permissions
effects
provenance
```

## Inferred facts

Every inferred fact should be visible.

Example:

```text
inferred permissions:
  read order.items
    source: step "Reserve inventory"
    reason: Inventory.reserve consumes order.items as input

  change Inventory
    source: operation Inventory.reserve
    reason: operation declares write<Inventory>
```

## Provenance

RIF must support provenance.

A provenance record should include:

```text
fact
source kind
source reference
rule used
confidence or certainty
whether user-confirmed
```

Example:

```text
provenance:
  fact: compensation Inventory.release(reservation)
  source: operation contract Inventory.reserve
  rule: reversible_resource_compensation
  status: inferred
```

## Unresolved questions

If the system cannot resolve a fact, RIF should preserve the unresolved item explicitly.

```text
unresolved questions:
  - phrase "refund payment" has multiple meanings:
      1. PaymentGateway.refund_full
      2. StoreCredit.issue
      3. ManualRefund.create
```

A RIF document with unresolved questions cannot be compiled to executable code until they are resolved or explicitly deferred under a policy.

## RIF as source of truth

After human review, accepted RIF becomes the readable source of truth.

```text
RSL = authoring shorthand
RIF = accepted readable meaning
EIG-Core = canonical graph truth
```

## RIF editing

Users and LLMs should be able to edit RIF directly.

Example patch:

```text
patch Add fulfillment retry behavior

target:
  intent ConfirmOrder

change:
  when step "Create shipment request" fails:
    set order.status = PaidAwaitingFulfillmentRetry
    stop with FulfillmentRetryNeeded

add guarantee:
  if shipment creation fails:
    order.status is PaidAwaitingFulfillmentRetry
```

The compiler validates patches before accepting them.

## RIF consistency rules

A valid RIF document must satisfy:

```text
every referenced thing exists
every referenced field exists
every referenced state belongs to the subject's state set
every step has a known operation or declared primitive behavior
every declared failure is handled, returned, retried, compensated, or marked impossible
every state change has change permission
every external call has capability permission
every guarantee refers to known semantic objects
```

## RIF to EIG-Core

RIF lowers to EIG-Core by creating graph objects:

```text
intent -> operation/process node
subject -> input node/port
step -> operation invocation node
call -> calls edge
input dependency -> data edge
changes -> permission/effect edge
failure behavior -> failure edge
compensation -> compensation edge
guarantee -> contract node
provenance -> metadata edge
```

## Display modes

RIF should be shown with progressive disclosure.

Collapsed:

```text
ConfirmOrder:
  Draft order -> Confirmed order
  steps: reserve inventory, capture payment, create shipment
  failures: payment failure releases reservation; shipping failure marks fulfillment retry
```

Expanded:

```text
show steps
show permissions
show effects
show failures
show guarantees
show cost
show security
show provenance
```

## RIF design goal

RIF should be simple enough that a non-programmer can ask, “Is this what we mean?” while being precise enough that a compiler can ask, “Is this safe and executable?”



---

<!-- Source: 05-eig-core-semantic-graph.md -->

# EIG-Core: Semantic Graph Specification

EIG-Core is the canonical program representation.

RSL and RIF are textual projections. Visual diagrams are projections. EIG-Core is the semantic source of truth.

## Definition

An EIG-Core program is a typed, attributed, directed semantic graph.

```text
Program = {
  modules,
  types,
  operations,
  graph,
  contracts,
  views,
  metadata
}
```

More formally:

```text
Program P = (Modules, Types, Nodes, Edges, Contracts, Views, Metadata)
```

## Core primitives

EIG-Core v0.1 should use a small set of primitives:

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

Everything else is a specialization.

Mapping from human concepts:

```text
Thing   -> Type
Rule    -> Contract
Action  -> Operation with effect/process region
Step    -> Operation invocation
State   -> Type + state-machine region
Event   -> Type + emission effect
View    -> View query/projection
```

## Node

```text
Node {
  id: StableId
  kind: NodeKind
  name: Symbol
  type: TypeRef?
  ports: Port[]
  attributes: Map
  regions: Region[]
  contracts: ContractRef[]
  metadata: Metadata
}
```

## Port

```text
Port {
  name: Symbol
  direction: in | out | inout
  type: TypeRef
  permission: PermissionKind?
  multiplicity: one | optional | many
  effect: EffectRef?
}
```

## Edge

```text
Edge {
  id: StableId
  kind: EdgeKind
  from: PortRef | NodeRef
  to: PortRef | NodeRef
  guard: PredicateRef?
  attributes: Map
  metadata: Metadata
}
```

## Region

```text
Region {
  kind: pure | effect | process | state_machine | failure | proof | view
  nodes: NodeRef[]
  edges: EdgeRef[]
  schedule: sequential | parallel | partial_order | unordered
}
```

## Stable identity

Graph nodes should have stable IDs. IDs may be based on normalized semantic content where possible.

Human-readable names are aliases. Semantic identity belongs to the graph.

This enables:

```text
semantic diffs
content-addressed definitions
cache reuse
proof reuse
safe renaming
stable visualization
LLM graph patches
```

## Node kinds

Initial node kinds:

```text
module
thing/type
field
state
state_set
event
operation
operation_invocation
intent
step
value
failure
guarantee
contract
permission
effect
capability
view
trusted_capsule
```

A smaller implementation may model these as specializations of the core primitives.

## Edge kinds

Initial edge kinds:

```text
contains
has_field
has_state
has_type
input
output
data
order
calls
reads
changes
owns
consumes
produces
requires
ensures
may_fail
handles_failure
compensates
emits
transitions_to
refines
traces_to
renders
```

## Type model

Core types:

```text
Bool
Int<bits>
Float<bits>
Decimal<precision, scale>
Text
Bytes
Time
Duration
Money<Currency>
Id<Thing>
Ref<Thing>
Record
Variant
List<T>
Array<T, N>
Set<T>
Map<K, V>
Option<T>
Result<T, Error>
Secret<T>
Capability<E>
Tensor<T, Shape>
```

No implicit null.

Missing values use:

```text
Option<T> = Some<T> | None
```

Fallible operations use:

```text
Result<T, E> = Success<T> | Failure<E>
```

## Operation

```text
Operation {
  id: StableId
  name: Symbol
  inputs: Port[]
  outputs: Port[]
  permissions: PermissionRequirement[]
  effects: Effect[]
  preconditions: Contract[]
  body: Region
  failure_cases: FailureCase[]
  postconditions: Contract[]
  cost_model: CostModel
  metadata: Metadata
}
```

## Permission

```text
PermissionKind =
  Own<T>
  Read<T>
  Change<T>
  Move<T>
  Consume<T>
  Share<T>
  Secret<T>
  Capability<E>
```

## Effect

```text
Effect =
  pure
  read<Resource>
  write<Resource>
  call<Service>
  emit<Event>
  allocate<Resource>
  release<Resource>
  time
  random
  secret_read
  secret_write
  unsafe<Reason>
```

## Contract

Contracts are graph nodes, not comments.

```text
Contract {
  kind: precondition | postcondition | invariant | temporal | refinement | proof_obligation
  expression: Expression
  scope: NodeRef[]
  diagnostic: DiagnosticTemplate?
  metadata: Metadata
}
```

Contracts can compile to:

```text
static proof obligations
runtime assertions
property tests
database constraints
monitoring rules
audit checks
```

## Failure case

```text
FailureCase {
  source: OperationInvocationRef
  failure: FailureType
  handler: Region
  result: returned | retried | compensated | stopped | marked_impossible
  contracts: Contract[]
}
```

## EIG-Core graph example

For ConfirmOrder:

```text
nodes:
  intent ConfirmOrder
  input order: Order
  step ReserveInventory
  step CapturePayment
  step CreateShipmentRequest
  step MarkOrderConfirmed
  failure PaymentFailed
  failure ShipmentFailed
  guarantee OrderConfirmed

edges:
  ConfirmOrder contains ReserveInventory
  order.items data -> ReserveInventory.items
  ReserveInventory produces reservation
  reservation data -> CreateShipmentRequest.reservation
  order.payment_method data -> CapturePayment.method
  order.total data -> CapturePayment.amount
  CapturePayment may_fail PaymentFailed
  PaymentFailed handles_failure -> ReleaseReservation
  CreateShipmentRequest may_fail ShipmentFailed
  ShipmentFailed handles_failure -> SetPaidAwaitingFulfillmentRetry
  MarkOrderConfirmed ensures OrderConfirmed
```

## JSON-like serialization

```json
{
  "eig_version": "0.1",
  "module": "commerce.order",
  "nodes": [
    {
      "id": "intent:ConfirmOrder",
      "kind": "intent",
      "name": "ConfirmOrder"
    },
    {
      "id": "step:ReserveInventory",
      "kind": "operation_invocation",
      "name": "Reserve inventory",
      "call": "Inventory.reserve"
    }
  ],
  "edges": [
    {
      "kind": "contains",
      "from": "intent:ConfirmOrder",
      "to": "step:ReserveInventory"
    }
  ]
}
```

## Graph validity rules

A valid EIG-Core graph must satisfy:

```text
every edge source exists
every edge target exists
every port type exists
every operation invocation resolves to an operation
every data edge type-checks
every required permission is satisfiable
every effect is authorized by a capability or policy
every declared failure is handled, returned, retried, compensated, or proven impossible
every guarantee is well-formed
every trusted capsule is explicit and audited
```

## Graph patches

LLMs should edit EIG-Core/RIF through patches, not arbitrary rewrites.

Example:

```json
{
  "patch": "AddShippingFailurePolicy",
  "target": "intent:ConfirmOrder",
  "operations": [
    {
      "op": "add_failure_handler",
      "source": "step:CreateShipmentRequest",
      "failure": "ShipmentFailed",
      "handler": [
        "set order.status = PaidAwaitingFulfillmentRetry",
        "stop with FulfillmentRetryNeeded"
      ]
    }
  ]
}
```

Patch validation rules:

```text
target exists
added references resolve
new graph remains well-typed
new permissions are valid
new effects are authorized
new failure behavior is complete
new contracts do not contradict existing contracts
```

## EIG-Core design rule

EIG-Core must be expressive enough for compilers, visualizers, checkers, and explanation generators to operate on the same semantic object.

No semantic behavior should exist only in a diagram, only in a comment, or only in a backend.



---

<!-- Source: 06-safety-permissions-effects.md -->

# Safety, Permissions, and Effects

EIGL aims for Rust-like safety without exposing Rust-like complexity to non-programmers.

The human-facing model is based on responsibility and permissions.

## Human safety language

Humans should understand safety through simple rules:

```text
One action is responsible for a thing at a time.
Many actions may read a thing at the same time.
Only one action may change a thing at a time.
A thing cannot be used after it has been consumed.
A temporary permission cannot outlive the thing it refers to.
A secret cannot be shown, logged, copied, or sent unless a rule permits it.
An external system cannot be called unless the action has permission.
A failure must be handled, returned, retried, compensated, or marked impossible.
```

Compiler terms map to human terms:

```text
responsibility         -> ownership
read permission        -> shared immutable borrow
change permission      -> exclusive mutable borrow
consume                -> move / affine use
temporary permission   -> lifetime-bounded borrow
secret boundary        -> information-flow constraint
external permission    -> capability
failure handling       -> typed Result / failure edge
```

## Permission kinds

```text
Own<T>          full responsibility for T
Read<T>         shared read-only permission
Change<T>       exclusive mutable permission
Move<T>         transfer responsibility
Consume<T>      use exactly once, then unavailable
Share<T>        safe cross-task sharing
Secret<T>       value with restricted flows
Capability<E>  permission to perform effect E
```

## Central aliasing rule

At any point in the execution graph, a value may have either:

```text
many Read permissions
```

or:

```text
one Change permission
```

but never both, unless a synchronization primitive or policy explicitly permits it.

## Permissions inferred from RIF

Human/RSL:

```text
Confirming an order changes the order.
```

RIF:

```text
permissions:
  change order
```

EIG-Core:

```text
PermissionRequirement(Change<Order>)
```

Backend:

```text
exclusive mutable access / transaction lock / versioned write / linear capability
```

The exact backend mechanism depends on whether the thing is an in-memory value, a database entity, an actor resource, or an external system.

## Effect kinds

```text
pure
read<Resource>
write<Resource>
call<Service>
emit<Event>
allocate<Resource>
release<Resource>
time
random
secret_read
secret_write
unsafe<Reason>
```

Pure operations:

```text
cannot read mutable state
cannot write mutable state
cannot call external systems
cannot read time or randomness
can be cached, reordered, inlined, vectorized, and parallelized
```

Effectful operations:

```text
must declare effects
must have required capabilities
must obey effect ordering
must expose failures
```

## Capability system

External powers must be explicit.

Example:

```text
action Send password reset email:
  needs permission to send email
  needs permission to read the user's email address
  must not read the user's password
```

Compiler form:

```text
requires:
  Capability<Email.Send>
  Read<User.email>

forbidden:
  Read<User.password_hash>
```

## Failure model

EIGL should not use implicit exceptions as the default.

Failures are visible and typed.

Example:

```text
action Capture payment:
  may fail with:
    CardDeclined
    PaymentProviderUnavailable
    FraudCheckFailed
```

Callers must handle failures:

```text
if payment capture fails because card is declined:
  mark the order as PaymentFailed
  tell the customer to use another payment method

if payment capture fails because provider is unavailable:
  retry up to 3 times
  then mark the order as PaymentPendingRetry
```

Compiler rule:

```text
Every declared failure must be handled, returned, retried, compensated, or marked impossible with proof.
```

## Secrets and information flow

Secret values have restricted flows.

Example:

```text
new_password: Secret<Text>
payment_method: Secret<PaymentMethod>
```

Rules:

```text
Secrets may not be logged.
Secrets may not be rendered in normal explanations.
Secrets may not be sent to unauthorized external services.
Secrets may not be copied into public fields.
Secrets may be transformed only by approved operations.
```

Example allowed flow:

```text
new_password -> Hash.password -> password_hash
```

Example forbidden flow:

```text
new_password -> EventLog.emit
```

## Trusted capsules

Unsafe or foreign low-level behavior must be isolated in trusted capsules.

Human-facing form:

```text
trusted capsule Fast image resize:
  reason:
    uses platform-specific SIMD instructions

  allowed to:
    read input image memory
    write output image memory

  must guarantee:
    does not read outside the input image
    does not write outside the output image
    does not keep references after it returns
    produces a valid image buffer
```

Compiler-facing form:

```text
foreign unsafe operation FastImageResize
requires proof_or_audit
effects read<InputBuffer>, write<OutputBuffer>
obligations:
  no_out_of_bounds_read
  no_out_of_bounds_write
  no_reference_escape
  output_validity
```

Trusted capsules must be visible in safety and security views.

## No hidden unsafe behavior

Safe EIGL cannot contain:

```text
raw pointer operations
unchecked memory access
unchecked casts
hidden dynamic code execution
unauthorized external calls
untracked mutation
untracked secret flows
```

Unsafe behavior may exist only inside trusted capsules with declared contracts.

## Concurrency safety

If two steps are unordered or parallel, their permissions must not conflict.

Invalid:

```text
Step A changes Account.balance.
Step B changes Account.balance.
Step A and Step B run at the same time.
```

Diagnostic:

```text
This is not safe.

Step A and Step B both change Account.balance at the same time.
Only one step may change Account.balance at a time.

Choose one:
  run Step A before Step B
  run Step B before Step A
  combine the changes into one step
  protect the account with a synchronization rule
```

## Persistent-state safety

For database or durable entities, `Change<T>` may lower to:

```text
transactional write lock
optimistic concurrency version check
actor mailbox ownership
linear write token
event-sourced command permission
```

The safety rule remains the same:

```text
one changer at a time
```

## Performance model

To be performant, EIGL should avoid mandatory:

```text
garbage collection
dynamic dispatch
runtime reflection
implicit boxing
implicit exceptions
stringly typed runtime lookup
runtime type guessing
```

The compiler should support:

```text
static dispatch
monomorphization
stack allocation
region allocation
move semantics
copy elision
inlining
SIMD/vectorization
partial-order scheduling
native code
Wasm
accelerator lowering
```

## Cost view

Humans should not have to write low-level performance annotations, but experts should be able to inspect them.

Example:

```text
cost view for BuildInvoice:

allocation:
  Invoice is created as a value.
  Invoice lines are created from order items.
  The number of invoice lines equals the number of order items.

dispatch:
  static dispatch

memory:
  order is read, not copied
  invoice is returned to caller

parallelism:
  line calculations may run in parallel if tax calculation has no external effects
```

## Safety theorem target

Safe EIGL should aim to prove:

```text
A well-typed, well-permissioned, well-effect-checked Safe EIGL program cannot:

  use a value after it has been moved or consumed
  read or write through a dangling reference
  double-free a resource
  mutate a value while it is being read by another live permission
  create a data race
  access a missing value as if it existed
  call an external capability it was not granted
  leak a Secret value through an unauthorized path
  ignore a declared failure in strict mode
  break a declared invariant except inside an audited trusted capsule
```

This is a design target. A real implementation would need a formal core calculus, checker proofs, compiler tests, and backend validation before making strong claims.



---

<!-- Source: 07-visualization-and-views.md -->

# Visualization and Views

EIGL programs must be visualizable in meaningful ways.

The visual representation is not documentation. It is a projection of the same semantic graph used by the compiler.

## Core rule

```text
The visual representation must be semantics-preserving.
```

Moving boxes around changes layout only.
Connecting an output port to an input port changes the program.
Adding a failure edge changes runtime behavior.
Hiding a node changes the view, not the program.

## Program views

EIGL should define official view types:

```text
Story view
Flow view
State view
Failure view
Permission view
Effect view
Security view
Cost view
Trace view
Type view
Resource view
Contract view
```

## View graph

A view is a query over EIG-Core plus rendering rules.

```text
View {
  id
  name
  kind
  root
  include rules
  hide rules
  grouping rules
  layout rules
  rendering rules
}
```

Example:

```text
view ConfirmOrderFlow:
  kind: flow
  root: ConfirmOrder
  include:
    steps
    data dependencies
    success order
    failure paths
    compensations
  group by:
    resource
  hide:
    proof obligations
    low-level runtime details
  layout:
    left to right
```

## Official visual mappings

```text
Thing/entity       rectangle
Action/operation   rounded rectangle
Step               rounded rectangle
Process/intent     cluster/container
Predicate/rule     diamond or note
Resource           cylinder
Event              circle
State              rounded state box
Transition         arrow between states
Contract           note/annotation
Failure edge       dashed arrow
Data edge          solid arrow
Control edge       bold arrow
Effect edge        labeled arrow
Compensation       dashed reverse arrow
Secret             shield badge
Capability         key badge
External call      network badge
Trusted capsule    warning badge
```

## Flow view

Shows ordered or partially ordered steps.

Example:

```text
[Order: Draft]
      |
      v
[Reserve inventory]
      |
      v
[Capture payment]
      |
      v
[Create shipment request]
      |
      v
[Mark order Confirmed]
      |
      v
[Order: Confirmed]
```

Flow views answer:

```text
What happens?
In what order?
What data is produced?
What state changes?
```

## Failure view

Shows failures, retries, stops, and compensations.

Example:

```text
[Capture payment fails]
      |
      v
[Release inventory reservation]
      |
      v
[Stop: PaymentFailed]

[Shipment request fails]
      |
      v
[Mark order PaidAwaitingFulfillmentRetry]
      |
      v
[Stop: FulfillmentRetryNeeded]
```

Failure views answer:

```text
What can fail?
What happens when it fails?
What gets undone?
What state remains?
```

## Permission view

Shows read/change/own/consume permissions.

Example:

```text
ConfirmOrder
  reads:
    order.status
    order.items
    order.total
    order.payment_method

  changes:
    Inventory
    PaymentLedger
    ShipmentQueue
    order.status

  temporarily responsible for:
    reservation

  external calls:
    PaymentGateway
    Shipping
```

Permission views answer:

```text
What does this action read?
What does this action change?
What must be exclusive?
What resources are consumed?
What can be shared?
```

## Effect view

Shows resource effects and external calls.

Example:

```text
ConfirmOrder
  writes Inventory
  calls PaymentGateway
  writes PaymentLedger
  calls Shipping
  writes ShipmentQueue
  writes Order
  emits OrderConfirmed
```

Effect views answer:

```text
What persistent state is touched?
What external systems are called?
What events are emitted?
Can this be cached, reordered, or parallelized?
```

## Security view

Shows secrets, trust boundaries, and allowed flows.

Example:

```text
Sensitive values:
  payment_method

Allowed flow:
  payment_method -> PaymentGateway

Forbidden flow:
  payment_method -> logs
  payment_method -> Shipping
  payment_method -> analytics

Trusted capsules:
  none
```

Security views answer:

```text
What secrets exist?
Where can secrets flow?
What external services receive sensitive data?
Where are trusted capsules used?
```

## Cost view

Shows cost-relevant facts.

Example:

```text
ConfirmOrder

external calls:
  PaymentGateway
  Shipping

allocations:
  reservation handle
  payment result
  shipment request

parallelism:
  not parallel by default because later steps depend on earlier results

dynamic dispatch:
  none

unbounded loops:
  none
```

Cost views answer:

```text
Where does time go?
What allocates?
What can be parallelized?
What is statically dispatched?
What has external latency?
```

## State view

Shows state machines.

Example:

```text
Draft --payment captured + inventory reserved--> Confirmed
Confirmed --shipment delivered-----------------> Completed
Confirmed --cancel requested, shipment not started--> Cancelled
```

State views answer:

```text
What states exist?
What transitions are allowed?
What conditions guard transitions?
What effects happen during transitions?
```

## Trace view

Shows one actual execution.

Example:

```text
Trace ConfirmOrder #12345

1. checked order.status == Draft             success
2. reserved inventory                        reservation #R9
3. captured payment                          payment #P7
4. create shipment request                   failed: ShippingUnavailable
5. set order.status = PaidAwaitingFulfillmentRetry
6. stopped with FulfillmentRetryNeeded
```

Trace views answer:

```text
What actually happened?
Which path was taken?
What failed?
What values were produced?
What state changed?
```

## View consistency rules

A valid EIGL implementation must obey:

```text
Views must not introduce semantics.
Views may hide graph elements but must indicate hidden semantic content.
Every visible element must map to one or more EIG-Core nodes or edges.
Every semantic edit through a view must produce a valid graph edit.
Layout metadata must not affect execution.
Generated diagrams must be reproducible from graph + view definition.
The same graph rendered through different views must preserve semantic identity.
```

## Editing through views

A user may edit a diagram. Edits must become graph patches.

Examples:

```text
Add arrow from CapturePayment failure to ReleaseReservation
  -> add failure handler edge

Connect reservation output to CreateShipment input
  -> add data edge

Delete MarkOrderConfirmed step
  -> remove operation invocation, then re-check guarantees
```

Invalid visual edits should produce human diagnostics.

Example:

```text
This edit is not safe.

You connected two parallel steps that both change Inventory.
Only one step may change Inventory at a time.
```

## View templates

EIG-Meta can define view templates.

```text
view template Flow view for Process:
  show:
    process subject
    ordered steps
    produced values
    final state
    failure handlers

  render:
    subject as state box
    step as action box
    produced value as small output node
    success edge as solid arrow
    failure edge as dashed arrow
    compensation edge as dashed reverse arrow

  hide by default:
    permission edges
    type constraints
    proof obligations

  reveal on request:
    permissions
    effects
    failures
    costs
    security flows
```

## Visualization design goal

A program should be understandable without reading implementation syntax.

Different people should see different projections:

```text
Product owner       story view, flow view
Backend engineer    effect view, permission view, trace view
Security engineer   security view, capability view
QA engineer         failure view, test view, trace view
Architect           type view, module view, resource view
Compiler            EIG-Core / EIG-IR
LLM                 RIF / graph patch format
```



---

<!-- Source: 08-compiler-generation.md -->

# Compiler Generation

EIGL should eventually generate its compiler from language specifications written in EIGL itself.

The compiler is not primarily hand-written code. It is a set of requirements, rules, transformations, constraints, diagnostics, explanations, visualizations, and lowering contracts.

## Thesis

```text
The compiler is an EIGL program.
```

Or more precisely:

```text
The language definition is written in EIG-Meta.
The compiler is generated from the language definition.
The generated compiler validates, explains, visualizes, and compiles EIGL programs.
```

## Language Definition Package

The compiler should be generated from a **Language Definition Package** or **LDP**.

An LDP contains:

```text
vocabulary
phrase patterns
domain dictionaries
syntax rules
elaboration rules
type rules
permission rules
effect rules
failure rules
contract rules
rewrite rules
lowering rules
diagnostic templates
explanation templates
visual view definitions
test-generation rules
backend contracts
```

## EIG-Meta

EIG-Meta is the language layer for writing LDPs.

It defines compiler concepts such as:

```text
SourceText
Phrase
PhrasePattern
ParseTree
RequirementDraft
ResolvedRequirement
Intent
Type
Operation
Permission
Effect
Failure
Contract
Diagnostic
Graph
Node
Edge
RewriteRule
CompilerPass
LoweringTarget
BackendContract
```

## Compiler passes as intents

Each compiler pass can be specified as an EIGL intent.

Example:

```text
compiler pass Resolve domain vocabulary:

  input:
    requirement draft
    domain dictionary

  output:
    resolved requirement

  does:
    find every noun phrase
    match each noun phrase to a known thing
    find every verb phrase
    match each verb phrase to a known action or phrase rule
    record every match with provenance

  if a phrase has no meaning:
    report Unknown phrase

  if a phrase has more than one meaning:
    report Ambiguous phrase

  guarantees:
    every resolved phrase has exactly one meaning
    every inferred meaning has a provenance record
```

## Compiler pipeline

Generated compiler pipeline:

```text
Requirement text
  ↓
Parse candidate phrases
  ↓
Resolve domain vocabulary
  ↓
Elaborate into RIF
  ↓
Normalize RIF
  ↓
Build EIG-Core graph
  ↓
Check types
  ↓
Check permissions
  ↓
Check effects
  ↓
Check failures
  ↓
Check contracts
  ↓
Generate explanations and views
  ↓
Lower to EIG-IR
  ↓
Optimize
  ↓
Emit executable artifact
```

## Generated components

The LDP should generate:

```text
phrase matcher
resolver
RIF elaborator
EIG-Core builder
type checker
permission checker
effect checker
failure checker
contract checker
explanation renderer
visual renderer
optimizer
lowering pipeline
diagnostics
tests
```

Not every generated component is trusted. The trusted checker validates outputs.

## Small trusted kernel

The entire compiler should not be trusted equally.

Suggested trust model:

```text
Generated elaborator: untrusted or semi-trusted
Generated optimizer: untrusted or semi-trusted
Generated explanation renderer: untrusted
Generated visualizer: untrusted
Generated diagnostics: untrusted

Trusted kernel:
  checks EIG-Core validity
  checks types
  checks permissions
  checks effects
  checks contracts/proof objects
  checks lowering obligations
```

The front end may be clever or LLM-assisted, but the accepted graph must pass the trusted checker.

## Vocabulary definitions

Example EIG-Meta vocabulary:

```text
word "order":
  means thing Order
  plural "orders"

phrase "draft order":
  means Order where status is Draft

phrase "confirmed order":
  means Order where status is Confirmed
```

## Action phrase definitions

```text
phrase "reserve inventory":
  needs order: Order
  means call Inventory.reserve(order.items)
  produces reservation: InventoryReservation
  changes Inventory
  compensation is Inventory.release(reservation)
```

## Phrase pattern definitions

```text
phrase pattern State transition requirement:

  form:
    "{subject} can become {target_state} when {condition}"

  produces:
    intent StateTransitionIntent

  fields:
    subject: Thing
    target_state: State of subject
    condition: Condition
```

## Compact syntax definition

```text
phrase pattern Compact state transition:

  form:
    "{action_name}: {from_state} -> {to_state} by {step_list}"

  examples:
    "Confirm order: Draft -> Confirmed by reserve inventory, capture payment, create shipment"

  parse:
    action_name is ActionName
    from_state is State
    to_state is State
    step_list is List<Phrase>

  infer:
    subject is the thing that owns from_state
    action changes subject state from from_state to to_state
    each phrase in step_list becomes a step

  require:
    from_state and to_state belong to the same state set
    every step phrase resolves to exactly one action
    the final state transition is valid for the subject

  produce:
    StateTransitionIntent
```

## Checking rule example: exclusive change

Human-readable rule:

```text
rule A thing that is changed by an action must be available exclusively to that action while the change happens.
```

RIF/EIG-Meta:

```text
checking rule ExclusiveChangePermission

applies to:
  action: Action
  thing: Thing

when:
  action changes thing

requires:
  action has Change permission for thing
  no other active action has Read permission for thing during the change
  no other active action has Change permission for thing during the change

diagnostic if violated:
  "This is not safe because {action} changes {thing}, while {other_action} also reads or changes it."
```

Formal constraint:

```text
Changes(action, thing) => RequiresPermission(action, Change(thing))

Holds(Change(action_a, thing), time)
  => not Holds(Read(action_b, thing), time)
  and not Holds(Change(action_b, thing), time)
  unless action_a == action_b
```

## Checking rule example: failures

```text
checking rule Declared failures must be handled:

  when:
    an action contains a step
    and the step may fail with a failure

  require:
    the action handles the failure
    or the action returns the failure
    or the action proves the failure cannot happen

  diagnostic if violated:
    "{step} may fail with {failure}, but {action} does not say what to do."
```

Formal constraint:

```text
StepInAction(step, action)
and MayFail(step, failure)
requires
  Handles(action, step, failure)
  or Returns(action, failure)
  or ImpossibleWithProof(action, step, failure)
```

## Diagnostics as part of the spec

Diagnostics should be generated with rules.

```text
diagnostic ConflictingChangePermission:

  when:
    two unordered steps both change the same thing

  message:
    "This is not safe because {first_step} and {second_step} both change {thing} at the same time."

  explain:
    "Only one step may change a thing at a time."

  suggest:
    "Run {first_step} before {second_step}."
    "Run {second_step} before {first_step}."
    "Combine both changes into one step."
    "Add a synchronization rule."
```

## Rewrite rules

Optimizations should be declarative and checked for legality.

```text
rewrite rule Remove redundant state set:

  when:
    a step sets field to value
    and the next step sets the same field to the same value
    and no step between them reads the field

  replace:
    remove the second set

  legal only if:
    setting the field has no external effect
    setting the field does not emit an event
    setting the field does not update audit history

  guarantee:
    observable behavior is unchanged
```

## Parallelization rule

```text
rewrite rule Parallelize independent pure steps:

  when:
    two steps are unordered
    both steps are pure
    neither step reads a value produced by the other

  replace:
    run the steps in parallel

  legal only if:
    both steps have no effects
    both steps do not read time or randomness

  guarantee:
    the result is deterministic
```

## Lowering rules

```text
lowering rule Process step to IR block:

  when:
    a process contains a step

  emit:
    an IR block for the step
    data edges for step inputs
    success edge to the next step
    failure edge to the step failure handler

  require:
    every input has a producer
    every failure edge has a target
    every effect has a capability

  guarantee:
    the IR block has the same declared effects as the source step
```

## Backend contracts

```text
backend contract NativeCode:

  accepts:
    checked EIG-IR

  guarantees:
    preserves control flow
    preserves data dependencies
    preserves permission constraints
    preserves effect order where required
    does not introduce unauthorized effects
    does not expose Secret values
    does not call trusted capsules unless explicitly present in EIG-IR

  may optimize:
    pure computations
    independent reads
    stack allocation
    region allocation
    static dispatch
    vectorization
    inlining

  must report:
    unsupported target feature
    unsupported trusted capsule
    layout conflict
    unresolved external capability
```

## Compiler generation pipeline

```text
Language Definition Package
        ↓
Validate language spec
        ↓
Generate compiler passes
        ↓
Generate diagnostics
        ↓
Generate explanation renderer
        ↓
Generate visual renderers
        ↓
Generate tests/fuzzers
        ↓
Generate bootstrap compiler
```

## LLM role in compiler generation

LLMs may help draft compiler rules, but cannot be trusted as final authority.

Good roles:

```text
suggest phrase patterns
translate informal design rules into EIG-Meta drafts
generate examples
explain compiler rules
find missing diagnostics
propose rewrite rules
generate tests from the spec
```

Bad roles:

```text
silently changing compiler semantics
generating unchecked machine code
resolving ambiguity without provenance
inventing lowering behavior without contracts
bypassing the trusted checker
```

## Compiler-generation design rule

The generated compiler can be clever. The trusted checker must be boring.



---

<!-- Source: 09-self-hosting-bootstrap.md -->

# Self-Hosting and Bootstrap Plan

The long-term goal is for EIGL to compile itself.

```text
EIGL is specified in EIGL.
The EIGL compiler is generated from that specification.
The generated compiler can compile the same specification again.
```

## Why self-hosting matters

Self-hosting demonstrates that:

```text
the language can express compiler logic
the intermediate representation is precise enough
the checker can validate complex systems
the generated compiler is not merely a toy
the language can maintain itself
```

## Start with RIF, not RSL

The first self-hosted compiler should not be written in the compact requirement surface.

It should be written in RIF.

Reason:

```text
RSL is pleasant but ambiguous by design.
RIF is explicit, normalized, and reviewable.
EIG-Core is canonical and checkable.
```

Self-hosting should begin at the precise layer.

## Bootstrap stages

### Stage -1: Manual trusted kernel

Written in an existing systems language.

Possible choices:

```text
Rust
Zig
C++
OCaml
Haskell
```

Responsibilities:

```text
parse EIG-Core or restricted RIF
validate graph well-formedness
check basic types
check permissions
check effects
check failures
emit bytecode or Wasm
run tests
```

This implementation should be small, auditable, and boring.

### Stage 0: Seed compiler

Written in an existing language.

Capabilities:

```text
compile restricted RIF to EIG-Core
run basic checker
lower to bytecode or Wasm
execute simple process graphs
produce diagnostics
produce views
```

Stage 0 does not need full RSL.

### Stage 1: Compiler spec written in RIF/EIG-Meta

Define compiler passes as RIF/EIG-Meta intents:

```text
BuildCoreGraphFromRIF
CheckReferences
InferPermissions
CheckEffects
CheckFailures
RenderExplanation
RenderFlowView
LowerProcessToIR
```

Stage 0 compiles this compiler spec into Compiler-1.

### Stage 2: Compiler-1 compiles the compiler spec

Compiler-1 compiles the same RIF/EIG-Meta compiler spec and produces Compiler-2.

### Stage 3: Compiler-2 compiles the compiler spec

Compiler-2 produces Compiler-3.

### Fixed-point check

Check that Compiler-2 and Compiler-3 are equivalent.

Possible equivalence levels:

```text
same accepted EIG-Core
same diagnostics
same generated explanations
same graph outputs
same executable behavior
bit-identical output under deterministic build mode
```

Bit-identical output is ideal but not required at first.

## Self-hosting definition

Minimal self-hosting:

```text
An EIGL compiler generated from an EIGL compiler specification can compile the same specification again and produce an equivalent compiler.
```

Strong self-hosting:

```text
Compiler-N and Compiler-N+1 produce equivalent EIG-Core and equivalent executable output for the compiler specification and standard test suite.
```

Deterministic self-hosting:

```text
Compiler-N and Compiler-N+1 are bit-identical under deterministic build settings.
```

## Trusted base

Initial trusted base:

```text
seed compiler
trusted checker
runtime primitives
backend code generator
standard library primitives
```

Over time, move into EIGL:

```text
phrase matcher
resolver
RIF elaborator
diagnostics
view renderers
many type rules
many permission rules
many effect rules
rewrite rules
optimization rules
lowering rules
test generation
```

Remain trusted:

```text
EIG-Core checker
proof checker
primitive runtime
machine-code/Wasm emitter or verified backend bridge
trusted capsules
```

## Compiler pass example in RIF

```text
intent BuildCoreGraphFromRIF

subject:
  rif_document: RIFDocument

inputs:
  language_spec: LanguageDefinitionPackage

outputs:
  graph: EIGCoreGraph

steps:
  1. Create graph module nodes from RIF modules.
  2. Create type nodes from RIF things.
  3. Create operation nodes from RIF actions.
  4. Create contract nodes from RIF rules.
  5. Create effect edges from RIF effect declarations.
  6. Create permission edges from RIF permission declarations.
  7. Create failure edges from RIF failure behavior.
  8. Validate that every reference resolves to one graph node.

failure behavior:
  if a reference cannot be resolved:
    emit UnresolvedReference diagnostic
    stop

  if a reference resolves to multiple graph nodes:
    emit AmbiguousReference diagnostic
    stop

guarantees:
  every graph node has a stable identity
  every edge source exists
  every edge target exists
  every operation has declared inputs and outputs
```

## Permission inference compiler pass

```text
compiler pass Infer permissions:

  input:
    intent: Intent

  output:
    intent with permission requirements

  rule:
    If a step reads a field, the step needs Read permission for that field.

  rule:
    If a step sets a field, the step needs Change permission for that field.

  rule:
    If a step passes a thing to an action that consumes it, the thing is no longer available after that step.

  rule:
    If two unordered steps both need Change permission for the same thing, the intent is unsafe.

  diagnostic:
    "Steps {first_step} and {second_step} both change {thing}. They must be ordered, combined, or protected by synchronization."

  guarantees:
    every read has a Read permission
    every change has a Change permission
    no unordered steps require conflicting Change permissions
```

## Compiler determinism policy

Compiler passes should be deterministic by default.

```text
policy CompilerDeterminism:

  compiler passes may not read:
    wall clock time
    random numbers
    environment variables
    filesystem outside declared inputs
    network resources

  unless:
    the pass declares the effect
    and the build mode permits it
```

This supports reproducible builds and fixed-point tests.

## Meta-rule: compiler must not invent meaning

```text
rule A compiler pass must not invent meaning.

A compiler pass may:
  infer a fact from a declared rule,
  copy a fact from an input,
  derive a fact from a checked constraint,
  or ask for clarification.

A compiler pass must not:
  choose between two possible meanings without a disambiguation rule.
```

Formal form:

```text
DerivedFact(f) requires
  SourceFact(f)
  or InferenceRule(rule, premises, f)
  or CheckedConstraint(c, f)
  or UserDecision(f)

Ambiguous(candidates) and no DisambiguationRule
  => Error(AmbiguousMeaning)
```

## Practical self-hosting milestone

The first major milestone should be:

```text
The RIF compiler written in RIF compiles itself.
```

Not:

```text
The full English-like RSL compiler compiles itself.
```

RSL self-hosting can come later.

## Long-term goal

```text
Requirements compile programs.
Requirements also compile the compiler.
```



---

<!-- Source: 10-prototype-roadmap.md -->

# Prototype Roadmap

This roadmap turns the specification into a practical implementation sequence.

## Guiding principle

Build the explicit layer first.

Do not begin with unrestricted natural language.

```text
RIF first.
EIG-Core second.
Checkers third.
Views fourth.
Execution fifth.
RSL sixth.
Compiler generation seventh.
Self-hosting eighth.
```

## Phase 1: EIG-Core schema

Implement the graph schema.

Deliverables:

```text
core_model.py or core_model.rs
Program
Module
Node
Edge
Type
Operation
Port
Region
Permission
Effect
Contract
FailureCase
View
JSON serialization
stable IDs
```

Acceptance criteria:

```text
Can construct a graph for ConfirmOrder.
Can serialize and deserialize the graph.
Can validate edge source/target existence.
Can produce a stable semantic hash for graph nodes.
```

## Phase 2: RIF parser

Implement a restricted RIF parser.

Deliverables:

```text
rif_model
rif_parser
examples/confirm_order.rif.md
examples/password_reset.rif.md
examples/bad_parallel_update.rif.md
```

Start with section-based parsing. Unknown lines may be preserved as raw semantic text.

Acceptance criteria:

```text
Can parse intent name.
Can parse subject.
Can parse requires.
Can parse steps.
Can parse call/output/changes/may fail/compensation.
Can parse failure behavior.
Can parse guarantees.
```

## Phase 3: RIF to EIG-Core builder

Implement graph construction from RIF.

Deliverables:

```text
graph_builder
reference resolver
provenance records
```

Acceptance criteria:

```text
Every RIF intent becomes an operation/process node.
Every step becomes an operation invocation node.
Every call becomes a calls edge.
Every changes line becomes a permission/effect node or edge.
Every failure behavior becomes a failure edge.
Every guarantee becomes a contract node.
```

## Phase 4: Basic checker

Implement initial safety checks.

Checks:

```text
reference resolution
type existence
field existence
state existence
known operation calls
failure completeness
permission conflicts
effect authorization
secret flow basics
guarantee well-formedness
compensation path availability
```

Acceptance criteria:

```text
confirm_order.rif.md passes.
bad_parallel_update.rif.md fails with a clear diagnostic.
password_reset.rif.md passes and detects secret handling constraints.
```

## Phase 5: Views and explanations

Generate human-readable views from RIF/EIG-Core.

Deliverables:

```text
explanations.py
views.py
flow view
failure view
permission view
effect view
security view
cost placeholder view
```

Acceptance criteria:

```text
Can explain ConfirmOrder in plain English.
Can render plain-text flow view.
Can render plain-text failure view.
Can render plain-text permission view.
Can render JSON view model for future diagrams.
```

## Phase 6: Simple interpreter

Implement a simple EIG process interpreter or simulator.

Deliverables:

```text
interpreter.py
mock resources
mock operation outcomes
trace generation
```

Acceptance criteria:

```text
Can simulate successful ConfirmOrder.
Can simulate payment failure and show compensation.
Can simulate shipping failure and show final retry state.
Can produce trace view.
```

## Phase 7: Bytecode or Wasm lowering

Choose a minimal executable backend.

Option A: custom bytecode interpreter.

```text
CHECK_REQUIRES
CALL_EFFECT
STORE_OUTPUT
REGISTER_COMPENSATION
HANDLE_FAILURE
SET_FIELD
ASSERT_GUARANTEE
RETURN
```

Option B: Wasm lowering for pure/simple effect-free operations first.

Acceptance criteria:

```text
Can lower a simple pure action.
Can lower a simple sequential process to bytecode.
Can execute bytecode and produce a trace.
```

## Phase 8: RSL subset

Implement compact RSL only after RIF works.

Deliverables:

```text
phrase dictionary
phrase pattern matcher
RSL to RIF elaborator
ambiguity diagnostics
```

Initial syntax:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

Acceptance criteria:

```text
Can elaborate compact RSL into RIF.
Can report unknown phrase.
Can report ambiguous phrase.
Can record provenance.
```

## Phase 9: EIG-Meta subset

Implement compiler-generation metadata.

Deliverables:

```text
phrase pattern definitions
checking rule definitions
diagnostic templates
view templates
```

Acceptance criteria:

```text
Can define a phrase pattern in EIG-Meta.
Can generate a phrase matcher from it.
Can define a diagnostic template and use it in checker output.
```

## Phase 10: Self-hosting preparation

Start writing compiler passes in RIF.

Candidate passes:

```text
BuildCoreGraphFromRIF
CheckReferences
InferPermissions
CheckEffects
CheckFailures
RenderExplanation
RenderFlowView
LowerProcessToBytecode
```

Acceptance criteria:

```text
At least one checker pass is specified in RIF.
Seed compiler can parse it.
Seed compiler can build EIG-Core for it.
```

## Suggested first examples

### Confirm order

Demonstrates:

```text
state transition
external calls
failure handling
compensation
permissions
effects
visual flow
```

### Password reset

Demonstrates:

```text
secrets
one-time token
state mutation
session revocation
guarantees
security view
```

### Bad parallel update

Demonstrates:

```text
permission conflict
diagnostic quality
human-friendly borrow-checker-like error
```

## Minimal CLI

```text
eigl parse examples/confirm_order.rif.md
eigl graph examples/confirm_order.rif.md --json
eigl check examples/confirm_order.rif.md
eigl explain examples/confirm_order.rif.md
eigl view examples/confirm_order.rif.md --flow
eigl view examples/confirm_order.rif.md --failure
eigl view examples/confirm_order.rif.md --permissions
eigl simulate examples/confirm_order.rif.md --scenario success
eigl simulate examples/confirm_order.rif.md --scenario payment_failure
```

## Non-goals for first prototype

Do not implement yet:

```text
full arbitrary English parsing
full native codegen
advanced theorem proving
full visual editor
distributed runtime
LLM integration
self-hosting
formal proof of safety
```

## First prototype success definition

The prototype is successful when it proves this loop:

```text
compact or explicit intent
  -> readable normalized representation
  -> semantic graph
  -> safety checks
  -> explanation
  -> visualization
  -> execution trace
```

Even without native code generation, this validates the language architecture.



---

<!-- Source: 11-examples.md -->

# Examples

This file collects example programs across RSL, RIF, explanations, and views.

## Example 1: Confirm Order

### RSL

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on payment failure -> PaymentFailed
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

### Domain dictionary entries

```text
thing Order:
  has status: OrderStatus
  has items: list of OrderItem
  has total: Money
  has payment method: PaymentMethod

states for Order:
  Draft
  Confirmed
  PaidAwaitingFulfillmentRetry
  Cancelled
  Completed

phrase "reserve inventory":
  means Inventory.reserve(order.items)
  produces reservation: InventoryReservation
  changes Inventory
  may fail with InventoryUnavailable
  compensation is Inventory.release(reservation)

phrase "capture payment":
  means PaymentGateway.capture(order.payment_method, order.total)
  produces payment: PaymentCapture
  changes PaymentLedger
  calls PaymentGateway
  may fail with PaymentFailed

phrase "create shipment":
  means Shipping.create_request(order, reservation)
  produces shipment: ShipmentRequest
  changes ShipmentQueue
  calls Shipping
  may fail with ShipmentFailed
```

### RIF

```text
intent ConfirmOrder

subject:
  order: Order

requires:
  order.status is Draft
  order.items.count > 0

state transition:
  order.status: Draft -> Confirmed

steps:
  1. Reserve inventory
     call: Inventory.reserve(order.items)
     output: reservation: InventoryReservation
     changes: Inventory
     may fail with: InventoryUnavailable
     compensation: Inventory.release(reservation)

  2. Capture payment
     call: PaymentGateway.capture(order.payment_method, order.total)
     output: payment: PaymentCapture
     changes: PaymentLedger
     external call: PaymentGateway
     may fail with: PaymentFailed

  3. Create shipment request
     call: Shipping.create_request(order, reservation)
     output: shipment: ShipmentRequest
     changes: ShipmentQueue
     external call: Shipping
     may fail with: ShipmentFailed

  4. Mark order confirmed
     set: order.status = Confirmed
     changes: order.status

failure behavior:
  if inventory reservation fails:
    stop with InventoryUnavailable

  if payment capture fails:
    release reservation
    stop with PaymentFailed

  if shipment creation fails:
    set order.status = PaidAwaitingFulfillmentRetry
    stop with FulfillmentRetryNeeded

guarantees:
  if this intent succeeds:
    order.status is Confirmed
    payment exists
    shipment request exists

  if payment capture fails after reservation:
    reservation is released

  if shipment creation fails:
    order.status is PaidAwaitingFulfillmentRetry

inferred permissions:
  reads:
    order.status
    order.items
    order.total
    order.payment_method

  changes:
    Inventory
    PaymentLedger
    ShipmentQueue
    order.status

inferred effects:
  writes Inventory
  calls PaymentGateway
  writes PaymentLedger
  calls Shipping
  writes ShipmentQueue
  writes Order
```

### Explanation

```text
To confirm an order, the system first checks that the order is in Draft status and has at least one item. It then reserves inventory for the order items. If inventory cannot be reserved, the process stops.

After inventory is reserved, the system captures payment using the order payment method and total. If payment capture fails, the system releases the inventory reservation and stops with a payment failure.

If payment succeeds, the system creates a shipment request. If shipment creation fails, the order is marked as paid but waiting for fulfillment retry.

When all steps succeed, the order is marked Confirmed.
```

### Flow view

```text
[Order: Draft]
      |
      v
[Reserve inventory]
      |
      v
[Capture payment]
      |
      v
[Create shipment request]
      |
      v
[Mark order Confirmed]
      |
      v
[Order: Confirmed]
```

### Failure view

```text
[Inventory reservation fails]
      |
      v
[Stop: InventoryUnavailable]

[Payment capture fails]
      |
      v
[Release inventory reservation]
      |
      v
[Stop: PaymentFailed]

[Shipment request fails]
      |
      v
[Mark order PaidAwaitingFulfillmentRetry]
      |
      v
[Stop: FulfillmentRetryNeeded]
```

## Example 2: Password Reset

### RSL

```text
Password reset:
  A user with a valid reset token can set a new password.
  The token can be used only once.
  After reset, all active sessions are revoked.
```

### RIF

```text
intent ResetPassword

subject:
  user: User

inputs:
  token: PasswordResetToken
  new_password: Secret Text

requires:
  token.exists
  token.user is user
  token.status is Unused
  token.expires_at is after now
  PasswordPolicy.accepts(new_password)

steps:
  1. Hash new password
     input: new_password
     output: password_hash: PasswordHash
     secret handling: new_password may not be logged or stored directly

  2. Set user password
     set: user.password_hash = password_hash
     changes: User

  3. Mark token used
     set: token.status = Used
     changes: PasswordResetToken

  4. Revoke active sessions
     call: Session.revoke_all(user)
     changes: SessionStore

guarantees:
  token.status is Used
  user password has changed
  user has no active sessions
  new_password is not stored directly
  new_password is not logged

permissions:
  read token
  change user.password_hash
  change token.status
  change SessionStore

effects:
  reads PasswordResetToken
  writes User
  writes PasswordResetToken
  writes SessionStore
```

### Explanation

```text
A password reset is allowed only when the reset token exists, belongs to the user, has not expired, and has not already been used. The new password must satisfy the password policy. The system hashes the new password, stores only the hash, marks the token as used, and revokes all active sessions. The plain password is never stored or logged.
```

### Security view

```text
Sensitive values:
  new_password

Allowed flow:
  new_password -> Hash.password -> password_hash

Forbidden flow:
  new_password -> logs
  new_password -> direct storage
  new_password -> events

State changes:
  user.password_hash
  token.status
  SessionStore
```

## Example 3: Bad Parallel Update

### RIF

```text
intent BadParallelUpdate

subject:
  counter: Counter

steps:
  parallel:
    1. Step A
       set: counter.value = counter.value + 1
       changes: counter.value

    2. Step B
       set: counter.value = counter.value + 1
       changes: counter.value

guarantees:
  counter.value increased by 2
```

### Expected diagnostic

```text
This is not safe.

Step A and Step B both change counter.value at the same time.
Only one step may change counter.value at a time.

Choose one:
  run Step A before Step B
  run Step B before Step A
  combine both changes into one step
  add a synchronization rule
```

### Corrected version

```text
intent SequentialCounterUpdate

subject:
  counter: Counter

steps:
  1. Step A
     set: counter.value = counter.value + 1
     changes: counter.value

  2. Step B
     set: counter.value = counter.value + 1
     changes: counter.value

guarantees:
  counter.value increased by 2
```

## Example 4: Loan Approval

### RSL

```text
Loan approval:
  A loan application can be approved when:
    the applicant identity is verified,
    the risk score is acceptable,
    the requested amount is within policy.

  Approving the application:
    records the approval,
    notifies the applicant,
    schedules disbursement.

  If notification fails:
    approval still remains valid,
    notification is retried later.
```

### RIF sketch

```text
intent ApproveLoanApplication

subject:
  application: LoanApplication

requires:
  application.identity.status is Verified
  Risk.score(application) is acceptable
  application.requested_amount is within LendingPolicy

steps:
  1. Record approval
     call: ApprovalLedger.record(application)
     output: approval: ApprovalRecord
     changes: ApprovalLedger

  2. Notify applicant
     call: NotificationService.send_approval(application.applicant, approval)
     external call: NotificationService
     may fail with: NotificationFailed

  3. Schedule disbursement
     call: DisbursementQueue.schedule(application, approval)
     changes: DisbursementQueue

failure behavior:
  if notification fails:
    record NotificationRetryNeeded
    continue

guarantees:
  if this intent succeeds:
    approval exists
    disbursement is scheduled

  if notification fails:
    approval remains valid
    notification retry is scheduled
```

## Example 5: Trusted Capsule

### Human-facing spec

```text
trusted capsule Fast image resize:
  reason:
    uses platform-specific SIMD instructions

  allowed to:
    read input image memory
    write output image memory

  must guarantee:
    does not read outside the input image
    does not write outside the output image
    does not keep references after it returns
    produces a valid image buffer
```

### Compiler-facing sketch

```text
foreign unsafe operation FastImageResize

inputs:
  input: Read<ImageBuffer>
  output: Change<ImageBuffer>

effects:
  read<InputBuffer>
  write<OutputBuffer>
  unsafe<platform_simd>

obligations:
  no_out_of_bounds_read
  no_out_of_bounds_write
  no_reference_escape
  output_validity
```

## Example 6: Compiler Pass in RIF

```text
intent CheckDeclaredFailuresHandled

subject:
  intent: Intent

steps:
  1. Find fallible steps
     call: Intent.steps_where(step.may_fail is not empty)
     output: fallible_steps: List<Step>

  2. For each fallible step, find declared failures
     output: declared_failures

  3. Check handling
     for each failure in declared_failures:
       require intent handles failure
       or intent returns failure
       or intent marks failure impossible with proof

failure behavior:
  if a declared failure is not handled:
    emit diagnostic UnhandledFailure
    stop checking this intent

guarantees:
  every declared failure is handled, returned, or proven impossible
```



---

<!-- Source: 12-open-questions.md -->

# Open Questions

This file records unresolved design choices.

## 1. How strict should RSL be?

Options:

```text
A. Very strict controlled English, easy to parse.
B. Moderately flexible English with deterministic phrase matching.
C. LLM-assisted loose English with RIF review and strict checker.
```

Recommended path:

```text
Start with A.
Add B gradually.
Allow C only as an authoring assistant, never as final authority.
```

## 2. What is the exact RIF syntax?

RIF could be:

```text
indentation-based Markdown-like text
YAML-like structured text
S-expression-like normalized format
JSON/TOML plus explanation layer
projectional-editor-only model
```

Recommended path:

```text
Use indentation-based Markdown-like RIF for the prototype.
Also provide JSON serialization for EIG-Core.
```

## 3. How formal should contracts be initially?

Options:

```text
A. Plain text contracts, checked only for reference validity.
B. Expression language with static checks.
C. SMT-backed formal constraints.
D. Proof-assistant-level proofs.
```

Recommended path:

```text
Start with B for simple expressions.
Add C for selected domains.
Do not begin with D.
```

## 4. What is the first backend?

Options:

```text
custom bytecode interpreter
Wasm backend
MLIR/LLVM backend
workflow engine backend
Rust transpilation as temporary experiment
```

Recommended path:

```text
Start with custom bytecode interpreter for process semantics.
Add Wasm for pure computations.
Consider MLIR/LLVM later for native performance.
Avoid Rust/Python/JS transpilation as the conceptual execution model.
```

## 5. How should persistent state map to ownership?

In-memory values can use ownership and borrow-like permissions.
Persistent entities require a different backend mechanism.

Possible lowerings:

```text
transaction locks
optimistic version checks
actor mailbox ownership
event-sourced command tokens
linear write capabilities
```

Open question:

```text
Should EIGL expose these as backend policies, or should they be part of the core effect system?
```

## 6. What is the minimum trusted kernel?

Candidate trusted kernel responsibilities:

```text
EIG-Core schema validation
reference resolution
type checking
permission checking
effect checking
failure completeness checking
contract/proof-object checking
backend obligation checking
```

Open question:

```text
Which of these can be generated safely, and which must remain manually audited?
```

## 7. Should RIF be editable by non-programmers?

RSL is clearly for non-programmers. RIF is more explicit.

Open question:

```text
Should non-programmers edit RIF directly, or only review it through views and explanations?
```

Likely answer:

```text
RIF should be readable by non-programmers but primarily edited by expert users, LLMs, or guided tools.
```

## 8. How should ambiguity be represented?

Possibilities:

```text
unresolved question nodes in RIF
compiler diagnostics only
interactive editor prompts
LLM clarification dialogs
```

Recommended path:

```text
Represent ambiguities as explicit unresolved question objects in RIF.
Compilation fails until resolved.
```

## 9. How should visual edits be validated?

A visual edit creates a graph patch.

Open questions:

```text
What patch format should be used?
How are partial edits represented?
How does the editor display invalid intermediate states?
Can an invalid graph be temporarily edited before being accepted?
```

Recommended path:

```text
Allow transient invalid editor states, but only accept valid graph patches into EIG-Core.
```

## 10. How should LLMs interact with the system?

Possible interfaces:

```text
RSL authoring
RIF patch generation
EIG-Core patch generation
explanation generation
ambiguity clarification
test generation
compiler-rule drafting
```

Recommended path:

```text
LLMs propose RIF/EIG-Core patches.
The checker validates patches.
LLMs explain validated RIF/EIG-Core, not raw guesses.
```

## 11. How close to English should the final surface be?

The tension:

```text
more English -> easier for humans, more ambiguity
more structure -> easier for compilers, less natural
```

Likely solution:

```text
Use guided controlled English with editor support, phrase dictionaries, and progressive disclosure.
```

## 12. How should compiler generation be bootstrapped?

Open questions:

```text
What seed language should be used?
How large should the seed compiler be?
When is self-hosting considered achieved?
Should the backend be self-hosted early or late?
```

Recommended path:

```text
Seed compiler in Rust or Python for speed of development.
Trusted checker in Rust or another systems language for auditability.
Self-host RIF compiler before RSL compiler.
Backend self-hosting comes later.
```

## 13. What should the first real domain be?

Candidates:

```text
commerce workflows
access control
password reset/authentication
data pipelines
embedded device state machines
ML model pipelines
```

Recommended first domain:

```text
commerce workflows or access control
```

Reason:

```text
They naturally demonstrate state transitions, effects, failures, permissions, compensation, and visual explanations.
```

## 14. How should performance claims be validated?

Needed benchmarks:

```text
pure computation
state-machine execution
workflow orchestration
data processing
allocation-heavy object construction
parallel independent steps
foreign/trusted capsule calls
```

Open question:

```text
At what point can EIGL honestly claim Rust-like performance for a subset?
```

Likely answer:

```text
Only after a native backend and cost model are implemented and benchmarked.
```

## 15. What are the formal semantics?

Eventually EIGL needs formal definitions for:

```text
values
types
ownership/permissions
lifetimes/effect scopes
state
resources
time
failure
contracts
refinement
lowering correctness
```

Recommended path:

```text
Start with operational semantics for a small core.
Add type/permission/effect soundness proofs for the safe subset.
```

