app Billing

things:
  thing Invoice
    field id: Id<Invoice>
    field subtotal: Int
    field tax: Int
    field total: Int
    field status: State<Draft, Finalized>

intent FinalizeInvoice

subject:
  invoice: Invoice

requires:
  invoice.status is Draft
  invoice.subtotal > 0
  invoice.tax > 0

steps:
  1. Calculate invoice total
     compute: invoice.total = invoice.subtotal + invoice.tax
     changes: invoice.total

  2. Finalize invoice
     when: invoice.total > 0
     set: invoice.status = Finalized
     changes: invoice.status

guarantees:
  if this intent succeeds:
    invoice.total > 0
    invoice.status is Finalized
