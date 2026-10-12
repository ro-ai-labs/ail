rung: R024-lru-cache
tier: 2
action: Analyze
targets: vm,native
blocked-on: pure AIL LRU implementation

# R024 — Immutable lossless Json least-recently-used cache

Analyze consumes explicit data.request:Text and selects only out.result:Bytes,
encoding the exact ordered compact runRequest result as UTF-8; trace is Simulated.
Independent reference.py models create/get/put/run/evaluate_request with ordered
Json pairs, opaque numeric lexemes and OrderedDict recency. A separately verified
list model and hand expectations precede all AIL source. Direct invalid valid-Json
state shapes, negative typed capacity and malformed operation shapes raise the
existing VALUE category. The wrapper returns exact ordered fields ok,state,events,
error with unchanged REQUEST/CAPACITY/OPERATIONS response strings. Negative capacity
is checked explicitly; only operation-shape validation catches VALUE. Transition
execution and result construction are outside that catch; internal VALUE propagates.

States contain exactly capacity then entries, LRU through MRU. Capacity normalizes
-0, accepts signed-i64 maximum without preallocation and rejects fractions,
exponents, booleans or out-of-range tokens. Hits promote; cached null is a hit;
misses preserve order; full-cache replacements promote without unrelated eviction.
Every retained payload keeps duplicate ordered pairs, number lexemes, decoded
Unicode, array order and independent copied storage. Selected duplicate request
and operation fields reject, while unrelated duplicates and get value duplicates
are ignored only after complete Json syntax/Unicode validation.

Named cases pin domain priorities, controls and pointer-like exact keys, Unicode
normalization distinctions, fake report separators, 8192-scalar keys and values,
5000-digit integers, huge exponents, duplicate payloads, 32 nested arrays,
capacity128 with256 distinct puts plus3 gets, capacity16 filled then512 mixed
accesses, capacity0/1/max and all required transitions. These sizes are minimum
success examples, not new resource caps. Existing fatal bounds remain observable.

Binding contract: docs/superpowers/specs/2026-10-12-lru-cache-design.md.
Task1 inputs/evidence are frozen and independently reviewed before implementation.
