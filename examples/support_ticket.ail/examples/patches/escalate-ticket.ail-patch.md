patch Add ticket escalation

target:
  app Support Tickets

change:
  add field Ticket.escalation reason: Text
  add view an escalation queue for support managers
  add action Escalate ticket
  when a support agent escalates a ticket:
    requires the ticket to exist
    changes the ticket escalation reason
    changes the ticket status to Overdue
    guarantees escalated tickets appear in the escalation queue
    records trace TicketEscalated
