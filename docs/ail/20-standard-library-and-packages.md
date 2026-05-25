# AIL Standard Library And Packages

## Purpose

The standard library and package model define reusable AIL semantics without
turning libraries into hidden compiler behavior. Library types, actions,
effects, capabilities, prompts, views, diagnostics, and conformance fixtures
are package artifacts.

## Package Manifest

Every AIL package contains `ail-package.md`:

```text
name: support-ticket
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things, actions, failures, guarantees, traces
imports: ail.std.core@0.1 as Core
conformance: first-slice
prompt-pack: ail.prompts@0.1
```

Required fields:

- `name`
- `version`
- `profile`
- `entry`
- `features`
- `conformance`

Optional fields:

- `imports`
- `capability-grants`
- `prompt-pack`
- `target-support`
- `schema-version`
- `safety-level`

The current package loader preserves `prompt-pack`, `target-support`,
`schema-version`, `safety-level`, and `capability-grants` as package metadata
and renders them into AIL-Core. Capability-grant enforcement remains checker
and package-resolver work.

## Import Rules

Imports bind package names to explicit versions or compatible ranges:

```text
imports: ail.std.collections@0.1 as Collections
imports: payments.stripe@2.4 compatible ^2.4 as Stripe
```

The checker resolves imports before normalization. Ambiguous package names,
unbounded major versions, missing capability grants, and conflicting aliases
are rejected.

The current local package loader implements exact path-version imports:

```text
imports: ../shared@0.1.0 as Shared
```

For this implemented form, the loader resolves `../shared` as the package
directory, checks the loaded package `version` against `0.1.0`, and rejects the
import before checking if the versions differ. Compatible range resolution and
registry package names remain package-resolver work, not ambient checker
behavior.

It also implements the v0.2-compatible local range form:

```text
imports: ../shared compatible ^0.1 as Shared
```

For this form, the loader resolves `../shared`, rejects unbounded major ranges
such as `*`, and accepts only packages with the same major version and a
version greater than or equal to the range base. Registry package names and
cross-major compatibility policies remain package-resolver work.

The loader also exposes a package dependency report for resolved imports. The
report records the root package, resolved import alias, declared path,
requirement, resolved package name and version, source path, source hash,
capability grants, approvals, and imported effect classes. This gives v0.2 a
verifiable package-lock surface before registry resolution exists. The
`ail-build`, `ail-lower`, `ail-conformance`, and `ail-compile` artifact writers
include this report and fingerprint in their manifests when the source package
imports other packages.

## Version Compatibility

Semantic version rules:

- patch version: diagnostics, examples, prompt wording, and non-semantic
  projections may change
- minor version: optional declarations may be added
- major version: canonical graph meaning, type signatures, effect classes,
  stable IDs, or accepted behavior may change

Package hashes include resolved dependency versions.

## Capability Grants

Packages do not receive ambient authority. A package declares requested
capabilities, and the importing package grants them explicitly:

```text
capability-grants:
  - package: payments.stripe
    capability: call external payment provider
    effects: [network, money]
    approvals: [manager approval over USD 500]
```

The checker rejects imported actions that use effects outside their grants.
For resolved imports in the current v0.2 slice, the grant `package` field may
name the import alias, import path, or resolved package name. Registry fetching
and registry identity resolution remain package-resolver work.

## Standard Library Modules

Initial standard library packages:

| Package | Contents |
| --- | --- |
| `ail.std.core` | `Text`, `Bool`, `Int`, `Decimal`, `Money`, `Time`, `Duration` |
| `ail.std.collections` | `List<T>`, `Map<K,V>`, `Set<T>`, `Option<T>`, `Result<T,E>` |
| `ail.std.actions` | validation, transformation, filtering, sorting, retry policies |
| `ail.std.effects` | state, file, network, message, clock, random, process |
| `ail.std.security` | `Secret<T>`, redaction, permissions, capabilities |
| `ail.std.runtime` | trace events, diagnostics, failures, guarantees |
| `ail.std.ui` | route, view, form, table, event, component |
| `ail.std.system` | memory, layout, region, allocation, device, ABI |
| `ail.std.compiler` | graph traversal, graph patch, diagnostics, renderers |

Current v0.2 package fixtures cover the required initial package set as real
AIL packages under `examples/ail_std_*.ail`:

| Package | Fixture | Checked public surface |
| --- | --- | --- |
| `ail.std.core` | `examples/ail_std_core.ail` | `Identity.copy`, trace `IdentityCopied` |
| `ail.std.collections` | `examples/ail_std_collections.ail` | `Option<T>`, `Result<T,E>`, `List<T>`, `Map<K,V>`, `Set<T>`, `Option.map` |
| `ail.std.effects` | `examples/ail_std_effects.ail` | read/write resource actions and trace events |
| `ail.std.security` | `examples/ail_std_security.ail` | `Secret<T>` field, reveal permission requirement, capability requirement, redaction/protection edge, trace |
| `ail.std.runtime` | `examples/ail_std_runtime.ail` | runtime task action, `RuntimeUnavailable` failure, failure handling, traces |

Each required fixture has `ail-package.md`, canonical `spec.ail-spec.md`,
`conformance: v0.2`, `schema-version: ail-core.schema.v0`, `target-support:
ail-core.schema.v0=supported`, and at least one accepted conformance fixture.
The proof test is:

```bash
cargo test cli_ail_stdlib_packages_have_checked_package_artifacts
cargo test cli_ail_stdlib_import_records_dependency_report
```

## Library Authoring Rules

A library package must include:

- canonical AIL-Spec
- normalized AIL-Core
- public API declarations
- capability requirements
- effect classes
- failure mappings
- stable diagnostics
- AIL-Flow projections
- prompt-pack guidance for agent usage
- accepted and rejected fixtures
- package conformance report

## Standard Documentation Projection

Every public library declaration renders to:

- canonical structured English
- friendly summary
- AIL-Core schema fragment
- AIL-Flow block or card
- diagnostic examples
- trace examples
- target support matrix

## Conformance Fixtures

Each standard package contributes:

- accepted import fixture
- rejected unresolved import fixture
- rejected version conflict fixture
- capability grant fixture
- package hash fixture
- projection fixture
- bytecode or runtime fixture when executable

## Example: Collections Package

AIL-Spec:

```text
Package: ail.std.collections.

Type: Option<T>.

Option has variants:

- Some(value: T)
- None

Function: Option.map.

When Option.map runs:

- if the option is Some(value), the function calls mapper with value
- the function returns Some(mapped value)
- if the option is None, the function returns None
- the function records a trace event named OptionMapEvaluated
```

Current implementation status: the bootstrap parser accepts `Type: Option<T>.`
and variant bullets such as `Some(value: T)` and `None`, lowers them into
checked `Type`, `Variant`, and variant payload `Field` nodes, and preserves the
standard-library function surface through render/reparse with clean checking.
The checker recognizes single-letter generic parameters in this context.
`Option.map` also has bytecode and VM trace evidence through
`ail_standard_library_option_map_executes_collection_transform_bytecode`.
Exhaustive `Match` checking remains later standard library semantics.

Checker obligations:

- `Match` over `Option<T>` must cover `Some` and `None`
- `Some` payload type must match `T`
- `Option.map` mapper must be callable with `T`
- no effectful mapper is accepted unless the function signature declares the
  effect
