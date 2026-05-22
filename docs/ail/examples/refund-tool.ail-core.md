# Refund Tool AIL-Core Example

```text
package refund-tool
profile AgentTool

node Tool RefundCustomerPayment
  purpose: "Refund customer payment"
  Provenance: spec:refund-tool

node Input order_id type Text
node Input refund_amount type Money
node Input reason type Text
node Input payment_token type Secret<Text>
node Output refund_id type Text

node Permission RequesterMayCreateRefunds
edge RefundCustomerPayment requires Permission.RequesterMayCreateRefunds

node Rule OrderExists
node Rule PaymentCaptured
node Rule RefundAmountWithinCapturedAmount
node Approval ManagerApprovalOverUsd500

edge RefundCustomerPayment requires Rule.OrderExists
edge RefundCustomerPayment requires Rule.PaymentCaptured
edge RefundCustomerPayment requires Rule.RefundAmountWithinCapturedAmount
edge RefundCustomerPayment requires-approval Approval.ManagerApprovalOverUsd500

node Effect PaymentProviderRefundCall
  target: PaymentProvider.refund
edge RefundCustomerPayment calls PaymentProviderRefundCall

node Effect RefundLedgerWrite
  target: RefundLedger
edge RefundCustomerPayment writes RefundLedgerWrite

node Secret PaymentToken
edge PaymentToken protects input:payment_token
edge RefundCustomerPayment protects Secret.PaymentToken

node Failure ProviderRejected
edge RefundCustomerPayment may-fail-with Failure.ProviderRejected
edge Failure.ProviderRejected writes Effect.HumanReviewTask

node Guarantee RefundNotGreaterThanCaptured
node Guarantee PaymentTokenRedacted
node Guarantee ExternalCallAudited
edge RefundCustomerPayment guarantees Guarantee.RefundNotGreaterThanCaptured
edge RefundCustomerPayment guarantees Guarantee.PaymentTokenRedacted
edge RefundCustomerPayment guarantees Guarantee.ExternalCallAudited

node Trace RefundApprovalRequested
node Trace RefundProviderCalled
node Trace RefundLedgerWritten
```
