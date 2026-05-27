# Support Shared Example

## Purpose

This support package defines reusable support-domain declarations imported by
the composed support-ticket package.

## Concepts Taught

- Shared packages that provide types and failures to another package.
- `Shared.User` as a namespaced import target.
- Reusable `PermissionDenied` failure behavior that avoids leaking secret
  values.

## Files To Inspect

- `ail-package.md`: support-shared package metadata.
- `spec.ail-spec.md`: shared user and permission-denied failure definitions.
- `../support_composed.ail/ail-package.md`: imports this package as `Shared`.
- `../support_composed.ail/spec.ail-spec.md`: uses `Shared.User` in the
  composed ticket model.

## Expected Replay Artifacts

This package is replayed through `support_composed.ail` catalog entries
`example-10` through `example-19`, where import resolution appears in checked
Core, dependency reports, `dependency-review.txt`, bytecode, VM traces, and
story anchors.

## Rejected Fixtures

This package has no package-local rejected fixtures. The related rejected path
is covered by `missing_registry_import.ail`, which demonstrates an unresolved
registry import. v0.3 should add local rejected fixtures for missing shared
types and incompatible package versions.

## Next Example To Read

Read `support_composed.ail/README.md` next for the accepted import path.

## v0.3 Learning Signal

Shared support examples need dependency review views that explain which
imported package owns each type, failure, and permission rule. The dependent
`support_composed.ail` corpus entries now emit deterministic dependency reviews
for `example-10` through `example-19`; the next bar is package-local rejected
fixtures for missing shared types and incompatible package versions.
