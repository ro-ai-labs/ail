app CompoundInvoices

things:
  thing Invoice
    field status: State<Draft, Finalized>
    field subtotal: Int
    field tax: Int
    field discount: Int
    field total: Int

intent FinalizeInvoice

subject:
  invoice: Invoice

requires:
  not (invoice.status is Finalized) and (invoice.subtotal > 0 or invoice.tax > 0)

steps:
  1. Set payable total
     when: not (invoice.tax > 0 and invoice.discount > 0)
     set: invoice.total = 1
     changes: invoice.total

  2. Finalize invoice
     set: invoice.status = Finalized
     changes: invoice.status

guarantees:
  if this intent succeeds:
    not (invoice.status is Draft) and invoice.total > 0
