# Notification Without Responder Pager AIL-Spec Example

The application Broken Incident Notification manages invalid responder
notification examples.

An Incident has:

- id: Text
- status: State<Declared, Mitigating>
- severity: State<Sev1, Sev2, Sev3>

Action: Notify incident responder.

When a responder notification is sent:

- the system requires the incident to exist
- the system records a notification audit entry
- the system records a trace event named IncidentResponderNotified
