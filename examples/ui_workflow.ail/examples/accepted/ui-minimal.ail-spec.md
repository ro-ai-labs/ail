# UI Minimal Accepted Fixture

The application Support UI manages ticket intake.

Action: Create ticket.

When a requester submits a ticket:

- the system requires title
- the system records a trace event named TicketCreated

Form: Create ticket.

The form calls action:

- CreateTicket

The form fields are:

- title: Text

The form validates:

- title is not empty

If form validation fails:

- FormValidationFailed

The form accessibility is:

- title error is announced
