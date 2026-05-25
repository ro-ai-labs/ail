# Support Ticket AIL-Spec Example

The application Support Tickets manages customer support tickets, assignments,
updates, internal notes, and overdue-ticket review.

The application has these users:

- Customer
- Support agent
- Support manager

A User has:

- id: Text
- role: State<Customer, SupportAgent, SupportManager>
- email: Text

A Ticket has:

- id: Text
- title: Text
- status: State<New, Open, Assigned, Closed, Overdue>
- customer: User
- assignee: Option<User>
- created_at: Time
- due_at: Time
- public_updates: List<Text>
- internal notes: Secret<List<Text>>

The application shows:

- an open ticket queue for support agents
- an Overdue tickets view for support managers
- a customer-visible ticket history that includes public updates and never
  includes internal notes

Action: Create ticket.

When a customer creates a ticket:

- the system requires the customer id and title
- the system creates a Ticket with status New
- the system records the customer as the ticket customer
- the system records an initial public update
- the system guarantees internal notes are empty and secret
- the system records a trace event named TicketCreatedScenario091

Action: Assign ticket.

When a support agent assigns a ticket:

- the system requires the ticket to exist
- the system requires the ticket status to be New or Open
- the system requires the assignee role to be SupportAgent or SupportManager
- the system changes the ticket assignee
- the system changes the status to Assigned
- the system records a public update
- the system guarantees the assignee can see internal notes
- the system records a trace event named TicketAssignedScenario091

Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- the system requires the ticket status not to be Closed
- the system changes the ticket status to Closed
- the system records a public update
- the system does not reveal internal notes to the customer
- the system guarantees closed tickets do not appear in the open ticket queue
- the system records a trace event named TicketClosedScenario091

When the scheduler marks overdue tickets:

- the system reads tickets whose status is New, Open, or Assigned
- the system requires the current time to be later than due_at
- the system changes the ticket status to Overdue
- the system records a public update
- the system records a trace event named TicketOverdueScenario091

Failure NotFound happens when a ticket id does not match a stored ticket:

- the system changes no ticket data
- the caller sees "Ticket not found"
- the trace records TicketNotFoundScenario091

Failure PermissionDenied happens when a user tries to see internal notes without
support staff permission:

- the system reveals no secret value
- the caller sees "Permission denied"
- the trace records InternalNotesDeniedScenario091
