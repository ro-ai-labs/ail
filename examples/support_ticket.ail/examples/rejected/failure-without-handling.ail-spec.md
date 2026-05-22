# Failure Without Handling AIL-Spec Example

The application Broken Tickets manages invalid ticket examples.

A Ticket has:

- id: Text
- status: State<Open, Closed>

Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- if NotFound
- the system changes the ticket status to Closed
- the system records a trace event named TicketClosed

Failure NotFound happens when a ticket id does not match a stored ticket:

- the trace records TicketNotFound
