app ParallelInvoice

things:
  thing Invoice
    field status: State<Draft, Free>
    field total: Int
    field payment_status: State<Pending, Captured>
    field shipping_status: State<Pending, Queued>
    field archival_status: State<Active, Archived>

intent ProcessInvoice

subject:
  invoice: Invoice

steps:
  1. Process invoice
     when: invoice.total > 0
     parallel invoke: CapturePayment, ReserveShipment
     otherwise parallel invoke: MarkFree, ArchiveInvoice

intent CapturePayment

subject:
  invoice: Invoice

steps:
  1. Capture payment
     set: invoice.payment_status = Captured

intent ReserveShipment

subject:
  invoice: Invoice

steps:
  1. Reserve shipment
     set: invoice.shipping_status = Queued

intent MarkFree

subject:
  invoice: Invoice

steps:
  1. Mark invoice free
     set: invoice.status = Free

intent ArchiveInvoice

subject:
  invoice: Invoice

steps:
  1. Archive invoice
     set: invoice.archival_status = Archived
