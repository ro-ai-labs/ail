app TriageDesk

types:
  enum Priority
    value Low
    value Normal
    value Critical

things:
  thing Ticket
    field id: Id<Ticket>
    field priority: Priority
    field status: State<Open, Routed>

operations:
  operation Routing.assign(priority: Priority) -> Unit

intent EscalateTicket

subject:
  ticket: Ticket

steps:
  1. Mark priority
     set: ticket.priority = Critical
     changes: ticket.priority

  2. Assign route
     call: Routing.assign(Critical)

  3. Mark routed
     set: ticket.status = Routed
     changes: ticket.status

guarantees:
  if this intent succeeds:
    ticket.status is Routed
