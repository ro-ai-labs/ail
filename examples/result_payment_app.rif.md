app ResultPayments

things:
  thing Payment
    field confirmation: Result<Int, Text>

operations:
  operation Payment.record(result: Result<Int, Text>) -> Unit

intent CapturePayment

subject:
  payment: Payment

steps:
  1. Store success
     set: payment.confirmation = Success(200)
     changes: payment.confirmation

  2. Record failure case
     call: Payment.record(Failure("timeout"))

guarantees:
  if this intent succeeds:
    payment.confirmation == Success(200)
