# JSON inspection for general record traversal

The ongoing objective is an agent-usable Python replacement whose checked specs
pass through ASD-STE100-based controlled English to executable binaries. The
previous turn was verified progress: List<Float>, 957 tests, independent oracles
and reviews. Current source/runtime inspection supports known-path lookup and
collection counts, but not discovery of unknown object fields or value kinds.

## Design and alternatives

Extend the existing JsonAccess subsystem with kind inspection and object-key
enumeration. Together with existing counts, pointers, typed Json extraction,
loops and recursive value functions, these enable a generic JSON tree walker.
A separate dictionary representation would duplicate the lossless Json model
and still need these bridges; putting ad hoc parsing in each AIL program would
bypass checked JSON semantics. This extension preserves every valid recursive
Json value, including ordered duplicates and lossless numbers. Broader native
collections and libraries remain part of the full goal.

## Source, Core and STE contract

New source forms:

```text
- the system gets JSON kind from data.document into output.kind
- the system gets JSON keys from data.document into output.keys
```

The source must be Json. Kind destination must be Text; keys destination must be
List<Text>. Functions use the same forms with `the function` and declared local
fields. Application/C interop only; legacy descriptive function/compiler-pass
bodies reject both forms, including both subjects. Malformed recognized prefixes
are errors rather than silently ignored descriptive steps.

Extend AilJsonAccess with Kind {source,key} and Keys {source,key}. JsonAccess
nodes keep exactly json_access_kind and value_type attributes, with kind values
`kind`/`keys` and result types Text/List<Text>. Use the existing ordered
json_access_step and source/destination edges. No path operand is allowed.
Global operands must have unique genuine declared storage; exact metadata,
owned action/local operations, secret access and import qualification apply.

JSON_KIND and JSON_KEYS instructions each have exactly key,source operands.
Reuse AIL-JSON-001 / AILBC016 diagnostics and shared JSON profile verification.
STE templates:

```text
Get JSON kind from field {source} into field {key}.
Get JSON keys from field {source} into field {key}.
```

Trace strings are `got JSON kind {key}` and `got JSON keys {key}`. New operations
participate in normal ordering, function dependencies, rendering/imports, saved
Core, STE validation and native/VM emission.

## Runtime semantics

Parse and validate the entire source before inspecting the root or charging a
result. Existing 1,048,576-byte raw and canonical bounds, 65,536 values including
root and 64 simultaneous containers apply. Unselected malformed/oversized data
must fail even when only the root kind is requested. Exact number spelling and
numeric magnitude do not affect classification or key enumeration.

JSON_KIND returns exactly one lowercase Text word: null, boolean, number,
string, array or object. It produces one owned Text result, charging its byte
length once to the 67,108,864-byte text arena. JSON_KEYS accepts objects only
and returns a canonical List<Text> of decoded member names in encounter order.
It preserves duplicates, empty names, NUL, Unicode and distinct normalization
forms. It charges final encoded list bytes once to the shared 67,108,864-byte
list/Json arena. Empty object returns [] and costs 2 bytes. Arrays/scalars are
JSON errors. Ordinary get still rejects ambiguous duplicate-name lookup.

Whole-source syntax/structural/canonical limits precede root-type validation,
which precedes final result limits and allocation. Errors use JSON/JSON-LIMIT,
contain no runtime contents, and preserve destination and both counters. Source
and destination may alias in direct protocol; validation and result creation
must finish before replacing source storage. Temporary parser/encoder storage
is not a semantic result allocation. A List<Text> function return remains an
additional one-copy return charge under the existing function contract.

## Implementation

VM reuses the strict lossless parser and encoder in structured_json.rs.
Native reuses structured_parse and its root kind/member descriptors, traverses
member siblings for decoded keys, then writes one canonical list result. Keep
existing scratch bounds, register/field layout and allocation/error helpers.
Do not parse the document or allocate scratch once per key. Neither operation
requires Float numeric helpers merely because numbers occur in its source.
Functions keep isolated locals, sorted parameter validation, shared depth/fuel,
and supported entrypoint rules. No dependencies, external interpreters, runtime
files or compiler subprocesses are added.

## Verification and example

Require observed red tests before implementation and VM/native agreement across
source, saved/rendered Core, STE and functions. Cover all six kinds, huge
lossless numbers, duplicate/escaped keys, empty/NUL/Unicode keys, key order,
nonobject rejection, aliases, malformed protocol/Core/profile/secret operands,
imports, full-source validation, depth/value/raw/canonical bounds and exact
shared arena accounting. Include key count at the 65,535-member Json limit.

An independent Python oracle retains object pairs and raw number tokens. A
runnable file example recursively counts scalar leaves in arbitrary arrays and
objects, with an AIL helper escaping dynamic JSON Pointer keys using existing
text operations. It must handle previously unknown keys containing /, ~, NUL
and Unicode. Verify exact file output on source/Core/STE × VM/native. Duplicate
names retain existing ambiguous-path failure rather than implicit deduplication.
Run full serial Rust tests, strict lint/fmt, prior JSON/value/list/Float oracles
and the unchanged frozen benchmark. Independent frontend and native reviews
must close findings. Keep the full language-development goal active.
