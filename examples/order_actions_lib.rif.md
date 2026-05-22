app OrderLibrary

exports:
  export thing Order
  export collection orders
  export intent ArchiveOrder

things:
  thing Order
    field status: State<Open, Archived>

collections:
  collection orders: Order

intent ArchiveOrder

subject:
  order: Order

steps:
  1. Archive order
     set: order.status = Archived
