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

Checker obligations:

- `Match` over `Option<T>` must cover `Some` and `None`
- `Some` payload type must match `T`
- `Option.map` mapper must be callable with `T`
- no effectful mapper is accepted unless the function signature declares the
  effect
