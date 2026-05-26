# AIL Standard Collections Example

## Purpose

`ail_std_collections.ail` teaches generic standard-library data types. It
defines `Option<T>`, `Result<T,E>`, `List<T>`, `Map<K,V>`, `Set<T>`, and the
`Option.map` function.

This package is the main replay family for schema-shaped standard-library
prompts. It proves that AIL can preserve generic type names, variants,
function requirements, traces, stored prompt transcripts, checked Core,
bytecode, and user-story artifacts across many prompt surfaces.

## Concepts Taught

- Generic type declarations and variant payloads.
- `Option<T>` and `Result<T,E>` as reusable reviewable contracts.
- Collection shapes for `List<T>`, `Map<K,V>`, and `Set<T>`.
- `Option.map` behavior over `Some(value)` and `None`.
- Trace coverage through `OptionMapEvaluated`.
- Rejected generic fixtures through `invalid-generic-variant-payload`.

## Files To Inspect

- `ail-package.md`: imports `../ail_std_core.ail compatible ^0.2 as Core`.
- `spec.ail-spec.md`: canonical generic collection specification.
- `examples/accepted/option-map-minimal.ail-spec.md`: accepted minimal
  `Option.map` fixture.
- `examples/rejected/invalid-generic-variant-payload.ail-spec.md`: rejected
  variant payload fixture.
- `../examples.md`: entries `example-0` through `example-9` replay the
  standard collection package across prompt surfaces.
- `../stories/example-0.md` through `../stories/example-9.md`: user-story
  views for the collection replay family.

## Expected Replay Artifacts

Replay the release corpus:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-std-collections-examples --release-evidence
```

Useful artifacts include:

- `examples/example-0/checked.ail-core.txt`
- `examples/example-0/artifact.ailbc.json`
- `examples/example-0/user-story.txt`
- `examples/example-9/checked.ail-core.txt`

For focused conformance:

```bash
cargo run -- ail-conformance examples/ail_std_collections.ail --artifact-dir /tmp/ail-std-collections-conformance
```

## Rejected Fixtures

The package includes `examples/rejected/invalid-generic-variant-payload.ail-spec.md`.
That fixture verifies that generic variant payloads stay structured and typed
instead of collapsing into loose text.

v0.3 should add rejected fixtures for missing `OptionMapEvaluated`, wrong
`None` behavior, dropping `Result<T,E>`, and changing generic names during
story/spec round-trip.

## Next Example To Read

Read `../ail_std_effects.ail/README.md` after this package to move from pure
generic values into declared read/write and network effects.

## v0.3 Learning Signal

AIL Standard Collections is replay-rich but still too narrow. v0.3 should add
map/filter/fold examples, stronger semantic anchors for generic names, and
story amendments that change collection behavior while preserving typed
variant structure.
