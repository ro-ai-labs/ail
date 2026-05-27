# Permission Without Rule Refund Tool AIL-Spec Example

Tool: Refund customer payment.

The AI Agent may request Refund customer payment when:

- the order exists
- the requester has permission to create refunds

The tool needs:

- payment token: Secret<Text>

The tool produces:

- refund id: Text

The tool can:

- call PaymentProvider.refund
- write a RefundLedger entry

The tool must not:

- reveal the payment token

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
