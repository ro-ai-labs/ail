# Incident Identity AIL-Spec Example

The application Incident Identity manages responders and service owners.

The application has these users:

- Responder
- Incident commander
- Service owner

A User has:

- id: Text
- role: State<Responder, IncidentCommander, ServiceOwner>
- email: Text
- pager: Text

A Team has:

- id: Text
- name: Text
- primary responder: User
- commander: User
