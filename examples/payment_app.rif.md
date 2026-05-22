app Payments

things:
  thing Invoice
    field id: Id<Invoice>
    field subtotal: Money
    field tax: Money
    field discount_rate: Decimal
    field total: Money

operations:
  operation Payment.capture(amount: Money) -> Unit

intent CaptureInvoice

subject:
  invoice: Invoice

steps:
  1. Set subtotal
     set: invoice.subtotal = USD:20.00
     changes: invoice.subtotal

  2. Set tax
     set: invoice.tax = USD:1.50
     changes: invoice.tax

  3. Set discount rate
     set: invoice.discount_rate = 0.10
     changes: invoice.discount_rate

  4. Compute total
     compute: invoice.total = invoice.subtotal + invoice.tax - invoice.subtotal * invoice.discount_rate
     changes: invoice.total

  5. Capture payment
     call: Payment.capture(invoice.total)

guarantees:
  if this intent succeeds:
    invoice.total == USD:19.5
