app TicketRouting

things:
  thing Ticket
    field id: Id<Ticket>
    field priority: State<Critical, Normal>
    field route: State<Unrouted, Escalated, Queued>

  thing Escalation
    field id: Id<Escalation>

  thing WorkItem
    field id: Id<WorkItem>

operations:
  operation PagerDuty.trigger(ticket_id: Id<Ticket>) -> Escalation
    changes: EscalationQueue
    external call: PagerDuty
    may fail with: PagerDutyUnavailable

  operation WorkQueue.enqueue(ticket_id: Id<Ticket>) -> WorkItem
    changes: WorkQueue

intent RouteTicket

subject:
  ticket: Ticket

requires:
  ticket.route is Unrouted

steps:
  1. Escalate critical ticket
     when: ticket.priority is Critical
     call: PagerDuty.trigger(ticket.id)
     output: escalation: Escalation
     set: ticket.route = Escalated
     changes: ticket.route
     may fail with: PagerDutyUnavailable

  2. Queue normal ticket
     when: ticket.priority is not Critical
     call: WorkQueue.enqueue(ticket.id)
     output: work_item: WorkItem
     set: ticket.route = Queued
     changes: ticket.route

failure behavior:
  if pager duty fails:
    stop with PagerDutyUnavailable

guarantees:
  if this intent succeeds:
    ticket.route exists

