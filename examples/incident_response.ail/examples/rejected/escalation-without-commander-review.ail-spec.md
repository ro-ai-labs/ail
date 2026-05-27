# Escalation Without Commander Review AIL-Spec Example

The application Broken Incident Escalation manages invalid escalation policy
examples.

An Incident has:

- id: Text
- status: State<Declared, Mitigating>
- severity: State<Sev1, Sev2, Sev3>
- private notes: Secret<List<Text>>

Action: Escalate incident.

When an incident commander escalates an incident:

- the system requires the incident to exist
- the system requires the incident severity to be Sev1 or Sev2
- the system changes the incident status to Mitigating
- the system records a notification audit entry
- the system does not reveal private notes
- the system records a trace event named IncidentEscalated
