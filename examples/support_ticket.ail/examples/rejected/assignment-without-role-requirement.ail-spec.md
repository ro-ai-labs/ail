# Assignment Without Role Requirement AIL-Spec Example

The application Broken Assignment Tickets manages invalid ticket assignment
examples.

The application has these users:

- Customer
- Support agent
- Support manager

A User has:

- id: Text
- role: State<Customer, SupportAgent, SupportManager>

A Ticket has:

- id: Text
- status: State<New, Open, Assigned>
- assignee: Option<User>
- public_updates: List<Text>

The application shows:

- a customer-visible ticket history that includes public updates

Action: Assign ticket.

When a support agent assigns a ticket:

- the system requires the ticket to exist
- the system requires the ticket status to be New or Open
- the system changes the ticket assignee
- the system changes the status to Assigned
- the system records a public update
- the system records a trace event named TicketAssigned
