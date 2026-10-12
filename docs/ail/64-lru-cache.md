# Immutable least-recently-used cache

The [LRU library](../../examples/ail_std_lru.ail/README.md) is ordinary pure AIL.
The [application](../../examples/lru-cache/README.md) provides Analyze and bounded
raw stdin Run, exact samples and source/STE/native commands. Import package
`ail.std.lru` 0.1.0 (Application profile) as `LRU` from `examples/ail_std_lru.ail`.
The import path is relative to the caller.

| API | Exact result |
| --- | --- |
| `create(capacity: Int) -> Json` | `{"capacity":C,"entries":{}}` |
| `get(state: Json, key: Text) -> Json` | `{"state":S,"hit":H,"value":V}` |
| `put(state: Json, key: Text, value: Json) -> Json` | `{"state":S,"replaced":R,"evicted":E}` |
| `run(capacity: Int, operations: Json) -> Json` | `{"state":S,"events":E}` |
| `runRequest(text: Text) -> Json` | `{"ok":true,"state":S,"events":E,"error":""}` or `{"ok":false,"state":null,"events":[],"error":CATEGORY}` |

State has exactly two unique decoded members, capacity and entries. Capacity is a
nonnegative strict signed-i64 integer; -0 normalizes to 0. Entries is an object
with unique exact Unicode names, in LRU to MRU order, and count at most capacity.
Input outer member order is unrestricted; returns always put capacity then entries.
Extra/missing/duplicate fields, bad types, overfull entries or duplicate entry names
raise VALUE. An absent-key lookup still validates the entire supplied state.

A get hit returns the payload and promotes its key to MRU. A miss returns false/null
and preserves recency. Cached null returns true/null. Put replacement promotes and
returns replaced true/evicted null, without evicting an unrelated member. An absent
insertion at full positive capacity evicts exactly the prior LRU and reports
`{"key":K,"value":V}`; otherwise eviction is null. Zero capacity stores nothing and
reports replaced false/evicted null. Retain each returned state for subsequent calls;
retained original states and payloads stay unchanged. run starts empty every time.

Operations validate fully before execution, in encounter order. Each operation
has unique decoded string op/key; op is lowercase get or put. Put requires one
unambiguous value of any Json kind. Get ignores value, including duplicates.
Unrelated fields/duplicates are ignored after whole-document validation. Escaped
aliases of selected member names count as duplicates. Events remain in operation
order: `{"op":"get","key":K,"hit":H,"value":V}` or
`{"op":"put","key":K,"replaced":R,"evicted":E}`. They contain results/evictions,
not a full state snapshot. The final state appears once.

Request wrapper syntax, Unicode, root, selected names/types and capacity i64 range
validate first; failures return REQUEST. Thus negative capacity with nonarray
operations is REQUEST. A valid wrapper with negative capacity returns CAPACITY,
even with bad operations. Remaining invalid shapes return OPERATIONS, without
partial events. Direct APIs use VALUE for domain errors. Malformed typed transports
may fail JSON/TEXT before recovery. runRequest catches VALUE only around operation
validation; transitions and result construction occur outside that catch. Fatal
errors and unexpected internal failures propagate.

Keys include empty, NUL, slash, tilde, numeric-looking and Unicode normalization
variants. Payload arrays and objects preserve order and duplicate pairs; numbers
retain lexemes such as -0, 1.2500e-2, 1E+0003 and 5000-digit tokens without numeric
conversion. Strings retain decoded scalars. Canonical encoding normalizes escape
spelling and insignificant whitespace: quote/backslash/tab/CR/LF use short escapes,
other C0/DEL/C1 controls use lowercase Unicode escapes, remaining scalars are UTF-8.

Analyze selects only UTF-8 Bytes, with trace Simulated. Run consumes one UTF-8
request through EOF and emits compact Json plus one LF, with trace Completed.
`--stdio` grants input/output and suppresses automatic reports. All domain results
exit zero with empty stderr. Empty/malformed JSON and two documents return REQUEST;
invalid UTF-8 and missing grant are fatal. Read, parse, validate, execute, serialize
and append LF all precede the only write. OS write failure is not transactional.

Immutable Json operations, validation, state transitions and event histories copy
values into cumulative arenas. This implementation makes no O(1), reclamation,
unbounded-history or generic Map claim. Existing limits are 1 MiB raw/encoded
Text/Json, 65,536 Json values, 64 containers/value frames and cumulative 64 MiB
Text and Json/list arenas, plus execution fuel. Wrapping payloads consumes depth.

Measured success includes capacity128 with256 puts/three gets, capacity16 with512
mixed accesses after filling, separate8192-scalar ASCII/Unicode keys and values,
5000-digit numbers/large exponents,32 nested arrays, max-i64 capacity with few entries,
zero capacity and pointer/control keys. A small valid request padded to exactly
1 MiB succeeds on stdin;1 MiB+1 fails TEXT-LIMIT. Request nesting67 and a63-array
payload wrapped in returned put state fail JSON-LIMIT. A request that stores then
evicts a 61-array payload parses at 64 containers but its 65-container result
fails JSON-LIMIT before writing; the 60-array control returns at 64 containers. An8192-scalar value plus30
gets succeeds;140 gets exhausts Json allocation.3000 get operations exhaust
validation fuel. Direct consumer observations include1400 absent gets with8192
scalars succeeding and2200 exhausting fuel;700 with16384 scalars succeeds while
1300 exhausts Text allocation.70 recursive consumer frames fail DEPTH. These are
observations of complete constructions, not portable maximum history counts.

The [permanent independent suite](../../scripts/ail_lru_differential.py) compares
complete expected values on fourteen app and fourteen direct routes, source/saved
Core/rendered Spec through VM/native directly and via STE, plus separately executed
and compiled saved bytecode. Every frozen request also runs as actual stdin.
Successful wrong output from an isolated get-promotion mutation is rejected on
both backends. Support registration adds the library as the33rd suffix package;
all128 catalog/story/request/response entries remain unchanged.
