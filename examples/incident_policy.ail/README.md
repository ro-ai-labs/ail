# Incident Policy Example

## Purpose

This support package defines service tiers, severity policy, escalation rules,
and policy-violation failure semantics for incident-response workflows.

## Concepts Taught

- Policy support modules imported by a larger application.
- Severity and service-tier state used by escalation decisions.
- Failure contracts that preserve incident state and record
  `IncidentPolicyViolation`.

## Files To Inspect

- `ail-package.md`: support-module metadata.
- `spec.ail-spec.md`: service, escalation policy, and policy-violation
  failure definitions.
- `../incident_response.ail/spec.ail-spec.md`: escalation actions that consume
  these policy definitions.

## Expected Replay Artifacts

This package is replayed through incident-response catalog entries such as
`example-111` through `example-115`, where policy anchors are preserved in
story, Core, VM trace, and target-contract evidence.

## Rejected Fixtures

This package has no package-local rejected fixtures. v0.3 should add rejected
policy fixtures for missing commander review, invalid severity escalation, and
service-tier mismatch.

## Next Example To Read

Read `incident_notifications.ail/README.md` next, then
`incident_response.ail/README.md` for the combined workflow.

## v0.3 Learning Signal

Incident policy examples need checker-visible escalation diagnostics so
reviewers can learn which policy rule blocks an unsafe incident action.
