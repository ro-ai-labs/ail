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

## Conformance Tests

Conformance tests include canonical render/reparse checks, graph normalization
checks, projection patch checks, invalid example checks, diagnostic checks,
runtime trace checks, and lower-level behavioral checks.
