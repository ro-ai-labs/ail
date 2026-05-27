# Minimal Refund Tool AIL-Spec Example

Tool: Refund customer payment.

The AI Agent may request Refund customer payment when:

- the order exists
- the payment was captured

The tool needs:

- order id: Text
- refund amount: Money
- payment token: Secret<Text>

The tool produces:

- refund id: Text

The tool can:

- read the order
- call PaymentProvider.refund
- write a RefundLedger entry

The tool must not:

- reveal the payment token

The tool requires permission:

- requester may create refunds

The tool requires approval:

- manager approval when the refund amount is over USD 500

The tool records:

- RefundCustomerPaymentRequested

The tool guarantees:

- payment token is redacted from all agent-visible output

Failure ProviderRejected happens when PaymentProvider rejects the refund:

- the system records failure ProviderRejected
- a human review task is created
- the trace records RefundProviderRejected
