# Missing Registry Import Example

## Purpose

This package is a rejected package-resolution example. It intentionally imports
`shared-lib@0.1.0 as Shared` while the registry index only names `other-lib`,
so the replay catalog can preserve unresolved registry-import diagnostics.

## Concepts Taught

- Registry-backed imports and package resolution.
- Diagnostic stories that preserve a package loader failure.
- The difference between a catalog entry that is useful because it fails and a
  package that should compile.

## Files To Inspect

- `ail-package.md`: declares the missing `shared-lib` import and registry path.
- `registry/ail-registry.md`: registry index that does not contain
  `shared-lib`.
- `spec.ail-spec.md`: minimal resolver action and `SharedImportResolved`
  trace anchor.
- `../stories/example-107.md`: diagnostic story for package-resolution
  failure.

## Expected Replay Artifacts

`example-107` writes `diagnostics.txt`, `user-story.txt`, request and response
fingerprints, and semantic-anchor preservation evidence for
`missing-registry-import`, `shared-lib`, `registry index`, and
`SharedImportResolved`.

## Rejected Fixtures

The package itself is the rejected fixture. The expected diagnostic mentions
that registry import `shared-lib as Shared` was not found in the registry
index.

## Next Example To Read

Read `support_composed.ail/README.md` next for the accepted local-import
variant that resolves `support_shared.ail` successfully.

## v0.3 Learning Signal

Package-resolution examples need repair tutorials that show how to add the
missing registry entry, change the import, and replay the corrected package.
