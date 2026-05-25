# Shared Support AIL-Spec Example

The application Shared Support manages reusable support-domain declarations.

A User has:

- id: Text
- role: State<Customer, SupportAgent, SupportManager>
- email: Text

Failure PermissionDenied happens when a user lacks support staff permission:

- the system reveals no secret value
- the caller sees "Permission denied"
- the trace records SharedPermissionDenied
