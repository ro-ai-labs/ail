# Incident Response Example

## Purpose

`incident_response.ail` is the high-level multi-module teaching package for a
realistic service incident workflow. It shows how AIL can represent an
application that spans identity, policy, notification, workflow transitions,
UI command surfaces, secret data, target contracts, VM replay, and regenerated
user-story views.

The example is meant to prove more than one action. It demonstrates that a
complex system can be authored as several specs, imported as a package graph,
checked into Core, lowered into bytecode, and replayed through VM, Wasm, and
Darwin target-contract evidence while preserving reviewer-facing stories.

## Concepts Taught

- Multi-module application structure with `incident_identity.ail`,
  `incident_policy.ail`, `incident_notifications.ail`, and
  `incident_response.ail`.
- Role modeling for responder, incident commander, and service owner users.
- Incident state transitions from declaration through postmortem.
- Policy-sensitive escalation and notification audit behavior.
- Secret private notes and pager tokens that must not leak to public timeline
  or agent-visible outputs.
- UI route, form, dashboard, and workflow semantics in the same package as
  application actions.
- Workflow ordering rules that block notification, resolution, and postmortem
  steps until earlier lifecycle steps have happened.
- User-story journeys across `story-to-spec`, `spec-to-story`, and
  `story-amendment`.

## Files To Inspect

- `ail-package.md`: package metadata, imports, profile, features, and target
  support.
- `spec.ail-spec.md`: the canonical incident response specification.
- `../incident_identity.ail/spec.ail-spec.md`: users, roles, teams, email, and
  pager data.
- `../incident_policy.ail/spec.ail-spec.md`: services, severity policy, and
  policy violation failure semantics.
- `../incident_notifications.ail/spec.ail-spec.md`: AgentTool notification
  surface, pager token handling, permission, approval, and audit trace.
- `../examples.md`: entries `example-111` through `example-115` cover the
  incident response prompt surfaces, targets, and story journeys.
- `../stories/example-111.md` through `../stories/example-115.md`: generated
  user-story views for declaration, escalation, lifecycle regeneration,
  amendment, and dashboard coverage.

## Expected Replay Artifacts

Replay the corpus to inspect the incident artifacts:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-incident-response-examples --release-evidence
```

Useful artifacts to inspect after replay:

- `examples/example-111/checked.ail-core.txt`
- `examples/example-111/artifact.ailbc.json`
- `examples/example-111/vm-trace.txt`
- `examples/example-112/target-report.txt`
- `examples/example-113/user-story.txt`
- `examples/example-114/target-report.txt`
- `examples/example-115/target-report.txt`

The direct conformance check is:

```bash
cargo run -- ail-conformance examples/incident_response.ail --artifact-dir /tmp/ail-incident-response-conformance
```

## Rejected Fixtures

This package includes package-local conformance fixtures:

- `examples/accepted/incident-escalation-minimal.ail-spec.md` accepts the
  minimal escalation, notification audit, resolution, and postmortem path.
- `examples/rejected/notification-without-responder-pager.ail-spec.md` rejects
  notification audit behavior that omits the responder pager requirement.
- `examples/rejected/resolve-without-mitigating-status.ail-spec.md` rejects
  resolution that skips the `Mitigating` predecessor state.
- `examples/rejected/postmortem-without-resolved-status.ail-spec.md` rejects
  postmortem start that skips the `Resolved` predecessor state.
- `examples/rejected/private-notes-public-timeline-leak.ail-spec.md` rejects
  private-note leakage into the public timeline.
- `examples/rejected/escalation-without-commander-review.ail-spec.md` rejects
  escalation that omits commander-review policy coverage.
- `examples/rejected/route-missing-permission.ail-spec.md` rejects command
  routes that read incident data without a route permission.
- `examples/rejected/dashboard-missing-permission.ail-spec.md` rejects
  service-owner dashboards that read incident data without a dashboard
  permission.

Running conformance with `--artifact-dir` now writes fingerprinted
package-local repair tutorials and checked repair proofs for each rejected
fixture under `rejected/<fixture>/`. The proof bundle includes
`repair-proof.txt`, `repair-candidate.ail-spec.md`,
`repair-checked.ail-core.txt`, and `repair-artifact.ailbc.json`, preserving the
diagnostic, source provenance, affected graph item, repair suggestion,
corrected fixture candidate, checked Core, and verified bytecode before any
repaired variant is promoted.

The top-level replay catalog includes `example-122` and `example-123`, which
promote the private-notes public timeline and commander-review repair
candidates into accepted end-to-end examples with stored request, response,
story, checked Core, bytecode, and VM trace evidence.

## Next Example To Read

Read `../refund_tool.ail/README.md` before this package if you want the smaller
AgentTool safety surface first. After this package, the next useful examples
should add more repaired incident variants through the same corpus-copy import
path, plus a richer stateful application that teaches persistence,
idempotency, locks, and replay after failure.

## v0.3 Learning Signal

Incident Response is the current high-level benchmark for AIL examples. It
shows that complex systems need richer story graphs across imported modules,
UI surfaces, workflow transitions, target contracts, and regenerated story
views. v0.3 should promote this from a passing corpus family into a guided
walkthrough with additional repair paths and story-diff artifacts that show
exactly how a user story amends the checked spec.
