# UI Destructive Confirmation Accepted Fixture

The application Support UI manages destructive ticket operations.

Action: Delete ticket.

When a support manager deletes a ticket:

- the system requires Support manager permission
- the system deletes Ticket
- the system records a trace event named TicketDeleted

Form: Delete ticket.

The form calls action:

- DeleteTicket

The form fields are:

- ticket id: Text

The form requires confirmation:

- reviewer confirms ticket deletion

The form accessibility is:

- delete ticket button is named

