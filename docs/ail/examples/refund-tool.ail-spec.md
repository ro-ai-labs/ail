# Refund Tool AIL-Spec Example

Tool: Refund customer payment.

The AI Agent may request Refund customer payment when:

- the order exists
- the payment was captured
- the refund amount is not greater than the captured amount

The tool needs:

- order id: Text
- refund amount: Money
- reason: Text
- payment token: Secret<Text>

The tool produces:

- refund id: Text

The tool can:

- read the order
- read the payment record
- call PaymentProvider.refund
- write a RefundLedger entry
- create a human review task when approval is required

The tool must not:

- reveal the payment token
- refund more than the captured amount

The tool requires permission:

- requester may create refunds

The tool requires approval:

- manager approval when the refund amount is over USD 500

The tool records:

- RefundCustomerPaymentRequested

If manager approval is required:

- the system creates an approval request
- the external refund call does not run until approval is granted
- the trace records RefundApprovalRequested

If PaymentProvider rejects the refund:

- the system records failure ProviderRejected
- the system writes no successful RefundLedger entry
- the customer is not notified automatically
- a human review task is created

The tool guarantees:

- Refund amount is less than or equal to the captured amount
- payment token is redacted from all agent-visible output
- every external call is represented in the audit trace
