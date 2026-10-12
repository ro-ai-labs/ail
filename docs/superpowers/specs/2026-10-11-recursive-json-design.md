# Recursive JSON values and typed lookup

Continue the active STE-to-binary and agent-usable Python-replacement goal.
Flat typed arrays cannot represent ordinary API payloads or records. Add a
`Json` runtime type with nested values and useful read operations through every
existing execution route. This is an architectural extension, not a claim that
the remaining language and ecosystem requirements are complete.

## Values

- `Json` accepts every RFC 8259 value kind: object, heterogeneous/nested array,
  string, number, Boolean and null. Validate full input and UTF-8; accept only
  JSON whitespace and valid paired surrogate escapes.
- Keep number tokens exactly, including large integers, fractions, exponent
  spelling and negative zero. This is lossless data transport, not additional
  arithmetic support. Extracting `Int` still requires a strict i64 JSON integer.
- Compact encoding removes whitespace and uses existing AIL string escaping.
  Preserve member order and duplicate members. Pointer lookup of a duplicated
  member fails as ambiguous, including keys equal after escape decoding.
- Maximum raw/canonical bytes: 1,048,576 each. Maximum simultaneously open
  containers: 64. Maximum total values including root: 65,536; member names do
  not count as values. Parsing validates the whole bounded document before
  checking canonical expansion; structural depth/value limits may fail during
  validation. Input and canonical bounds also apply before lookup/count.
- Extend literal assignment and JSON parse/format to `Json`. Literal constants
  consume no result budget. Parse Json and get Json charge canonical bytes to
  the existing shared 64 MiB list arena. Format Json charges the text arena.
  Scratch is bounded and reclaimed; no hidden intermediate result charges.

## Operations

Source:

```
the system gets JSON value at path data.path from data.document into output.value
the system counts entries in JSON data.document into output.count
```

Get source is Json; path is a Text field containing the RFC 6901 string form
(not a URI fragment). Empty path selects root; slash separates tokens; ~0/~1
decode once to tilde/slash. Arrays require canonical unsigned decimal indices
(0 or nonzero digit followed by digits); negative indices, leading zeroes and
the nonexistent '-' element fail. Object keys use exact Unicode equality with
no normalization, including empty and NUL keys. Missing, ambiguous and scalar
traversals fail. Destination declaration selects Int, Bool, Text, List<Int>,
List<Text> or Json, with the same decoding rules as JSON_PARSE.

Count source is Json array or object, destination Int. It counts immediate
array items or object member pairs, including duplicate members; scalars fail.
Both operations validate source completely before path validation/conversion.
Lookup validates the entire path syntax before traversal. A path is bounded to
1 MiB and must be valid UTF-8. Results are immutable and destination aliases
are safe. Count and scalar Int/Bool lookup consume no result budget; Text and
list lookup charge only the final typed result to its existing arena.

## Graph and protocol

- `JsonAccess` has exactly `json_access_kind=get|count` and `value_type`, no node
  type. Get uses destination type; count requires Int.
- Exactly one executable Action owner via `json_access_step` with required
  numeric step_index and optional numeric operation_index. Exactly one
  `json_access_source` (global Json) and `json_access_destination` (typed global).
  Get additionally has one `json_access_path` (global Text); count forbids it.
  Operand edges have no attributes; only provenance may be added.
- `JSON_GET` operands exactly key,path,source,type; `JSON_COUNT` exactly key,source.
- STE: `Get {type} value at JSON path {path} from field {source} into field {key}.`
  and `Count entries in JSON field {source} into field {key}.`
- Only Application and C interop in Core and direct bytecode/STE. Functions and
  compiler-pass bodies reject the operations. Derive secret reads/writes from
  actual operands; imports namespace fields only. Preserve execution order and
  all source/Core/STE round trips.
- Compile diagnostics AIL-JSON-001; bytecode wrapper AILBC016. Runtime malformed
  source/type/UTF-8 uses AIL-RUNTIME-JSON; structural/size/allocation limits use
  AIL-RUNTIME-JSON-LIMIT; malformed or unresolved pointer uses
  AIL-RUNTIME-JSON-PATH. Oversized path uses JSON-LIMIT. Native allocation
  failures use JSON-LIMIT; VM host-OOM recoverability remains unpromised.
- Traces: `got JSON <key>` and `counted JSON <key>`, without operand values.
  Failed operations leave destination and semantic allocation counters unchanged.

## Implementation and verification

Use focused Rust recursive parsing/runtime helpers and directly generated native
helpers; no external runtime, subprocess parser or new package dependency.
Keep old scalar/list JSON behavior and diagnostics intact. Compare independently
with Python JSON using number-token and object-pair hooks, plus RFC pointer cases.
Test malformed grammar/Unicode, duplicate keys, depth/node/byte boundaries,
alias lifetime, budgets, exact Core metadata, imports and derived secret access.
Verify a file of nested records: iterate records via dynamic pointer, extract
integer/text fields, aggregate integers and emit a report on VM and native ELF,
including source, saved Core and direct STE paths.

Object mutation, richer function values, floating-point arithmetic, arbitrary
precision computation and wider Python compatibility remain open parts of the
same active objective.
