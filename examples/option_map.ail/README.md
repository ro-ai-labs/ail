# Option Map Example

## Purpose

`option_map.ail` is the focused transform example for `Option<T>` and
`Option.map`. It is intentionally smaller than `ail_std_collections.ail` and
uses UI-tagged prompt surfaces to show how a simple generic transform can be
reviewed as a route, form, or state-flow building block.

The package is useful when checking whether AIL preserves one precise
operation through prompt capture, checked specification, checked Core,
bytecode, and story regeneration without hiding the generic behavior inside a
larger standard-library package.

## Concepts Taught

- `Option<T>` as a generic value contract.
- `Some(value)` and `None` behavior.
- `Option.map` requirements over input option and mapper function.
- `OptionMapEvaluated` trace coverage.
- UI-tagged replay metadata through `ui.form`, `ui.route`, and `ui.state`.
- Semantic anchors that keep the generic operation visible in story views.

## Files To Inspect

- `ail-package.md`: Application profile metadata and collection-transform
  feature declaration.
- `spec.ail-spec.md`: the focused `Option<T>` and `Option.map` specification.
- `../examples.md`: entries `example-20` through `example-24` exercise the
  package over interview, requirements, spec-draft, core-draft, and
  diagnostic-repair prompt surfaces.
- `../stories/example-20.md` through `../stories/example-24.md`: story views
  with anchors for `Option<T>`, `Option.map`, `OptionMapEvaluated`, and the
  UI surface tags.
- `../ail_std_collections.ail/README.md`: the broader standard-library version
  that adds `Result<T,E>`, `List<T>`, `Map<K,V>`, and `Set<T>`.

## Expected Replay Artifacts

Replay the corpus with release evidence enabled:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-option-map-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-20/checked.ail-core.txt`
- `examples/example-20/artifact.ailbc.json`
- `examples/example-20/user-story.txt`
- `examples/example-20/ui-semantic-tags.txt`
- `examples/example-24/checked.ail-core.txt`
- `examples/example-24/user-story.txt`

For a focused package check:

```bash
cargo run -- ail-conformance examples/option_map.ail --artifact-dir /tmp/ail-option-map-conformance
```

## Rejected Fixtures

This package does not yet include package-local rejected fixtures. v0.3 should
add rejected specs for dropping `OptionMapEvaluated`, returning `Some` for a
`None` input, changing `Option<T>` to a concrete-only type, and losing the
mapper function requirement.

## Next Example To Read

Read `../ail_std_collections.ail/README.md` after this guide to see the same
generic operation inside the broader standard library package. Then read
`../ui_workflow.ail/README.md` to move from UI-tagged metadata into real UI
routes, forms, dashboards, accessibility checks, and Wasm target reports.

## v0.3 Learning Signal

Option Map now has package-local guidance, story anchors, and deterministic
`ui-semantic-tags.txt` replay artifacts for the exact generic transform plus
its `ui.form`, `ui.route`, and `ui.state` prompt surfaces. The next bar is a
real UI wrapper that calls the checked transform and package-local rejected
fixtures for losing `OptionMapEvaluated`, the mapper requirement, or the
generic `Option<T>` contract.
