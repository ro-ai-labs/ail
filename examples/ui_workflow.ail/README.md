# UI Workflow Example

## Purpose

`ui_workflow.ail` is the UI profile teaching package. It models a user-facing
support interface with a ticket detail route, create-ticket form, support
manager dashboard, refund approval workflow, accessibility requirements, and
checked action calls.

This example is the main high-level UI benchmark for AIL. It proves that
stored prompt transcripts can generate a UI-focused spec, lower it to checked
AIL-Core, compile bytecode, run VM evidence, and produce
`wasm32-unknown-sandbox-wasm` target-contract artifacts while preserving a
reviewable user-story view.

## Concepts Taught

- UI profile package metadata and target support.
- routes with parameterized paths, reads, permissions, and trace events.
- forms that call checked actions, declare fields, validate input, and record
  validation failure traces.
- dashboards with role-sensitive permissions and filters.
- accessibility requirements, including announced field errors.
- Workflow ordering, especially blocking `Provider call before Manager
  approval`.
- Destructive action confirmation as a UI safety rule.
- Wasm target-contract evidence for browser-like sandbox targets.
- User-story replay for `story-to-spec` and `spec-to-story` UI journeys.

## Files To Inspect

- `ail-package.md`: UI profile metadata, feature list, schema support, and
  Wasm target support.
- `spec.ail-spec.md`: canonical UI workflow specification.
- `examples/accepted/ui-minimal.ail-spec.md`: minimal accepted UI fixture.
- `examples/accepted/destructive-confirmation-minimal.ail-spec.md`: accepted
  destructive-action confirmation fixture.
- `examples/rejected/inaccessible-error-text.ail-spec.md`: rejected
  accessibility fixture.
- `examples/rejected/workflow-out-of-order-provider-call.ail-spec.md`:
  rejected workflow-order fixture.
- `examples/rejected/dashboard-missing-permission.ail-spec.md`: rejected
  dashboard permission fixture.
- `examples/rejected/destructive-action-without-confirmation.ail-spec.md`:
  rejected destructive-action safety fixture.
- `examples/rejected/form-missing-action.ail-spec.md`: rejected form/action
  linkage fixture.
- `../examples.md`: entries `example-65`, `example-108`, and `example-109`
  replay the real UI profile package through Core/spec, spec-draft, and
  requirements prompt surfaces.
- `../stories/example-65.md`, `../stories/example-108.md`, and
  `../stories/example-109.md`: user-story views for the UI workflow family.

## Expected Replay Artifacts

Replay the corpus to inspect UI Workflow artifacts:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-ui-workflow-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-65/checked.ail-core.txt`
- `examples/example-65/artifact.ailbc.json`
- `examples/example-65/target-report.txt`
- `examples/example-65/user-story.txt`
- `examples/example-108/target-report.txt`
- `examples/example-109/target-report.txt`

For focused conformance, including local accepted and rejected fixtures:

```bash
cargo run -- ail-conformance examples/ui_workflow.ail --artifact-dir /tmp/ail-ui-workflow-conformance
```

For a direct Wasm contract build of the form action:

```bash
cargo run -- ail-compile examples/ui_workflow.ail --action CreateTicketForm --target wasm32-unknown-sandbox-wasm --artifact-dir /tmp/ail-ui-workflow-wasm
```

## Rejected Fixtures

This package already has package-local rejected fixtures for important UI
failure modes:

- inaccessible form error text;
- `Provider call before Manager approval`;
- dashboard access without the matching permission;
- destructive actions without confirmation;
- forms that do not call a checked action.

v0.3 should turn these into repair tutorials that show the original story, the
diagnostic, the corrected spec, the regenerated story, and the changed
target-contract report.

## Next Example To Read

Read `../support_ticket.ail/README.md` before this guide if you want the
application workflow behind the UI. After this package, read
`../incident_response.ail/README.md` for a larger high-level system that mixes
application actions, UI routes, dashboards, workflow transitions, and
multi-module policy.

## v0.3 Learning Signal

UI Workflow covers the first serious user-facing surface, and its primary
story files now carry semantic anchors for routes, forms, dashboards,
workflow traces, and Wasm target reports. v0.3 still needs stronger visual
review artifacts, story-to-screen traceability, and accessibility exercises
that prove the regenerated user-story view preserves the intended user
experience.
