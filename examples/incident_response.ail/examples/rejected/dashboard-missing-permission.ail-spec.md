# Dashboard Missing Permission AIL-Spec Example

The application Broken Incident Dashboard manages invalid dashboard permission
examples.

An Incident has:

- id: Text
- status: State<Declared, Mitigating, Resolved>
- severity: State<Sev1, Sev2, Sev3>
- service: Text

Dashboard: Service owner incident dashboard.

The dashboard reads:

- Incident.status
- Incident.severity
- Incident.service

The dashboard filters:

- status is not Resolved
- severity is Sev1 or Sev2

The dashboard records trace:

- ServiceOwnerIncidentDashboardViewed
