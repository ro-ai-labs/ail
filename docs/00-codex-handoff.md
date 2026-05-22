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
