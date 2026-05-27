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
- `examples/accepted/notify-responder-minimal.ail-spec.md`: minimal accepted
  notification contract for package-local conformance.
- `examples/rejected/*.ail-spec.md`: rejected notification contracts that teach
  approval, permission, secret-output, and provider-audit repairs.
- `../incident_response.ail/spec.ail-spec.md`: the application action that
  records notification audit evidence.

## Expected Replay Artifacts

This support package is exercised by incident-response catalog entries,
especially `example-112` and `example-115`, where notification anchors survive
into checked artifacts and target reports.

## Rejected Fixtures

Run the package-local repair set with:

```sh
cargo run -- ail-conformance examples/incident_notifications.ail --artifact-dir /tmp/ail-incident-notifications-conformance
```

The accepted fixture is `notify-responder-minimal.ail-spec.md`.
Rejected fixtures are:

- `approval-without-rule.ail-spec.md`: teaches `AIL018` when a tool mentions
  approval without an explicit approval rule.
- `permission-without-rule.ail-spec.md`: teaches `AIL019` when a tool mentions
  permission without an explicit permission rule.
- `pager-token-secret-output.ail-spec.md`: teaches `AIL020` when an
  agent-visible output exposes `Secret<Text>` without reveal permission.
- `provider-call-without-audit-entry.ail-spec.md`: teaches
  `AIL-AGENT-AUDIT-001` when an external provider call lacks an audit write or
  audit-trace guarantee.

## Next Example To Read

Read `incident_response.ail/README.md` next to see the notification tool inside
a complete incident lifecycle.

## v0.3 Learning Signal

Incident notification examples now teach AgentTool-specific repair tutorials
for approval, permission, secret redaction, and provider-call audit evidence.
The next bar is provider failure and retry-policy fixtures that connect
notification delivery failures to incident workflow recovery.
