# Composed Support AIL-Spec Example

The application Composed Support Tickets manages support tickets with shared users.

A Ticket has:

- id: Text
- customer: Shared.User
- status: State<Open, Closed>

Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- the system changes the ticket status to Closed
- the system guarantees closed tickets are hidden from the active queue
- the system records a trace event named TicketClosedScenario018
