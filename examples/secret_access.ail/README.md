# Secret Access Example

## Purpose

`secret_access.ail` is the focused secret and permission example. It models a
ticket with secret internal notes, a `View internal notes` action, support
staff permissions, customer redaction, and a `PermissionDenied` failure path.

The package is useful when reviewing whether AIL can preserve security intent
through checked specification, checked Core, bytecode, target report, and
regenerated user stories without leaking the secret value.

The top-level spec is the clean path. The package-local conformance fixtures
teach the v0.3 failure paths for support-role requirements, redaction, denied
access traces, and declared failure handling.

## Concepts Taught

- `Secret<List<Text>>` fields for internal notes.
- Role checks for `SupportAgent` and `SupportManager`.
- Redaction guarantees that customers never receive internal notes.
- Successful access traces through `InternalNotesViewed`.
- Denied access traces through `InternalNotesDenied`.
- Runtime evidence that permission failures report `PermissionDenied` while
  keeping internal notes secret.

## Files To Inspect

- `ail-package.md`: Application profile metadata and secret/failure feature
  declarations.
- `spec.ail-spec.md`: the Secret Access specification.
- `examples/accepted/view-internal-notes-minimal.ail-spec.md`: minimal accepted
  fixture for support-role gated secret reads and denied-access tracing.
- `examples/rejected/internal-notes-without-support-role.ail-spec.md`: rejected
  fixture for `AIL-SECRET-ROLE-001`.
- `examples/rejected/internal-notes-without-redaction.ail-spec.md`: rejected
  fixture for `AIL005`.
- `examples/rejected/permission-denied-without-trace.ail-spec.md`: rejected
  fixture for `AIL-TRACE-002`.
- `examples/rejected/permission-denied-without-failure-section.ail-spec.md`:
  rejected fixture for `AIL003`.
- `../examples.md`: entries `example-75` through `example-79` exercise
  security-permissions over core-to-spec, core-to-summary, flow-patch,
  trace-debug, and interop prompt surfaces.
- `../stories/example-75.md` through `../stories/example-79.md`: story views
  with anchors for the secret action, traces, permission failure, and prompt
  surface.

## Expected Replay Artifacts

Replay the corpus with release evidence enabled:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-secret-access-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-75/checked.ail-core.txt`
- `examples/example-75/artifact.ailbc.json`
- `examples/example-75/target-report.txt`
- `examples/example-75/user-story.txt`
- `examples/example-79/target-report.txt`

For a focused package check:

```bash
cargo run -- ail-conformance examples/secret_access.ail --artifact-dir /tmp/ail-secret-access-conformance
```

## Rejected Fixtures

The rejected fixtures are intentionally narrow:

- `internal-notes-without-support-role.ail-spec.md` -> `AIL-SECRET-ROLE-001`
- `internal-notes-without-redaction.ail-spec.md` -> `AIL005`
- `permission-denied-without-trace.ail-spec.md` -> `AIL-TRACE-002`
- `permission-denied-without-failure-section.ail-spec.md` -> `AIL003`

## Next Example To Read

Read `../ail_std_security.ail/README.md` to see reusable security contracts,
then `../support_ticket.ail/README.md` for the broader application workflow
that embeds secret internal notes inside ticket state and customer-facing
history.

## v0.3 Learning Signal

Secret Access now has package-local accepted/rejected fixtures that prove secret
read permission, redaction, failure declaration, and denied-access trace gaps
are caught before compile or runtime replay. The next v0.3 bar is threat-model
annotations and audit-trail artifacts that connect these diagnostics to
reviewer-facing security stories.
