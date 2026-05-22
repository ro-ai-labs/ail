# AIL Types, Values, Permissions, And Effects

## Purpose

AIL types, values, permissions, and effects define what programs can know,
change, call, reveal, allocate, lower, and explain.

## Core Types

Initial core types include:

- Text
- Bool
- Int
- Decimal
- Money
- Time
- Duration
- State
- List<T>
- Map<K, V>
- Option<T>
- Result<T, E>
- Secret<T>
- Thing types
- Tool contract types
- Region and layout types for AIL-System

## Structured Values

Structured values may be things, lists, maps, options, results, tool calls,
trace records, diagnostics, graph patches, and compiler-pass inputs or outputs.
The checker validates every field and nested value against its declared type.

## Option And Result

`Option<T>` represents presence or absence. `Some(value)` contains a `T`;
`None` contains no value.

`Result<T, E>` represents success or failure. `Success(value)` contains a `T`;
`Failure(value)` contains an `E`.

Actions must explain how they handle absent values and failures when those
states affect behavior.

## Secret Values

`Secret<T>` marks a value as sensitive. Secret values remain typed, but they
cannot flow into a response, log, trace explanation, no-code view, or agent
message unless a permitted redaction or disclosure rule exists.
For agent tools, a secret output is treated as disclosure and requires an
explicit reveal or disclose permission.

## Permissions

A Permission declares who or what may read, write, call, approve, own, reveal,
or lower a value or capability. Permissions attach to actions, tools, views,
runtime components, and compiler passes.

## Capabilities

A Capability is a runtime-enforced grant derived from permissions. A tool or
action may request a capability, but the runtime checks whether the request is
allowed before any effect occurs.

## Effects

An Effect describes an observable change or interaction:

- read state
- write state
- delete state
- call external system
- emit event
- send message
- allocate memory
- mutate device state
- lower semantic graph to code
- publish package artifact

Effects must be declared so they can be checked, reviewed, traced, and lowered.

## Ownership And Sharing

Ownership and Sharing connect high-level permissions to systems-level safety.
At the application level, ownership can mean who controls a record or approval.
At the systems level, ownership can mean which component may mutate memory, when
a borrow is valid, which region holds a value, and which capability permits a
device or OS call.

The same language concept should explain both "a manager owns this approval"
and "the network driver owns this buffer until packet handling completes."

## Human Explanation Rules

Every type, permission, capability, and effect must be explainable in structured
English. If a generated artifact performs a write, disclosure, allocation, or
external call, the user-facing projections must be able to say why it is
allowed and which rule introduced it.
