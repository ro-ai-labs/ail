# A reusable least-recently-used cache written in AIL

## Intent and choice

Continue the authorized Specs → checked Core → ASD-STE100-based STE → VM/native
language-development goal after the completed 22/22 JSON pretty-printer milestone.
Implement planned real-program rung R024 as a pure, reusable LRU cache library,
an operation-history runner and a bounded stdin request tool. Callers can retain
the returned cache state between calls. This is a new public library interface;
use a written design, implementation plan and independent oracle before source.
Standing continuous reversible-work authority covers this work without repeated
design/permission pauses, git mutations, dependency installation or publication.

Considered approaches: a batch-only simulator measures eviction but leaves no
reusable state API; executable generic Map would require a separate compiler
milestone and broader semantics. Prefer pure Json state with existing ordered
object mutation, exact-key lookup and ordinary functions. Removing a member then
appending it expresses MRU promotion. No host cache primitive is required.
The existing descriptive `ail.std.collections` Map declaration does not establish
executable generic-map support, and this milestone makes no such claim.

References: [recursive JSON](../../ail/42-recursive-json.md),
[JSON mutation](../../ail/43-json-mutation.md),
[value functions](../../ail/44-value-functions.md) and
[JSON inspection](../../ail/47-json-inspection.md). Existing compiler/runtime,
prior rung inputs/programs, tests and scripts stay unchanged. A reproduced defect
requires separately reviewed scope. The broader Python-replacement and AI-agent
goal remains active; this rung does not complete the alpha-publication gate.

## State and preservation

A cache state is a Json object with exactly two unique decoded member names:
`capacity` and `entries`. Input member order is unrestricted. Capacity is a strict
JSON integer in signed 64-bit range, with a nonnegative value; `-0` means zero.
Decimals, exponents, Booleans and out-of-range integers are invalid capacities.
Entries is an object with unique decoded keys, in order from **LRU to MRU**.
Its pair count must not exceed capacity. Capacity zero therefore requires empty
entries. Extra outer state fields, missing/duplicate fields, duplicate decoded
entry keys, a wrong root/type or an overfull state are invalid cache shapes.

Every returned state has exactly `capacity`, then `entries`. Capacity is encoded
as an ordinary nonnegative integer, so input `-0` becomes `0`. Entry keys remain
in the specified recency order. Values are arbitrary valid Json, including null,
Booleans, numbers, strings, arrays and nested objects with ordered duplicate pairs.
Duplicate entry keys are invalid; duplicate pairs inside a stored value are valid.

Keys match exact decoded Unicode: empty, NUL, slash, tilde, numeric-looking and
`-` keys are ordinary names; case and normalization variants remain distinct.
Payload Json preserves array order, object pair order, duplicate names, every
number lexeme and decoded strings. No numeric conversion occurs for payloads.
`-0`, `1.2500e-2`, `1E+0003`, 5,000-digit integers and large exponent spellings
must survive insertion, lookup, replacement and eviction exactly as Json values.
Insignificant whitespace and string escape spelling normalize through the existing
canonical Json encoder. Quote/backslash/tab/CR/LF use documented short escapes;
other C0/DEL/C1 controls use lowercase Unicode escapes; other scalars are literal
UTF-8. This preserves the documented Json representation, not original source bytes.

Functions are immutable: returned states cannot modify a caller's retained input
state or payload. A miss preserves entries and their order; normalization of the
outer state representation still applies. Returned events never contain hidden
mutable references. Existing Json copy/allocation semantics apply.

## Pure library API

Package `ail.std.lru` version 0.1.0, Application profile, directory
`examples/ail_std_lru.ail`, imported as `LRU`. Exact signatures:

- `create(capacity: Int) -> Json`: return `{"capacity":C,"entries":{}}`.
  Negative capacity raises existing catchable `VALUE`. Zero and signed-i64 maximum
  capacity are supported; capacity is an entry bound, not a preallocation request.
- `get(state: Json, key: Text) -> Json`: validate state, then return exactly
  `{"state":S,"hit":H,"value":V}`. A hit returns the stored value and moves
  that key to MRU. A miss leaves recency unchanged and returns `false`/`null`.
  A cached null therefore returns `true`/`null`. Invalid state shape raises `VALUE`.
- `put(state: Json, key: Text, value: Json) -> Json`: validate state, then return
  exactly `{"state":S,"replaced":R,"evicted":E}`. If key exists, replace its
  value, promote it to MRU, set replaced true and evicted null. Do not evict an
  unrelated entry on replacement. If key is absent and capacity is positive,
  insert it at MRU, evicting exactly the prior LRU entry only when full. Eviction
  is `{"key":K,"value":V}` in that order; otherwise it is null. Capacity zero
  stores nothing, reports replaced false and evicted null. Invalid state shape
  raises `VALUE`.
- `run(capacity: Int, operations: Json) -> Json`: start from create(capacity),
  validate the operation sequence below, then execute it and return exactly
  `{"state":S,"events":E}`. Every call starts empty; there is no global cache.
  Negative capacity or invalid operation shape raises `VALUE`.
