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
