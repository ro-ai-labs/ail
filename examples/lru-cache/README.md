# LRU cache request tool

The [pure LRU library](../ail_std_lru.ail/README.md) supplies all cache behavior.
Analyze consumes `data.request: Text` and selects only `out.result: Bytes`, with
trace Simulated. Decode the lowercase hex bytes as UTF-8 to obtain compact Json;
this preserves Unicode separators and payloads resembling ordinary report lines.
Run reads one UTF-8 request through EOF and writes one compact result plus LF,
with trace Completed. `--stdio` grants streams and suppresses ordinary reports.

From the repository root, with the existing compiler built:

```bash
target/debug/ail ail-run examples/lru-cache --action Run --stdio < examples/lru-cache/input.json
target/debug/ail ail-vm examples/ste/lru-cache.ail-ste --action Run --stdio < examples/lru-cache/input.json
target/debug/ail ail-compile examples/lru-cache --action Run --target linux-x86_64-elf --out /tmp/lru-cache
/tmp/lru-cache --stdio < examples/lru-cache/input.json
target/debug/ail ail-run examples/lru-cache --action Analyze 'data.request={"capacity":1,"operations":[{"op":"get","key":"missing"}]}'
python3 scripts/ail_lru_differential.py --artifact-dir /tmp/lru-acceptance
```

The acceptance driver uses a 7200-second supervisor deadline with descendant
cleanup and 90-second leaf deadlines. The artifact directory must be initially
empty. The driver freezes the oracle, compiler and semantic dependencies, snapshots
sources, saves full input/output bytes and tests fourteen app and fourteen direct
API routes. [input.json](input.json) and [expected.json](expected.json) demonstrate
promotion, replacement without unrelated eviction, eviction of cached null and
preservation of duplicate payload members and number tokens. The three Run executions
produce the exact expected file; Analyze uses the Bytes report transport.

The request is one object with unique decoded `capacity` and `operations` members.
Capacity is a strict signed-i64 integer; operations is an array. Get/put require
unique string op/key; put also requires one value. Get ignores value, and unrelated
fields/duplicates are ignored after whole-document syntax/Unicode validation.
Wrapper failures return REQUEST, negative capacity after valid wrapper checks
returns CAPACITY, then invalid operation shapes return OPERATIONS. Validate every
operation before any transition. All three structured errors exit zero with exact
compact Json+LF and empty stderr. Empty input or two documents returns REQUEST.
Invalid UTF-8, missing stdio grant and fatal runtime bounds exit nonzero with no
result stdout. Every fallible construction step precedes the only stdout write.

State persistence uses the returned value: retain the state from get/put for the
next call. A new run/runRequest starts empty. Entries are LRU to MRU; hits and
replacements promote, misses preserve order, and only full absent-key insertion
evicts the prior LRU. Capacity zero stores nothing; cached null has hit true.
Exact Unicode keys distinguish normalization variants. Outer state normalizes
capacity then entries and -0 to 0. Payload order, duplicate pairs, number lexemes
and decoded strings survive immutable copies; escape spelling/whitespace normalize.

Direct domain rejection is VALUE; malformed typed transports and fatal errors
remain existing runtime errors. The [reference](../../docs/ail/64-lru-cache.md)
details APIs, exact shapes, event order, error priority and measured copying bounds.
The [source](spec.ail-spec.md) contains the complete app; the benchmark uses its
same Analyze body. This is a bounded in-memory request tool, with no persistent
storage or JSON-lines protocol.
