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
