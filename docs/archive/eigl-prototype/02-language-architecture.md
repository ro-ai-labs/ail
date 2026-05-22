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
