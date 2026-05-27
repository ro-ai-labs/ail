# Resolve Without Mitigating Status AIL-Spec Example

The application Broken Incident Resolution manages invalid incident lifecycle
examples.

An Incident has:

- id: Text
- status: State<Declared, Mitigating, Resolved>
- timeline: List<Text>

Action: Resolve incident.

When an incident commander resolves an incident:

- the system requires the incident to exist
- the system changes the incident status to Resolved
- the system records a public timeline update
- the system records a trace event named IncidentResolved
