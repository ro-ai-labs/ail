# Approval Without Rule Refund Tool AIL-Spec Example

Tool: Refund customer payment.

The AI Agent may request Refund customer payment when:

- the order exists

The tool needs:

- payment token: Secret<Text>

The tool produces:

- refund id: Text

The tool can:

- call PaymentProvider.refund
- write a RefundLedger entry
- create a human review task when approval is required

The tool must not:

- reveal the payment token
- run without manager approval when the refund amount is over USD 500

The tool requires permission:

- requester may create refunds

The tool records:

- RefundCustomerPaymentRequested

The tool guarantees:

- payment token is redacted from all agent-visible output

Failure ProviderRejected happens when PaymentProvider rejects the refund:

- the system records failure ProviderRejected
- a human review task is created
- the trace records RefundProviderRejected
