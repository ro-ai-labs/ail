app Commerce

things:
  thing Order
    field status: State<Draft, Confirmed, PaidAwaitingFulfillmentRetry>
    field items: List<OrderItem>
    field total: Int
    field payment_method: PaymentMethod

  thing OrderItem
    field id: Id<OrderItem>

  thing PaymentMethod
    field id: Id<PaymentMethod>

  thing PaymentLedger
    field id: Id<PaymentLedger>

  thing ShipmentQueue
    field id: Id<ShipmentQueue>

  thing InventoryReservation
    field id: Id<InventoryReservation>

  thing PaymentCapture
    field id: Id<PaymentCapture>

  thing ShipmentRequest
    field id: Id<ShipmentRequest>

phrases:
  phrase "reserve inventory":
    means Inventory.reserve(order.items)
    produces reservation: InventoryReservation
    changes Inventory
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

intent Confirm order:
  subject:
    order: Order

  requires:
    order.status is Draft
    order.items.count > 0

  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on payment failure -> PaymentFailed
  on shipping failure -> PaidAwaitingFulfillmentRetry
