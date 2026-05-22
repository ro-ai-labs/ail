app BranchInvoice

things:
  thing Invoice
    field total: Int
    field status: State<Paid, Free>

intent ClassifyInvoice

subject:
  invoice: Invoice

steps:
  1. Classify invoice
     when: invoice.total > 0
     set: invoice.status = Paid
     otherwise set: invoice.status = Free

guarantees:
  if this intent succeeds:
    invoice.status is Paid or invoice.status is Free
