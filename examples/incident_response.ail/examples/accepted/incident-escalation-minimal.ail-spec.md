# Incident Escalation Minimal Accepted AIL-Spec Example

The application Accepted Incident Escalation manages incident escalation,
notification audit, and lifecycle review examples.

An Incident has:

- id: Text
- status: State<Declared, Mitigating, Resolved, Postmortem>
- severity: State<Sev1, Sev2, Sev3>
- timeline: List<Text>
- private notes: Secret<List<Text>>

The application shows:

- a public timeline that never includes private notes

Action: Escalate incident.

When an incident commander escalates an incident:

- the system requires the incident to exist
- the system requires the incident severity to be Sev1 or Sev2
- the system requires the escalation policy to require commander review
- the system changes the incident status to Mitigating
- the system records a notification audit entry
- the system guarantees public timeline subscribers can see the escalation
- the system does not reveal private notes
- the system records a trace event named IncidentEscalated

Action: Notify incident responder.

When a responder notification is sent:

- the system requires the incident to exist
- the system requires responder pager
- the system records a notification audit entry
- the system records a trace event named IncidentResponderNotified

Action: Resolve incident.

When an incident commander resolves an incident:

- the system requires the incident to exist
- the system requires the incident status to be Mitigating
- the system changes the incident status to Resolved
- the system records a public timeline update
- the system records a trace event named IncidentResolved

Action: Start postmortem.

When a service owner starts postmortem review:

- the system requires the incident status to be Resolved
- the system changes the incident status to Postmortem
- the system records a trace event named IncidentPostmortemStarted
