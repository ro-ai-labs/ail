# Postmortem Without Resolved Status AIL-Spec Example

The application Broken Incident Postmortem manages invalid postmortem lifecycle
examples.

An Incident has:

- id: Text
- status: State<Mitigating, Resolved, Postmortem>

Action: Start postmortem.

When a service owner starts postmortem review:

- the system requires the incident to exist
- the system changes the incident status to Postmortem
- the system records a trace event named IncidentPostmortemStarted
