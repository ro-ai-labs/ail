# Immutable LRU cache library

## Purpose

`ail.std.lru` 0.1.0 supplies reusable pure cache states, plus a validated operation
history runner. Import this Application package as `LRU`. Every function accepts
ordinary AIL values and returns Json; no cache lives in host or global storage.

## Concepts Taught

`create(capacity: Int) -> Json`, `get(state: Json, key: Text) -> Json`,
`put(state: Json, key: Text, value: Json) -> Json`,
`run(capacity: Int, operations: Json) -> Json` and
`runRequest(text: Text) -> Json` separate typed state reuse from request recovery.
State is exactly capacity then entries, ordered from LRU to MRU. Hits promote;
misses preserve recency. A cached null is a hit with null value. Replacements
promote without unrelated eviction; a full insertion evicts the old LRU.
Zero capacity stores nothing. Valid reordered states and capacity -0 normalize.
Exact decoded Unicode names match without normalization; pointer tokens escape
~ before /. Payload Json retains number lexemes, ordered duplicate pairs and
strings. Caller states and payloads remain available after returned copies change.

Get returns state/hit/value; put returns state/replaced/evicted. Eviction is null
or key/value. Run returns state/events, with one event per operation. All operation
shapes validate before transitions. Each operation has unique string op/key;
put also has one value. Get ignores value fields, including duplicates. Unrelated
fields and duplicates are ignored after complete Json validation.

## Files To Inspect

[spec.ail-spec.md](spec.ail-spec.md) defines validation, canonical state construction,
pointer escaping and shared lookup/insert transitions. Public get/put normalize
caller states; executeValidated reuses lookup/insert after complete operation
validation. [ail-package.md](ail-package.md) binds version, profile and features.
The [application](../lru-cache) shows Analyze and complete UTF-8 Bytes reporting.

## Expected Replay Artifacts

Use the unchanged compiler to save checked Core, render Spec, export STE and lower
bytecode from the app. VM and native Analyze select only out.result=hex:... and
trace Simulated; decoded bytes are the compact result Json. Direct probes can
select Bytes of all five API results and retained inputs with trace Probed.

Each immutable mutation and successful function return copies its result into
bounded arenas. State scans, validation, key sorting and ordered JSON mutation
have real costs; building event history repeatedly copies prior events. Existing
1 MiB values, 65,536 Json values, 64 container/frame limits, cumulative 64 MiB
Text and List/Json arenas and instruction fuel apply. Overwrites do not refund
allocations. Capacity is an entry bound and does not preallocate; signed-i64
maximum can hold a few entries. This API makes no O(1) or unbounded-history claim.

## Rejected Fixtures

Negative typed capacity, invalid valid-Json states or operation shapes raise VALUE.
States reject extra/missing/duplicate outer fields, wrong capacity/type, duplicate
entry names or more entries than capacity. Decoded escaped aliases are duplicates.
Payload duplicates remain valid. Malformed typed transports and fatal runtime
bounds retain their existing diagnostics and are not promised body-level recovery.

runRequest returns ok/state/events/error. Wrapper failures yield REQUEST; valid
wrappers with negative capacity yield CAPACITY; invalid operation shapes yield
OPERATIONS. Failed responses have false/null/[] and that error string. Only the
operation validator is protected by VALUE recovery: unexpected transition errors
propagate. Successful responses have true, final state, full events and empty error.

## Next Example To Read

Read [lru-cache](../lru-cache) for current Analyze use, then
[json-inspection](../json-inspection) for generic key traversal and
[json-pretty](../json-pretty) for lossless canonical Json transport.

## v0.3 Learning Signal

An explicit immutable Json API provides persistent state reuse with ordered keys,
opaque values and shared transitions through checked source/Core/STE/VM/native.
Useful-size checks compare complete model states/events for capacity 128 with 256
puts plus three gets, capacity 16 with 512 mixed accesses after filling, separate
8192-scalar ASCII/Unicode keys and values, 5000-digit numeric/exponent payloads
and 32 nested arrays. These are demonstrated examples, not interface caps.

Observed boundaries remain fatal: a 63-array payload can be valid by itself yet
exceed 64 containers when attached to returned state; a 3000-get operation validator
exhausts ordinary instruction fuel. A history with an 8192-scalar value and 30 gets
succeeds, while 140 gets exhaust Json allocation. Repeated absent lookup with an
8192-scalar key succeeds 1400 times; 2200 reaches consumer fuel. With a 16384-scalar
key, 700 repetitions succeed and 1300 exhaust cumulative Text allocation. These
counts describe specific consumers, not universal maximums. Native diagnostic
wording differs from VM wording while the error families agree; fatal failures
never become request-domain successes.

It exercises existing language features without adding a compiler opcode or a
host cache. Bounded cumulative copying remains observable; this example does not
establish generic executable Map support or complete Python compatibility.
