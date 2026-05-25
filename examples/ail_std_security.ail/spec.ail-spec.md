# AIL Standard Security Package

Package: ail.std.security.

The application AIL Standard Security manages secret values, permissions, and
capability declarations.

A SecretEnvelope has:

- id: Text
- payload: Secret<Text>
- permission: Text
- capability: Text

Action: Reveal secret.

When an authorized reviewer reveals a secret:

- the system requires reveal permission
- the system requires secret capability
- the system reads SecretEnvelope payload
- the system does not reveal SecretEnvelope payload
- the system guarantees secret payload is redacted from traces
- the system records a trace event named SecretRevealed
