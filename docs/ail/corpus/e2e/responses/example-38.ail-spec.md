# Generic Runtime AIL-Spec Example

The application Runtime Tickets manages simple runtime checks.

A Ticket has:

- id: Text
- priority: State<Low, High>
- status: State<Open, Closed>

A SupportTicket has:

- priority: State<Low, High>

Action: Prioritize ticket.

When a support agent prioritizes a ticket:

- the system requires the ticket to exist
- the system requires the ticket priority not to be High
- the system changes the ticket priority to High
- the system guarantees high priority tickets are handled first
- the system records a trace event named TicketPrioritizedScenario038
