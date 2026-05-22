# Support Ticket AIL-Core Example

This pseudo-IR corresponds to `support-ticket.ail-spec.md`. It is readable
rather than a final serialization format.

```text
package support-ticket
profile Application

node Application support-ticket
  label: "Support Tickets"
  purpose: "Manage customer support tickets"
  Provenance: spec:1

node Thing Ticket
node Field Ticket.id type Text
node Field Ticket.title type Text
node Field Ticket.status type State<New, Open, Assigned, Closed, Overdue>
node Field Ticket.customer type User
node Field Ticket.assignee type Option<User>
node Field Ticket.public_updates type List<Text>
node Field Ticket.internal_notes type Secret<List<Text>>

node Thing User
node Field User.id type Text
node Field User.role type State<Customer, SupportAgent, SupportManager>
node Field User.email type Text

node Action CreateTicket
edge CreateTicket writes Ticket
edge CreateTicket guarantees Guarantee.InternalNotesEmpty
edge CreateTicket records Trace.TicketCreated
edge CreateTicket has-provenance spec:create-ticket

node Action AssignTicket
edge AssignTicket requires Rule.TicketExists
edge AssignTicket requires Rule.AssigneeIsSupportStaff
edge AssignTicket reads Ticket.status
edge AssignTicket writes Ticket.assignee
edge AssignTicket writes Ticket.status
edge AssignTicket grants Permission.SupportStaffReadInternalNotes
edge AssignTicket guarantees Guarantee.AssignedTicketHasAssignee
edge AssignTicket records Trace.TicketAssigned
edge AssignTicket has-provenance spec:assign-ticket

node Action CloseTicket
edge CloseTicket requires Rule.TicketExists
edge CloseTicket requires Rule.TicketNotClosed
edge CloseTicket reads Ticket.status
edge CloseTicket writes Ticket.status
edge CloseTicket writes Ticket.public_updates
edge CloseTicket protects Secret.TicketInternalNotes
edge CloseTicket guarantees Guarantee.InternalNotesNotCustomerVisible
edge CloseTicket guarantees Guarantee.ClosedTicketNotInOpenQueue
edge CloseTicket records Trace.TicketClosed
edge CloseTicket may-fail-with Failure.NotFound
edge CloseTicket may-fail-with Failure.PermissionDenied
edge CloseTicket has-provenance spec:close-ticket

node View OverdueTickets
edge OverdueTickets reads Ticket.status
edge OverdueTickets requires Rule.UserIsSupportManager

node Permission SupportStaffReadInternalNotes
edge SupportStaffReadInternalNotes allows-read Ticket.internal_notes
edge SupportStaffReadInternalNotes requires Rule.UserIsSupportStaff

node Guarantee InternalNotesNotCustomerVisible
node Guarantee ClosedTicketNotInOpenQueue
node Guarantee AssignedTicketHasAssignee
node Guarantee InternalNotesEmpty

node Failure NotFound
node Failure PermissionDenied

node Trace TicketCreated
node Trace TicketAssigned
node Trace TicketClosed
node Trace TicketOverdue
```