- `runRequest(text: Text) -> Json`: validate the request below and return its
  stable structured result, catching only the specified domain categories.

All public functions and helpers are pure ordinary AIL. No I/O, external/process
calls, compiler opcode, host LRU implementation, changed budget, arbitrary small
capacity/operation cap, instruction-padding helper or new dependency is allowed.
Coherent helpers may share validation, pointer escaping and state transitions.
The history runner must reuse the same get/put transition implementation as the
public state API; it must not be an independent second cache algorithm.

Raw malformed Json or invalid Text in a direct typed call may fail existing
argument validation before the body can recover. `VALUE` covers negative typed
capacity and invalid shapes of valid Json values, not a promise to remap malformed
runtime transports. Existing JSON/TEXT and fatal bounds remain observable there.
Direct callers identify the invalid argument from the API context; the request
adapter below provides distinct CAPACITY and OPERATIONS response strings.

## Operations, events and error priority

Operations is an array. Each item is an object with one unambiguous string `op`
and one unambiguous string `key`. Op is exactly lowercase `get` or `put`.
Put additionally requires one unambiguous `value` of any Json kind, including null.
Get ignores any `value` member, including duplicate value members. Both operations
ignore unrelated fields and unrelated duplicate names after full Json validation.
Escaped-equivalent selected member names are duplicates and invalid. Unknown op,
wrong item/root/type, missing key/value or duplicated selected field is invalid.

Validate capacity first, then all operation shapes in encounter order before
performing any cache transitions. No partial event list is returned on a domain
failure. Existing fatal validation/allocation bounds can still interrupt validation;
the design does not promise domain errors over an already reached fatal bound.

There is exactly one event per successful operation, in request order:

- Get: `{"op":"get","key":K,"hit":H,"value":V}`.
- Put: `{"op":"put","key":K,"replaced":R,"evicted":E}`.

The fields and order are exact. Events carry results and evicted payloads, not a
copy of the whole state after every operation. The final state appears once.

A request is one Json object with exactly one unambiguous `capacity` strict-i64
integer member and one unambiguous `operations` array member. Extra members and
unrelated duplicate names are ignored after whole-document syntax/Unicode
validation. Invalid syntax, empty input, bad root, missing/ambiguous selected
member, incorrect selected type or an out-of-range capacity returns REQUEST.
This wrapper validation precedes calling run: e.g. negative capacity plus a
nonarray operations member is REQUEST. A valid wrapper with negative capacity
returns CAPACITY, even if an operation has bad shape. Otherwise invalid operation
shape returns OPERATIONS. No broad recovery absorbs fatal errors.

Success is exactly `{"ok":true,"state":S,"events":E,"error":""}`.
Failure is exactly `{"ok":false,"state":null,"events":[],"error":CATEGORY}`,
where CATEGORY is REQUEST, CAPACITY or OPERATIONS. These are response strings,
not new runtime error categories. The request adapter checks negative capacity
explicitly, then catches VALUE only around the shared operation-shape validator
to produce OPERATIONS. Cache transitions and result construction run outside
that catch. A coherent validated-execution helper may avoid repeated validation.
Unexpected internal VALUE or other data failure must propagate rather than being
disguised as a valid response; run constructs and maintains its own valid state.

This direct-error taxonomy corrects the original design's unsupported custom
CAPACITY/CACHE/OPERATIONS raises. Minimal source probes demonstrated that the
existing compiler accepts only its fixed catchable categories. The correction
preserves every request/result byte, all success semantics and required sizes;
the independent direct-API model is amended and reviewed before implementation.
No compiler/category-mask extension is part of this milestone.

## Distinguishing example

Capacity 2, with these operations:

```json
[
  {"op":"put","key":"a","value":1},
  {"op":"put","key":"b","value":null},
  {"op":"get","key":"a"},
  {"op":"put","key":"c","value":3},
  {"op":"get","key":"b"},
  {"op":"put","key":"a","value":4}
]
```

Final state is `{"capacity":2,"entries":{"c":3,"a":4}}`.
The first two puts have replaced false and evicted null. The first get is a hit
with value 1, making b the LRU. The c insertion evicts `{"key":"b","value":null}`.
The b lookup misses and leaves recency unchanged. The last put replaces a,
promotes it and evicts nothing. This distinguishes LRU from FIFO, hit from miss,
cached null from absence and replacement from insertion.

## Application and transport

`examples/lru-cache` and `bench/R024-lru-cache/program.ail` import the same library.
Their Analyze action is identical apart from package import paths: consume
`data.request: Text`, call runRequest, format its result as compact Json Text,
encode UTF-8 Bytes and select only `out.result: Bytes`, with trace `Simulated`.
Complete `out.result=hex:<lowercasehex>` remains one ASCII state line under the
unchanged benchmark observer. This retains payload U+2028/U+2029 and fake report
text without modifying the old observer. Public library results remain Json.

