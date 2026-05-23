# AIL-Core Semantic IR

## Purpose

AIL-Core is the canonical typed semantic graph for accepted AIL programs. It is
the artifact the trusted checker validates and the artifact every projection
must preserve.

## Graph Model

AIL-Core is a directed attributed graph. Nodes represent semantic entities.
Edges represent typed relationships such as ownership, reads, writes,
requires, guarantees, calls, lowers-to, and came-from.

The graph is not a syntax tree. It represents meaning after names, aliases,
imports, defaults, and profile-specific elaboration have been normalized.

## Stable Identity

Every node has a stable ID scoped by package, module, profile, and declaration.
IDs must be stable across formatting changes and projection round trips.

Example:

```text
node action:support-ticket/CloseTicket
node thing:support-ticket/Ticket
edge action:support-ticket/CloseTicket reads field:support-ticket/Ticket.status
```

## Node Kinds

Initial node kinds are:

```text
Application, Thing, Field, Action, Step, Tool, SystemComponent, Resource,
Event, Rule, View, Value, Capability, Permission, Effect, Failure, Guarantee,
Secret, Approval, Trace, Region, Layout, Allocation, ExecutionContext,
InterruptPriority, InterruptMask, SchedulerTask, Lowering, Diagnostic,
Function, Input, Output, Branch, Loop, Match, Call, Return, Package,
ExternalBinding, Prompt, CorpusFixture
```

Profiles may add specialized nodes only when they map back to these core
concepts or are accepted through the evolution protocol.

## Edge Kinds

Initial edge kinds include:

- contains
- has-field
- uses-resource
- requires
- reads
- writes
- calls
- performs
- targets-resource
- authorizes-resource
- owns-resource
- borrows-resource
- mutably-borrows-resource
- uses-region
- in-region
- uses-layout
- layouts-resource
- uses-allocation
- allocates-resource
- uses-lock-guard
- guards-resource
- uses-lock-resource
- runs-in-context
- uses-interrupt-priority
- prioritizes-context
- uses-interrupt-mask
- masks-context
- schedules-task
- task-runs-in-context
- uses-task-priority
- prioritizes-task
- uses-task-timing
- times-task
- emits
- may-fail-with
- handles-failure
- guarantees
- requires-approval
- protects-secret
- grants-permission
- records-trace
- has-provenance
- projects-to
- lowers-to
- depends-on

Each edge kind defines source node kinds, target node kinds, cardinality, and
checker rules.

The machine-checkable node and edge catalog is `18-ail-core-schema.md`.

## Attributes

Attributes hold typed scalar or structured values. They include names, English
labels, types, default values, visibility, effect class, ordering, diagnostic
text, source span, package version, and conformance level.

Attributes are normalized before equivalence checks. Equivalent aliases,
ordering differences, and formatting differences must not change graph meaning.

## Provenance

Every accepted node or edge has Provenance. Provenance records the AIL-Spec
paragraph, no-code view item, package default, agent inference, or evolution
proposal that produced it.

Provenance must distinguish confirmed human intent from agent inference.

## Normalization

Normalization resolves imports, aliases, type names, defaults, package versions,
profile expansions, and stable ordering. The normalized graph is the input to
strong equivalence.

Normalization must be deterministic. Two equivalent programs must normalize to
the same graph even if they were authored through different projections.

## Equivalence

Strong equivalence means two artifacts normalize to the same AIL-Core graph,
allowing stable ordering and approved alias normalization.

Behavioral equivalence means they produce the same observable behavior,
diagnostics, permissions, effects, failures, guarantees, and traces for the
same inputs.

Explanation equivalence means projections communicate the same semantics to
humans and pass automated semantic checks.

## Serialization Expectations

The first implementation may use a readable text serialization for tests and
review. A mature implementation should define a canonical package format with:

- graph nodes and edges
- typed attributes
- provenance
- package metadata
- profile metadata
- conformance declarations
- deterministic ordering
- stable hashes

## Canonical Schema Boundary

The canonical package envelope, graph patch schema, stable serialization order,
normalization algorithm, and semantic hash algorithm are defined in
`18-ail-core-schema.md`. Implementations may expose readable text for review,
but conformance compares normalized schema data, not hand-written prose.

## Graph Patch Boundary

AIL-Spec edits, AIL-Flow visual edits, agent repairs, and migration proposals
all become graph patches. A patch must declare:

- base graph hash
- operations over nodes, edges, or attributes
- provenance for every operation
- human confirmation state when semantics change
- expected diagnostics or accepted status

A patch is accepted only after the resulting graph validates against the schema
catalog and checker rules.

## Package Schema

Package metadata is part of the semantic boundary. Package name, version,
profile, imports, feature flags, prompt-pack version, capability grants,
target support, and conformance level affect validation and hashing.
