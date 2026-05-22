app BrokenRuntimeContract

things:
  thing Invoice
    field total: Int

intent BreakContract

subject:
  invoice: Invoice

steps:
  1. Set invalid total
     set: invoice.total = 0
     changes: invoice.total

guarantees:
  if this intent succeeds:
    invoice.total > 0

