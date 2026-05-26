# Incident Policy AIL-Spec Example

The application Incident Policy manages severity, escalation, and service
objectives.

A Service has:

- id: Text
- name: Text
- tier: State<Critical, Standard, Internal>
- owner: Text

An EscalationPolicy has:

- id: Text
- service: Service
- severity: State<Sev1, Sev2, Sev3>
- response_minutes: Int
- require commander: Bool

Failure PolicyViolation happens when an incident action violates the service
escalation policy:

- the system changes no incident state
- the caller sees "Policy violation"
- the trace records IncidentPolicyViolation
