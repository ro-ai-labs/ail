# UI Workflow AIL-Spec Example

The application Support UI manages ticket intake, operational dashboards, and
provider handoff workflows.

A Ticket has:

- id: Text
- title: Text
- status: State<New, Open, Overdue, Refunded>

Action: Create ticket.

When a requester submits a ticket:

- the system requires title
- the system creates Ticket.status
- the system records a trace event named TicketCreated

Action: Provider call.

When a provider call starts:

- the system requires Manager approval
- the system records a trace event named ProviderCallStarted

Route: Ticket detail.

The route path is:

- /tickets/:ticket_id

The route reads:

- Ticket.id
- Ticket.status

The route requires permission:

- requester may read ticket

The route records trace:

- RouteTicketDetailViewed

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

Dashboard: Support manager dashboard.

The dashboard reads:

- Ticket.status

The dashboard requires permission:

- Support manager may view overdue tickets

The dashboard filters:

- status is Overdue

The dashboard records trace:

- DashboardViewed

Workflow: Refund approval.

The workflow steps are:

- Request
- Manager approval
- Provider call

The workflow blocks:

- Provider call before Manager approval

The workflow records trace:

- RefundApprovalWorkflowViewed
