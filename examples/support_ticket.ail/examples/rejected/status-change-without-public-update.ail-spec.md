# Status Change Without Public Update AIL-Spec Example

The application Broken Public Update Tickets manages invalid ticket status
examples.

A Ticket has:

- id: Text
- status: State<Open, Closed>
- public_updates: List<Text>

The application shows:

- a customer-visible ticket history that includes public updates

Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- the system requires the ticket status not to be Closed
- the system changes the ticket status to Closed
- the system records a trace event named TicketClosed
