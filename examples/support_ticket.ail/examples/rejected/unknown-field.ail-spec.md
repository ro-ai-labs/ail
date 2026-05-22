# Unknown Field AIL-Spec Example

The application Broken Tickets manages invalid ticket examples.

A Ticket has:

- id: Text
- status: State<Open, Closed>

Action: Archive ticket.

When a support agent archives a ticket:

- the system reads ticket owner email
- the system changes ticket archive code to Archived
- the system records a trace event named TicketArchived
