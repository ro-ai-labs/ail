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
