# Secret Output Refund Tool AIL-Spec Example

Tool: Refund customer payment.

The AI Agent may request Refund customer payment when:

- the order exists

The tool needs:

- payment token: Secret<Text>

The tool produces:

- payment token: Secret<Text>

The tool can:

- call PaymentProvider.refund

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
