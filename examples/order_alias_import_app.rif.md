imports:
  import order_actions_lib.rif.md as Shared
  import order_actions_lib.rif.md as Ops

app Orders
module Orders.Main

things:
  thing Order
    field status: State<Open, Archived>

intent ProcessOrder

subject:
  order: Order

steps:
  1. Open order
     set: order.status = Open
