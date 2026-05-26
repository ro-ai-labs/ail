# Incident Identity Example

## Purpose

This support package defines the responder, incident commander, service owner,
user, and team identity model imported by the incident-response multi-module
examples.

## Concepts Taught

- Support-module packages that are not counted directly in the replay catalog.
- Shared identity types for incident workflows.
- Role fields that downstream incident actions use for authorization and
  assignment review.

## Files To Inspect

- `ail-package.md`: support-module metadata.
- `spec.ail-spec.md`: user roles, email, pager, and team ownership.
- `../incident_response.ail/spec.ail-spec.md`: imports and uses this identity
  model.

## Expected Replay Artifacts

This package is replayed through `incident_response.ail` catalog entries such
as `example-111` through `example-115`, where identity anchors appear in
checked Core, VM traces, target reports, and generated story artifacts.

## Rejected Fixtures

This package has no package-local rejected fixtures. v0.3 should add rejected
identity fixtures for missing responder role, unknown commander, and invalid
team ownership.

## Next Example To Read

Read `incident_policy.ail/README.md` next, then return to
`incident_response.ail/README.md` for the full workflow.

## v0.3 Learning Signal

Incident identity examples need explicit role and ownership diagnostics so
multi-module workflows can teach why an assignment or escalation is rejected.
