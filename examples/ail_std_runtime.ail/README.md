# AIL Standard Runtime Example

## Purpose

`ail_std_runtime.ail` teaches runtime task execution, runtime failures, and
standard package capability grants. It defines `RuntimeTask`, the `Run task`
action, and the `RuntimeUnavailable` failure path.

This package is important because it imports both `Core` and `Effects`, then
declares a capability grant for runtime network host effect. It proves that
standard-library packages can depend on each other while keeping host effects
and failure behavior reviewable.

## Concepts Taught

- `RuntimeTask` state transitions from `Ready` through `Running`, `Failed`,
  and `Complete`.
- `Run task` requirements and `TaskRun` trace coverage.
- `RuntimeUnavailable` failure handling.
- Failure-side state mutation and user-visible error text.
- Dependency reporting for `Core` and `Effects` imports.
- Capability grants, including the rejected `missing-capability-grant` case.

## Files To Inspect

- `ail-package.md`: imports `../ail_std_core.ail` and
  `../ail_std_effects.ail`, then grants `runtime network host effect`.
- `spec.ail-spec.md`: canonical runtime package specification.
- `examples/accepted/run-task-minimal.ail-spec.md`: accepted minimal runtime
  task fixture.
- `examples/rejected/missing-capability-grant.ail/ail-package.md`: rejected
  package manifest without the required capability grant.
- `examples/rejected/missing-capability-grant.ail/spec.ail-spec.md`: rejected
  runtime spec paired with the missing grant.

## Expected Replay Artifacts

Run focused conformance:

```bash
cargo run -- ail-conformance examples/ail_std_runtime.ail --artifact-dir /tmp/ail-std-runtime-conformance
```

Run the dependency report check:

```bash
cargo test cli_ail_stdlib_import_records_dependency_report --test ail_toolchain
```

Useful artifacts include the conformance report, dependency report, rejected
fixture diagnostics, and checked Core output for `RunTask`.

## Rejected Fixtures

The package includes `examples/rejected/missing-capability-grant.ail`, which
verifies that runtime network host effects require an explicit package-level
capability grant.

v0.3 should add rejected fixtures for missing `TaskRun`, failure handling that
does not set `Failed`, successful runs from non-`Ready` tasks, and
`RuntimeUnavailable` paths that omit the user-visible diagnostic.

## Next Example To Read

Read `../stateful_counter.ail/README.md` next for deterministic runtime state,
then `../network_driver.ail/README.md` for lower-level system boundaries.

## v0.3 Learning Signal

AIL Standard Runtime should become the bridge between application workflows,
system effects, and host execution. v0.3 should add scheduler examples,
retry/backoff policy examples, richer runtime diagnostics, and story
amendments that change task lifecycle behavior without losing failure
semantics.
