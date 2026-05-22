intent BadParallelUpdate

subject:
  order: Order

steps:
  schedule: unordered

  1. Mark order paid
     set: order.status = Paid
     changes: order.status

  2. Mark order cancelled
     set: order.status = Cancelled
     changes: order.status

guarantees:
  if this intent succeeds:
    order.status is Paid

