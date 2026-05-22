# Unknown Field Type AIL-Spec Example

The application Broken Tickets manages invalid ticket examples.

A Ticket has:

- id: Text
- metadata: MysteryBox

Action: Inspect ticket.

When a support agent inspects a ticket:

- the system requires the ticket to exist
- the system reads ticket metadata
- the system records a trace event named TicketInspected
