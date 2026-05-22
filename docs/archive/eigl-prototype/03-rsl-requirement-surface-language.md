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
