app InvoiceWorkflow

things:
  thing Invoice
    field status: State<Draft, Paid, Free>
    field total: Int

intent ClassifyInvoice

subject:
  invoice: Invoice

steps:
  1. Classify invoice
     when: invoice.total > 0
     invoke: MarkPaid
     otherwise invoke: MarkFree

intent MarkPaid

subject:
  invoice: Invoice

steps:
  1. Mark invoice paid
     set: invoice.status = Paid

intent MarkFree

subject:
  invoice: Invoice

steps:
  1. Mark invoice free
     set: invoice.status = Free

guarantees:
  if this intent succeeds:
    invoice.status is Paid or invoice.status is Free
