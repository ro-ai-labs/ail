# Route Missing Permission AIL-Spec Example

The application Broken Incident Route manages invalid route permission
examples.

An Incident has:

- id: Text
- status: State<Declared, Mitigating>
- severity: State<Sev1, Sev2, Sev3>
- timeline: List<Text>

Route: Incident command center.

The route path is:

- /incidents/:incident_id/command

The route reads:

- Incident.status
- Incident.severity
- Incident.timeline

The route records trace:

- IncidentCommandCenterViewed
