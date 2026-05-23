# AIL Round-Trip And Equivalence

## Purpose

Round-trip and equivalence rules define when different AIL artifacts represent
the same program. They protect the language from projection drift, agent
hallucination, formatter bugs, and no-code editing ambiguity.

## Required Round Trips

AIL requires these round trips:

```text
AIL-Spec -> AIL-Core -> AIL-Spec
AIL-Core -> AIL-Spec -> AIL-Core
AIL-Core -> AIL-Flow view -> AIL-Core patch
AIL-Core -> trace -> explanation
AIL-Core -> lower-level artifact -> semantic explanation
AIL-Spec Canonical -> model output -> AIL-Spec Canonical
AIL-Core -> AIL-Bytecode -> AIL-Core explanation
```

Each round trip declares what must be identical, what may be normalized, and
what must be rechecked.

## Strong Equivalence

Strong Equivalence means two artifacts normalize to the same AIL-Core graph,
allowing deterministic ordering and approved alias normalization.

Strong equivalence is the default authority for source and projection
round-trips.

## Behavioral Equivalence

Behavioral Equivalence means two artifacts produce the same observable behavior,
diagnostics, permissions, effects, failures, guarantees, trace events, and
approved external calls for the same inputs and environment assumptions.

## Explanation Equivalence

Explanation Equivalence means two human-facing projections communicate the same
behavior, rules, permissions, effects, failures, guarantees, and trace causes to
reviewers and automated semantic checks.

## Embedding Distance As Regression Signal

Embedding distance may help detect unexpected explanation drift. However,
embedding distance is not the compiler authority. Graph and behavioral checks
remain authoritative.

## Equivalence Failures

An equivalence failure must report:

- compared artifacts
- normalized node or edge difference
- behavior or trace difference
- source provenance
- suggested repair
- whether the failure blocks compilation or only blocks a projection update

## Normalization Algorithm

Round-trip checks use the normalization algorithm from
`18-ail-core-schema.md`:

1. Resolve imports and aliases.
2. Expand package and profile defaults.
3. Normalize type names.
4. Normalize stable IDs.
5. Sort unordered nodes and edges.
6. Preserve semantic order for steps, branches, parameters, and traces.
7. Remove projection-only layout metadata.
8. Compute semantic hash.

## Semantic Diff Format

```json
{
  "before_hash": "ail-core:fnv64:...",
  "after_hash": "ail-core:fnv64:...",
  "status": "different",
  "node_deltas": [],
  "edge_deltas": [],
  "attribute_deltas": [],
  "projection_loss": [],
  "diagnostics": []
}
```

Blocking failures change behavior, permissions, effects, failures, guarantees,
traces, safety class, package identity, or backend obligations. Non-blocking
failures affect only friendly wording or visual layout and must still be
reported.

## Graph Isomorphism Constraints

Graph isomorphism may rename projection-local IDs, but it may not rename stable
package IDs, semantic node names, edge kinds, type names, permissions, effects,
failures, guarantees, trace IDs, package imports, or target bindings.

## Projection Loss Rules

A projection may omit visual layout coordinates and friendly wording. A
projection may not omit secrets, permissions, effects, failure handling,
approval gates, trace obligations, or target lowering requirements.

## Explanation Equivalence Rubric

An explanation is equivalent when it preserves:

- actor and action
- data read and written
- permission and approval checks
- external calls
- failures and compensation
- guarantees
- trace cause
- safety class

Embedding distance may flag drift, but graph and trace checks decide
acceptance.

## Conformance Tests

Conformance tests include canonical render/reparse checks, graph normalization
checks, projection patch checks, invalid example checks, diagnostic checks,
runtime trace checks, and lower-level behavioral checks.

Every required round trip has:

- accepted fixture
- rejected fixture
- diagnostic repair example

The initial fixture inventory is under `corpus/roundtrip/`.
