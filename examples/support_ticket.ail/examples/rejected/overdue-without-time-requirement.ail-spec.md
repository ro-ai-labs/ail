# Overdue Without Time Requirement AIL-Spec Example

The application Broken Overdue Tickets manages invalid overdue-ticket examples.

A Ticket has:

- id: Text
- status: State<New, Open, Assigned, Overdue>
- due_at: Time
- public_updates: List<Text>

The application shows:

- a customer-visible ticket history that includes public updates

Action: Marks Overdue Tickets.

When the scheduler marks overdue tickets:

- the system reads tickets whose status is New, Open, or Assigned
- the system changes the ticket status to Overdue
- the system records a public update
- the system records a trace event named TicketOverdue
