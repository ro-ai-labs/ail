# Runtime Generic Example

## Purpose

`runtime_generic.ail` is the focused runtime value-flow example. It models
Runtime Tickets where `Prioritize ticket` changes a ticket from low priority
to high priority, records `TicketPrioritized`, and proves that typed state
requirements survive the full prompt/spec/Core/bytecode/target-report path.

The package is useful when reviewing whether AIL can keep a small typed action
understandable across regenerated user stories without relying on a larger
application workflow.

## Concepts Taught

- State values with `Low` and `High` priority variants.
- Runtime preconditions such as ticket existence and not already being high
  priority.
- State mutation from low priority to high priority.
- Guarantees that high-priority tickets are handled first.
- Trace coverage through `TicketPrioritized`.
- Target-report evidence for a compact typed action.

## Files To Inspect

- `ail-package.md`: Application profile metadata and first-slice conformance.
- `spec.ail-spec.md`: the Runtime Tickets specification.
- `examples/rejected/missing-ticket-prioritized-trace.ail-spec.md`: rejected
  fixture for dropping the `TicketPrioritized` trace from the runtime action.
- `../examples.md`: entries `example-35` through `example-39` exercise
  runtime-generics over core-to-spec, core-to-summary, flow-patch,
  trace-debug, and interop prompt surfaces.
- `../stories/example-35.md` through `../stories/example-39.md`: story views
  with anchors for the runtime action, trace, initial state, and prompt
  surface.

## Expected Replay Artifacts

Replay the corpus with release evidence enabled:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-runtime-generic-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-35/checked.ail-core.txt`
- `examples/example-35/artifact.ailbc.json`
- `examples/example-35/target-report.txt`
- `examples/example-35/user-story.txt`
- `examples/example-39/target-report.txt`

For a focused package check:

```bash
cargo run -- ail-conformance examples/runtime_generic.ail --artifact-dir /tmp/ail-runtime-generic-conformance
```

## Rejected Fixtures

The package includes
`examples/rejected/missing-ticket-prioritized-trace.ail-spec.md`, which
verifies that `Prioritize ticket` must preserve the `TicketPrioritized` trace
before conformance accepts the fixture.

v0.3 should add more rejected specs for prioritizing a missing ticket, allowing
an already-high-priority ticket through the action, and changing the priority
state type without a story amendment.

## Next Example To Read

Read `../stateful_counter.ail/README.md` for the simpler deterministic state
baseline, then `../support_ticket.ail/README.md` for a full application
workflow that expands typed ticket state into roles, permissions, scheduler
behavior, and target evidence.

## v0.3 Learning Signal

Runtime Generic now has package-local guidance and story anchors for typed
runtime flow. v0.3 should add clearer type-inference explanations, rejected
fixtures for invalid state transitions, and story diffs that show how a user
request changes the type contract.
