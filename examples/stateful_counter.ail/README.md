# Stateful Counter Example

## Purpose

`stateful_counter.ail` is the smallest runtime-state package in the corpus. It
exists to prove that AIL can describe a mutable value, lower a state-changing
action, execute it through VM/native evidence, and regenerate a semantically
similar story view from the checked artifacts.

The example is intentionally minimal, so it should be read as a baseline rather
than as a complete state-management tutorial.

## Concepts Taught

- Application profile state with a single `Counter` thing.
- Integer field mutation through `Increment counter`.
- Trace emission through `CounterIncremented`.
- VM replay for deterministic state changes.
- Native Linux target evidence for a simple state transition.
- User-story metadata for a spec-to-story and story-amendment path.

## Files To Inspect

- `ail-package.md`: package metadata and declared state/runtime features.
- `spec.ail-spec.md`: the canonical counter specification.
- `../examples.md`: entries `example-95` through `example-98`, `example-100`,
  and `example-110` replay the counter across Core/spec/repair prompt surfaces.
- `../stories/example-95.md` through `../stories/example-98.md`,
  `../stories/example-100.md`, and `../stories/example-110.md`: user-story
  views generated for the stateful-runtime examples.

## Expected Replay Artifacts

Replay the corpus to inspect the counter artifacts:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-stateful-counter-examples --release-evidence
```

Useful artifacts to inspect after replay:

- `examples/example-95/checked.ail-core.txt`
- `examples/example-95/artifact.ailbc.json`
- `examples/example-95/vm-trace.txt`
- `examples/example-100/vm-trace.txt`
- `examples/example-110/target-IncrementCounter.elf`
- `examples/example-110/target-report.txt`
- `examples/example-110/user-story.txt`

The direct package check is:

```bash
cargo run -- ail-check examples/stateful_counter.ail
```

## Rejected Fixtures

This package does not yet include rejected fixtures. That absence is itself a
v0.3 gap: a useful stateful package should include rejected examples for stale
state, missing trace coverage, invalid numeric updates, lock or ownership
conflicts, and replay-after-failure behavior.

## Next Example To Read

Read `../runtime_generic.ail/spec.ail-spec.md` next for a broader runtime value
flow. Then read `../incident_response.ail/spec.ail-spec.md` for a high-level
workflow that uses state across multiple modules and stories.

## v0.3 Learning Signal

The current counter examples prove that the pipeline handles a single mutable
integer, but they do not yet teach persistence, idempotency, retries,
migrations, locking, or replay after failure. v0.3 should replace some repeated
counter prompt-surface entries with richer stateful scenarios that exercise
those boundaries.
