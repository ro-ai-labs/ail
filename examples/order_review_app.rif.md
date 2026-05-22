app OrderReview

things:
  thing Order
    field status: State<Draft, Approved, Rejected>
    field review_note: State<None, ApprovedByPolicy, RejectedByPolicy>

intent ReviewOrder

subject:
  order: Order

inputs:
  approved: Bool

steps:
  1. Review order
     when: approved is true
     invoke: ApproveOrder(should_approve = approved)
     otherwise invoke: RejectOrder(should_approve = approved)

returns:
  decision: order.status
  note: order.review_note

intent ApproveOrder

subject:
  order: Order

inputs:
  should_approve: Bool

requires:
  should_approve is true

steps:
  1. Approve order
     set: order.status = Approved
  2. Mark approval note
     set: order.review_note = ApprovedByPolicy

intent RejectOrder

subject:
  order: Order

inputs:
  should_approve: Bool

requires:
  should_approve is false

steps:
  1. Reject order
     set: order.status = Rejected
  2. Mark rejection note
     set: order.review_note = RejectedByPolicy
