# AIL-Spec Structured English

## Purpose

AIL-Spec is deterministic structured English. It is the human-reviewable source
projection used before and after normalization into AIL-Core.

AIL-Spec should be readable by English speakers without requiring them to learn
symbol-heavy programming syntax. It should also be regular enough for an AI
Agent to generate, patch, normalize, and explain.

## Required Qualities

An AIL-Spec document must be:

- explicit about actors, data, actions, rules, failures, secrets, permissions,
  effects, guarantees, and views
- deterministic enough to elaborate into AIL-Core
- stable enough to render from AIL-Core without losing semantics
- reviewable by non-engineers
- patchable in small sections
- diagnostic-friendly when something is missing or ambiguous

## Document Shape

An application document uses this shape:

```text
The application <name> manages <purpose>.

The application stores:
- <thing>

A <thing> has:
- <field name>: <type or explanation>

When <actor/event> <action>:
- the system requires ...
- the system reads ...
- the system changes ...
- the system calls ...
- if ... fails ...
- the system guarantees ...
```

Every paragraph that introduces behavior receives stable provenance so the
derived AIL-Core nodes can point back to the human-reviewed source.

## Application Sections

An application section defines the purpose, users, stored things, external
systems, views, and top-level guarantees.

Required slots:

- purpose
- actors
- stored things
- external systems
- visible views
- global secrets
- global guarantees

## Action Sections

An action section defines one executable behavior.

Required slots:

- trigger or actor
- inputs
- preconditions
- reads
- writes
- external calls
- failures
- approvals
- guarantees
- trace expectations

The checker rejects an action that changes data without a declared permission
or reads a secret without a declared capability.

## Tool Sections

A tool section defines a capability that an AI Agent or runtime component can
request. It must name purpose, allowed use, inputs, outputs, permissions,
effects, secrets, approvals, failures, guarantees, and audit trace events.
Permission requirements are explicit grants; they are not inferred from
general precondition prose.
Approval requirements are explicit review gates; they are not inferred from
general "must not" prose.

## Failure Sections

Failures are named outcomes, not hidden exceptions. Each failure must define:

- when it can occur
- which data remains unchanged
- which compensation runs
- what the user or caller sees
- what trace event is recorded

## Secret And Permission Sections

Secrets must be named as `Secret<T>` values or structured fields that contain
secrets. Permissions must describe who or what may read, write, call, approve,
or disclose each resource.

The spec must say what data may be revealed to each audience.

## Human Confirmation Rules

The AI Agent may infer draft details, but accepted AIL-Spec must distinguish:

- human-stated facts
- agent-inferred facts
- defaults from a package or profile
- unresolved questions

Human confirmation is required before compiling an inferred rule that affects
permissions, effects, secrets, money, safety, or external calls.

## Invalid Or Ambiguous Specs

The checker rejects AIL-Spec or derived AIL-Core when required semantic slots
are missing. Examples include:

- an action says it "updates the account" without naming fields
- a tool calls an external provider without declaring effects
- a response exposes a value whose type contains `Secret`
- a failure is named but no handling or trace is defined
- two actions can write the same value concurrently without a join rule
