# Private Notes Public Timeline Leak AIL-Spec Example

The application Broken Incident Privacy manages invalid incident privacy
examples.

An Incident has:

- id: Text
- status: State<Mitigating>
- private notes: Secret<List<Text>>
- timeline: List<Text>

The application shows:

- a public timeline that never includes private notes

Action: Publish private notes.

When an incident commander publishes private notes:

- the system requires the incident to exist
- the system records private notes to the public timeline
- the system records a trace event named IncidentPrivateNotesPublished
