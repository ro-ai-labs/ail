# Close Ticket Minimal Accepted AIL-Spec Example

The application Accepted Tickets manages accepted ticket examples.

A Ticket has:

- id: Text
- status: State<Open, Closed>

Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- if NotFound
- the system changes the ticket status to Closed
- the system guarantees closed tickets do not appear in the open ticket queue
- the system records a trace event named TicketClosed

Failure NotFound happens when a ticket id does not match a stored ticket:

- the caller sees "Ticket not found"
- the trace records TicketNotFound
