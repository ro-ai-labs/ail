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
     reads: order.items
     changes: Inventory
     compensation: Inventory.release(reservation)

  2. Capture payment
     call: PaymentGateway.capture(order.payment_method, order.total)
     output: payment: PaymentCapture
     reads: order.payment_method
     reads: order.total
     changes: PaymentLedger
     external call: PaymentGateway
     may fail with: PaymentFailed

  3. Create shipment request
     call: Shipping.create_request(order, reservation)
     output: shipment: ShipmentRequest
     reads: order
     reads: reservation
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
    payment exists
    shipment exists

