# AIL Standard Effects Example

## Purpose

`ail_std_effects.ail` teaches declared runtime effects in the standard
library. It models `ResourceEffect` and actions for reading resources, writing
resources, and sending network messages.

This package connects standard-library values to observable behavior. It is
the place to inspect when checking whether AIL can represent resource access,
state mutation, network effects, and traceable effect boundaries before a
runtime or system package consumes them.

## Concepts Taught

- `ResourceEffect` as a reviewable declaration of resource, effect, and id.
- `Read resource` requirements and `ResourceRead` trace coverage.
- `Write resource` state mutation and `ResourceWritten` trace coverage.
- Network effect modeling through `Send network message`.
- Trace coverage through `NetworkMessageSent`.
- Importing `ail.std.core` for shared primitive contracts.

## Files To Inspect

- `ail-package.md`: imports `../ail_std_core.ail compatible ^0.2 as Core`.
- `spec.ail-spec.md`: canonical effects specification.
- `examples/accepted/read-resource-minimal.ail-spec.md`: accepted minimal
  effect fixture.
- `../ail_std_runtime.ail/ail-package.md`: imports this package as `Effects`
  and grants the runtime network host effect.
- `../../docs/ail/20-standard-library-and-packages.md`: standard-library
  inventory and proof commands.

## Expected Replay Artifacts

Run the focused conformance command:

```bash
cargo run -- ail-conformance examples/ail_std_effects.ail --artifact-dir /tmp/ail-std-effects-conformance
```

Run the standard-library package artifact test:

```bash
cargo test cli_ail_stdlib_packages_have_checked_package_artifacts --test ail_toolchain
```

Useful artifacts include the conformance report, dependency report, and
checked Core output for the accepted effect fixture.

## Rejected Fixtures

This package currently has no local rejected fixture. v0.3 should add rejected
fixtures for undeclared resources, write effects without declared write
support, network effects without host capability, and effects that omit
`ResourceRead`, `ResourceWritten`, or `NetworkMessageSent` traces.

## Next Example To Read

Read `../ail_std_runtime.ail/README.md` next to see this package imported by a
runtime package with explicit capability grants.

## v0.3 Learning Signal

AIL Standard Effects needs more failure-oriented teaching. v0.3 should connect
effect declarations to host capability checks, OS/system boundaries, and
repair tutorials for missing or over-broad effect declarations.
