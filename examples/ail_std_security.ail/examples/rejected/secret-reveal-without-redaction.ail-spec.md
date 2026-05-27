# Rejected Security Fixture: Secret Reveal Without Redaction

Package: ail.std.security.

The application AIL Standard Security manages secret values, permissions, and
capability declarations.

A SecretEnvelope has:

- id: Text
- payload: Secret<Text>

Action: Reveal secret.

When an authorized reviewer reveals a secret:

- the system requires reveal permission
- the system requires secret capability
- the system reads SecretEnvelope payload
- the system records a trace event named SecretRevealed
