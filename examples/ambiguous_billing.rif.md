app AmbiguousBilling

things:
  thing Invoice
    field status: State<Open, Closed>
    field billing_mode: Text

intent ResolveBilling

subject:
  invoice: Invoice

unresolved questions:
  - phrase "refund payment" has multiple meanings:
      1. PaymentGateway.refund_full
      2. StoreCredit.issue
