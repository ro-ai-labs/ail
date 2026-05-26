# Secret Access AIL-Spec Example

The application Secret Access manages restricted internal notes.

A Requester has:

- id: Text
- role: State<Customer, SupportAgent, SupportManager>

A Ticket has:

- id: Text
- internal notes: Secret<List<Text>>

Action: View internal notes.

When a requester views internal notes:

- the system requires the ticket to exist
- the system requires the requester role to be SupportAgent or SupportManager
- the system reads ticket internal notes
- the system does not reveal internal notes to the customer
- the system guarantees only support staff can see internal notes
- the system records a trace event named InternalNotesViewedScenario078

Failure PermissionDenied happens when requester role is not SupportAgent or SupportManager:

- the caller sees "Permission denied"
- the trace records InternalNotesDeniedScenario078
