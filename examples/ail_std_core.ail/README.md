# AIL Standard Core Example

## Purpose

`ail_std_core.ail` is the smallest standard-library package. It teaches the
shape of primitive contracts that other AIL standard packages can import and
reuse.

The package currently defines `Identity.copy`, which returns a value unchanged
and records `IdentityCopied`. That makes it the baseline for checking standard
library package metadata, accepted local fixtures, Core lowering, and stable
trace names before moving to richer generic or effect packages.

## Concepts Taught

- Standard-library package metadata and `ail.std.*` naming.
- Generic function inputs and outputs.
- Primitive contracts that preserve values without mutation.
- Trace coverage for pure helper functions through `IdentityCopied`.
- Local accepted fixtures under `examples/accepted`.
- v0.3 need for a larger primitive contract surface.

## Files To Inspect

- `ail-package.md`: package identity, version, Application profile, and schema
  support.
- `spec.ail-spec.md`: canonical `ail.std.core` specification.
- `examples/accepted/identity-copy-minimal.ail-spec.md`: minimal accepted
  fixture for `Identity.copy`.
- `../ail_std_collections.ail/ail-package.md`: imports this package as `Core`.
- `../../docs/ail/20-standard-library-and-packages.md`: standard-library
  package inventory and proof commands.

## Expected Replay Artifacts

Run the standard-library structural tests:

```bash
cargo test cli_ail_stdlib_packages_have_checked_package_artifacts --test ail_toolchain
```

For focused conformance:

```bash
cargo run -- ail-conformance examples/ail_std_core.ail --artifact-dir /tmp/ail-std-core-conformance
```

Useful artifacts include the conformance report, checked package artifact, and
manifest fingerprint written under the artifact directory.

## Rejected Fixtures

This package currently has no local rejected fixture. v0.3 should add rejected
fixtures for missing `IdentityCopied` trace coverage, generic output drift,
and non-identity behavior that mutates or drops the input value.

## Next Example To Read

Read `../ail_std_collections.ail/README.md` next to see `Core` imported by a
generic collection package.

## v0.3 Learning Signal

AIL Standard Core is intentionally small. v0.3 should expand it into a fuller
primitive contract package with accepted and rejected examples for `Text`,
`Bool`, `Int`, `Money`, `Time`, `Duration`, and identity-preserving helpers.
