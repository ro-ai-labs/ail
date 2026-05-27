# Stateful Counter Example

## Purpose

`stateful_counter.ail` is the smallest runtime-state package in the corpus. It
exists to prove that AIL can describe a mutable value, lower a state-changing
action, execute it through VM/native evidence, and regenerate a semantically
similar story view from the checked artifacts.

The top-level spec is intentionally minimal, so it should be read as the
runtime-state baseline. The package-local conformance fixtures are the v0.3
teaching layer for durable state, idempotent retries, shared-state locking, and
replay after failure.

## Concepts Taught

- Application profile state with a single `Counter` thing.
- Integer field mutation through `Increment counter`.
- Trace emission through `CounterIncremented`.
- VM replay for deterministic state changes.
- Native Linux target evidence for a simple state transition.
- User-story metadata for a spec-to-story and story-amendment path.
- Persistence guarantees for replay-visible counter writes.
- Idempotency keys for retryable counter increments.
- Lock or serialization rules for shared counter mutation.
- Replay recovery policy for failure after a counter write.

## Files To Inspect

- `ail-package.md`: package metadata and declared state/runtime features.
- `spec.ail-spec.md`: the canonical counter specification.
- `examples/accepted/persistent-increment-minimal.ail-spec.md`: minimal
  persistence guarantee for a counter increment.
- `examples/accepted/idempotent-increment-request-minimal.ail-spec.md`:
  request-id and dedupe-state evidence for retryable increments.
- `examples/accepted/locked-counter-increment-minimal.ail-spec.md`: System
  lock guard plus Application serialization guarantee.
- `examples/accepted/replay-after-failure-minimal.ail-spec.md`: failure
  handling with replay recovery policy.
- `examples/rejected/*.ail-spec.md`: package-local negative fixtures for
  missing persistence, idempotency, lock, and replay policy checks.
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
- `examples/example-95/state-boundary-review.txt`
- `examples/example-95/state-boundary-review.fingerprint.txt`
- `examples/example-100/vm-trace.txt`
- `examples/example-110/target-IncrementCounter.elf`
- `examples/example-110/target-report.txt`
- `examples/example-110/state-boundary-review.txt`
- `examples/example-110/user-story.txt`

The direct package checks are:

```bash
cargo run -- ail-check examples/stateful_counter.ail
cargo run -- ail-conformance examples/stateful_counter.ail --artifact-dir /tmp/ail-stateful-counter-conformance
```

## Rejected Fixtures

The rejected fixtures are intentionally small and checker-focused:

- `increment-without-persistence-guarantee.ail-spec.md` -> `AIL-STATE-001`
- `retryable-increment-without-idempotency-key.ail-spec.md` -> `AIL-STATE-002`
- `shared-counter-without-lock.ail-spec.md` -> `AIL-STATE-003`
- `failure-after-write-without-replay-policy.ail-spec.md` -> `AIL-STATE-004`

## Next Example To Read

Read `../runtime_generic.ail/spec.ail-spec.md` next for a broader runtime value
flow. Then read `../incident_response.ail/spec.ail-spec.md` for a high-level
workflow that uses state across multiple modules and stories.

## v0.3 Learning Signal

The current counter package now teaches the first stateful runtime policies:
persistence, idempotency, locking, and replay recovery. Its v0.3 state signal
is now promoted by deterministic state-boundary review artifacts that connect
these policies, `AIL-STATE-001` through `AIL-STATE-004`, `CounterIncremented`,
and VM/native replay fingerprints. The next bar is a richer state family with
migrations, stale-state conflict detection, multi-action transactions, and
durable runtime evidence beyond text-level policy checks.
