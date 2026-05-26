# Incident Notifications Example

## Purpose

This AgentTool support package defines the responder notification tool used by
the incident-response workflow.

## Concepts Taught

- AgentTool support packages imported by a high-level Application profile.
- Secret pager tokens that must not be exposed to agent-visible output.
- Audit traces for external provider calls such as `PagerProvider.notify`.
- Approval and permission requirements for Sev1 notification paths.

## Files To Inspect

- `ail-package.md`: AgentTool support-module metadata.
- `spec.ail-spec.md`: `Notify incident responder` tool contract.
- `../incident_response.ail/spec.ail-spec.md`: the application action that
  records notification audit evidence.

## Expected Replay Artifacts

This support package is exercised by incident-response catalog entries,
especially `example-112` and `example-115`, where notification anchors survive
into checked artifacts and target reports.

## Rejected Fixtures

This package has no package-local rejected fixtures. v0.3 should add rejected
notification fixtures for missing pager approval, leaked pager token, and
provider-call audit omissions.

## Next Example To Read

Read `incident_response.ail/README.md` next to see the notification tool inside
a complete incident lifecycle.

## v0.3 Learning Signal

Incident notification examples need AgentTool-specific repair tutorials for
approval, secret redaction, and audit-trace failures.
