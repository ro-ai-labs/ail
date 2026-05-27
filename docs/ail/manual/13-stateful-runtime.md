# AIL Manual: Stateful Runtime

## Purpose

The Stateful Runtime chapter checks `examples/stateful_counter.ail` as the
smallest package that teaches mutable Application state beyond a happy-path
counter. It is the manual companion to the package-local fixtures for
persistence, idempotent retries, shared-state serialization, and replay
recovery after failure.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter stateful-runtime --run-checks
```

The direct commands are:

```sh
cargo run -- ail-conformance examples/stateful_counter.ail --artifact-dir /tmp/ail-manual-stateful-runtime-conformance
cargo run -- ail-run examples/stateful_counter.ail --action IncrementCounter counter.value=41
```

## What It Proves

- `examples/stateful_counter.ail/spec.ail-spec.md` still lowers to checked Core
  and integer bytecode for `IncrementCounter`.
- `examples/stateful_counter.ail/examples/accepted/persistent-increment-minimal.ail-spec.md`
  proves a replay-visible persistence guarantee for counter writes.
- `examples/stateful_counter.ail/examples/accepted/idempotent-increment-request-minimal.ail-spec.md`
  proves retryable increments carry an idempotency key and processed-request
  state.
- `examples/stateful_counter.ail/examples/accepted/locked-counter-increment-minimal.ail-spec.md`
  proves shared counter mutation has a lock or serialization rule.
- `examples/stateful_counter.ail/examples/accepted/replay-after-failure-minimal.ail-spec.md`
  proves a failure after a write has replay recovery policy.
- `examples/stateful_counter.ail/examples/rejected/*.ail-spec.md` rejects the
  same stateful authoring mistakes with stable `AIL-STATE-*` diagnostics.
- The runtime command proves the checked counter action moves
  `counter.value=41` to `counter.value=42` and emits the trace
  `add counter.value by 1 -> 42`.

## Expected Evidence

The chapter should surface:

```text
conformance-report.txt
manifest.ail-conformance.txt
accepted: persistent-increment-minimal.ail-spec.md
accepted: idempotent-increment-request-minimal.ail-spec.md
accepted: locked-counter-increment-minimal.ail-spec.md
accepted: replay-after-failure-minimal.ail-spec.md
rejected: increment-without-persistence-guarantee.ail-spec.md AIL-STATE-001
rejected: retryable-increment-without-idempotency-key.ail-spec.md AIL-STATE-002
rejected: shared-counter-without-lock.ail-spec.md AIL-STATE-003
rejected: failure-after-write-without-replay-policy.ail-spec.md AIL-STATE-004
ail-run succeeded
counter.value=42
add counter.value by 1 -> 42
trace CounterIncremented
ail conformance: ok
```

## Relationship To User Story Mode

User Story mode proves a story can become requirements, accepted spec, checked
Core, bytecode, agent trace, and target evidence. This chapter proves the
stateful runtime surface behind those stories has local policy checks for
durability, retry safety, serialization, and replay recovery before the
examples corpus treats a stateful response as accepted evidence.
