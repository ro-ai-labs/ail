# Incident Response AIL-Spec Example

The application Incident Response manages service incidents across intake,
assignment, escalation, notification, resolution, and postmortem review.

The application has these users:

- Responder
- Incident commander
- Service owner

An Incident has:

- id: Text
- title: Text
- status: State<Declared, Triaged, Mitigating, Resolved, Postmortem>
- severity: State<Sev1, Sev2, Sev3>
- service id: Text
- commander id: Text
- responder id: Text
- timeline: List<Text>
- private notes: Secret<List<Text>>

An IncidentUpdate has:

- id: Text
- incident: Incident
- author id: Text
- body: Text
- visibility: State<Public, Private>

The application shows:

- an active incident command view for incident commanders
- a responder queue for assigned responders
- a service-owner dashboard for unresolved Sev1 and Sev2 incidents
- a public timeline that never includes private notes

Action: Declare incident.

When a service owner declares an incident:

- the system requires title
- the system requires service
- the system creates Incident with status Declared
- the system records the declaring service owner in the timeline
- the system guarantees private notes are empty and secret
- the system records a trace event named IncidentDeclaredScenario115

Action: Assign responder.

When an incident commander assigns a responder:

- the system requires the incident to exist
- the system requires the incident status to be Declared or Triaged
- the system requires responder role to be Responder
- the system changes the incident responder
- the system changes the incident status to Triaged
- the system records a public timeline update
- the system records a trace event named IncidentResponderAssignedScenario115

Action: Escalate incident.

When an incident commander escalates an incident:

- the system requires the incident to exist
- the system requires the incident severity to be Sev1 or Sev2
- the system requires the escalation policy to require commander review
- the system changes the incident status to Mitigating
- the system records a notification audit entry
- the system guarantees public timeline subscribers can see the escalation
- the system does not reveal private notes
- the system records a trace event named IncidentEscalatedScenario115

Action: Notify incident responder.

When a responder notification is sent:

- the system requires the incident to exist
- the system requires responder pager
- the system records a notification audit entry
- the system records a trace event named IncidentResponderNotifiedScenario115

Action: Resolve incident.

When an incident commander resolves an incident:

- the system requires the incident to exist
- the system requires the incident status to be Mitigating
- the system changes the incident status to Resolved
- the system records a public timeline update
- the system records a trace event named IncidentResolvedScenario115

Action: Start postmortem.

When a service owner starts postmortem review:

- the system requires the incident status to be Resolved
- the system changes the incident status to Postmortem
- the system records a trace event named IncidentPostmortemStartedScenario115

Action: Complete incident lifecycle.

When an incident commander completes the incident lifecycle:

- the system records lifecycle review completion
- the system guarantees every lifecycle step has a timeline entry
- the system records a trace event named IncidentLifecycleCompletedScenario115

Route: Incident command center.

The route path is:

- /incidents/:incident_id/command

The route reads:

- Incident.status
- Incident.severity
- Incident.timeline

The route requires permission:

- incident commander may read incident command center

The route records trace:

- IncidentCommandCenterViewedScenario115

Form: Escalate incident.

The form calls action:

- EscalateIncident

The form fields are:

- incident id: Text
- severity: Text
- reason: Text

The form validates:

- incident id is not empty
- severity is Sev1 or Sev2

If form validation fails:

- IncidentEscalationValidationFailedScenario115

The form accessibility is:

- escalation errors are announced

Dashboard: Service owner incident dashboard.

The dashboard reads:

- Incident.status
- Incident.severity
- Incident.service

The dashboard requires permission:

- service owner may view service incidents

The dashboard filters:

- status is not Resolved
- severity is Sev1 or Sev2

The dashboard records trace:

- ServiceOwnerIncidentDashboardViewedScenario115

Workflow: Incident lifecycle.

The workflow steps are:

- Declare
- Assign responder
- Escalate
- Notify responder
- Resolve
- Postmortem

The workflow blocks:

- Notify responder before Escalate
- Resolve before Notify responder
- Postmortem before Resolve

The workflow records trace:

- IncidentLifecycleViewedScenario115