The standalone app additionally has `Run`: with a stdio grant, read bounded UTF-8
stdin through EOF as one request document, call runRequest, format the result,
append one LF and write stdout. Use --stdio to suppress ordinary state reports.
Both success and structured domain-error responses exit zero with exactly one
compact result plus LF and empty stderr. Empty/malformed JSON is a REQUEST
response. Two documents are invalid, not two jobs. No JSON-lines protocol, file
argument, service, TTL, resize, deletion operation or persistent storage is added.

Missing stdio grant, invalid UTF-8 and fatal Text/Json/list/depth/frame/fuel bounds
are nonzero failures, with no result stdout and no private-input diagnostic leak.
All parsing, validation, simulation, serialization and final LF join precede the
only write. Arbitrary OS write failure is not promised transactional rollback.
The tool is bounded to one input/result in memory; it is not a long-lived cache
server. Pure state APIs provide reuse within AIL programs.

## Required useful sizes and limits

Freeze independent success cases before implementation for all of these:

- Capacity 128 with 256 distinct scalar-valued puts, then gets of an evicted key,
  a surviving LRU key and the current MRU. Assert every eviction and final order.
- Capacity 16 with 512 accesses that mix hits, misses and existing-key replacements
  after filling it; exact model events and final recency must agree.
- Separate 8,192-scalar ASCII and U+754C keys and separate values, with put, get,
  replacement and an eventual eviction as appropriate. Keep each argv request
  below the existing OS argument bound by using literal UTF-8 for long scalars.
- A 5,000-digit integer payload, huge exponent token and nested duplicate-pair
  payload, exercised by put/get/eviction without Int or Float conversion.
- A payload containing 32 nested array containers, used in put/get/eviction.
- Signed-i64 maximum capacity with a few entries, zero capacity, empty operations,
  arbitrary control/pointer-like keys and Unicode separator/fake-state payloads.

These are minimum demonstrated cases, not new caps or throughput promises.
Existing 1 MiB raw/encoded Text/Json bounds, 65,536 Json values, 64 open containers,
64 value frames, cumulative 64 MiB Text and list/Json arenas and ordinary fuel
remain. Wrapping payloads consumes structure depth; repeated immutable copies
and event history consume cumulative allocation. Input bytes alone do not predict
acceptance. Document observed bounds; do not claim O(1), linear time, reclamation,
unbounded histories, arbitrary precision arithmetic or generic Map support.

Exercise fatal structural and representative history/allocation/fuel failures
with actual diagnostics, not guessed categories. An exactly 1 MiB stdin document
containing a small valid request plus permitted JSON whitespace must succeed;
1 MiB+1 fails the input Text bound. No worker weakens the frozen success sizes
to obtain a pass; an infeasible requirement returns to root for a written ruling.

## Independent verification and delivery

Before AIL source, write and independently review a Python OrderedDict model of
recency and eviction. Retain ordered-pair Json objects and opaque numeric tokens
for payloads, with full Unicode validation and an independent canonical encoder.
Do not use AIL results for expectations. State API and batch/request adapters are
all independently modeled. Cross-check hand traces and seeded properties against
a second simple list-of-pairs recency model; preservation and immutability matter.
Freeze intent, reference and named .args cases, then demonstrate the actual
unchanged benchmark evaluator fails because program.ail is absent. Preserve
actual pre-source direct-library RED probes and all failed harness attempts.

Acceptance compares complete independent expected bytes, ordinary reports and
raw stdin through source, saved Core and rendered Spec, each directly and through
STE on VM/native, plus separately executed and native-compiled saved bytecode:
14 application routes. A direct-call probe exposes all five public APIs across
the same 14 routes, checks sequential state reuse and unchanged retained inputs,
and catches VALUE at its direct-call boundary to report domain failures for comparison.
Do not select private inputs. Probe state bytes also use UTF-8 Bytes transport.

Frontload exact ordinary reports, each domain category, representative fatal
categories and mutations before expensive matrices. In an isolated copied library,
change a real recency/eviction rule; valid successful wrong output must be rejected
by independent expectations on VM and native. A report or compile failure is not
semantic mutation detection. Inspect actual semantic IR for pure functions and
permitted app-only I/O. Pin exact oracle membership, source dependencies including
runtime/build.rs, compiler and artifacts before execution and recheck afterward.
Every subprocess has a finite deadline and child cleanup.

Deliver coherent library/app guides, stdin sample and expected result, checked-in
STE export, `scripts/ail_lru_differential.py`, `docs/ail/64-lru-cache.md`, narrow
navigation and support-only registration. All seven required library guide
sections are meaningful. Expect 33 suffix packages; retain all 128 historical
catalog/story/request/response entries. Run the four affected package/docs/STE
tests, actual support-loader positive/negative/restored closure, local links,
documented sample commands and the unchanged full 23-rung benchmark. Existing
22 fingerprints/pass entries remain. Record 23/23 only after task and final
whole-milestone reviews and fresh dependency checks, with the unchanged lock writer
and no --bless-ladder. Save reviewable evidence; keep the broader goal active.
