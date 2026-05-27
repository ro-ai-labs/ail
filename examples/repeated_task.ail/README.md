# Repeated Task Example

## Purpose

`repeated_task.ail` is the focused scheduled-workflow example. It models a
small Maintenance Runner where `Run maintenance cycle` repeats
`IncrementCounter` three times and records `MaintenanceCycleCompleted`.

The package is useful when reviewing whether AIL can express repeated action
execution, preserve stateful trace evidence, and carry scheduler-like workflow
intent from prompt capture through checked specification, checked Core,
bytecode, target report, and regenerated user story.

## Concepts Taught

- Repeated action execution through `Run maintenance cycle`.
- Integer state mutation on `Counter.value`.
- Reuse of the `IncrementCounter` action inside a larger workflow.
- Temporal policy authoring with explicit scheduler-behavior claims.
- Retry and backoff policy authoring for scheduled repeated workflows.
- Trace coverage through `CounterIncremented` and
  `MaintenanceCycleCompleted`.
- Scheduled-workflow metadata with `scheduler`, `task.store`, and `audit.log`
  interaction tags.
- Target-report evidence for a high-level workflow that still lowers to
  deterministic runtime behavior.

## Files To Inspect

- `ail-package.md`: Application profile metadata and `repeated-tasks` feature
  declaration.
- `spec.ail-spec.md`: the Maintenance Runner specification.
- `../examples.md`: entries `example-80` through `example-84` exercise the
  scheduled-workflow family over interview, requirements, spec-draft,
  core-draft, and diagnostic-repair prompt surfaces.
- `../stories/example-80.md` through `../stories/example-84.md`: story views
  with anchors for the maintenance cycle, repeated action, trace event, and
  prompt surface.

## Expected Replay Artifacts

Replay the corpus with release evidence enabled:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-repeated-task-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-80/checked.ail-core.txt`
- `examples/example-80/artifact.ailbc.json`
- `examples/example-80/target-report.txt`
- `examples/example-80/user-story.txt`
- `examples/example-84/target-report.txt`

For a focused package check:

```bash
cargo run -- ail-conformance examples/repeated_task.ail --artifact-dir /tmp/ail-repeated-task-conformance
```

## Rejected Fixtures

Package-local fixtures now cover the first scheduler-policy boundary:

- `examples/accepted/temporal-policy-minimal.ail-spec.md` keeps the repeated
  `IncrementCounter` lowering and adds an explicit scheduler-behavior claim
  plus temporal policy.
- `examples/accepted/retry-backoff-policy-minimal.ail-spec.md` adds a bounded
  retry policy and exponential backoff policy to the scheduled maintenance
  workflow.
- `examples/rejected/scheduler-without-temporal-policy.ail-spec.md` claims
  scheduler behavior for the repeated maintenance cycle without a temporal
  policy and must report `AIL-WORKFLOW-001`.
- `examples/rejected/retry-policy-without-backoff.ail-spec.md` declares a
  retry policy for the scheduled maintenance cycle without a backoff policy and
  must report `AIL-WORKFLOW-002`.

Future rejected fixtures should cover dropping `MaintenanceCycleCompleted`,
repeating the wrong action, changing the repeat count without a story
amendment, and omitting counter state mutation.

## Next Example To Read

Read `../stateful_counter.ail/README.md` before this guide if you need the
single-action state baseline. Then read `../support_ticket.ail/README.md` and
`../incident_response.ail/README.md` to see high-level workflows with richer
roles, permissions, UI, and target-contract surfaces.

## v0.3 Learning Signal

Repeated Task now has package-local guidance and story anchors for the
maintenance cycle, repeated action, trace event, and scheduled-workflow
metadata. v0.3 now includes temporal policy syntax, retry/backoff policy
syntax, an `AIL-WORKFLOW-001` diagnostic for missing temporal policies, and an
`AIL-WORKFLOW-002` diagnostic for retry policies without backoff. Release
evidence emits `workflow-scheduler-review.txt` and a fingerprint for each
repeated-task corpus entry so reviewers can connect scheduled-workflow stories
to accepted and rejected policy fixtures. The next bar is richer scheduler
policy forms and story amendments that explain temporal policy changes.
