# JSON record mutation

Continue the active STE-to-binary and agent-usable Python replacement goal.
Recursive Json currently supports reading nested records but cannot construct
or update structured reports from computed values. Add immutable set/remove
operations through source, checked Core, STE, VM and native ELF.

## Source and semantics

```
the system sets JSON value at path data.path in data.document to data.value into output.document
the system removes JSON value at path data.path from data.document into output.document
```

Source and destination are global Json fields; path is global Text. Set's value
is a global field of type Int, Bool, Text, List<Int>, List<Text> or Json. Its type
comes from the declaration. Convert the value with existing JSON_FORMAT rules,
including Int transport signs/leading zeroes and lossless Json number tokens.

- Paths use the existing RFC 6901 string form, with exact Unicode and decoded
  ~0/~1 escapes; validate the entire path syntax before traversal.
- Set with empty path replaces the root. Remove with empty path fails JSON-PATH.
- Parents must already exist and be containers; no implicit parent creation.
- On objects, set replaces one uniquely named member in place, or appends a new
  member at the end if absent. Remove requires one uniquely named member and
  deletes that pair. Selected duplicate names fail as ambiguous, including
  duplicates that become equal after escape decoding. Other pairs retain order.
- On arrays, canonical unsigned decimal indices must name an existing item.
  Set replaces that item; remove deletes it and shifts following items left.
  Set with final token `-` appends. Remove '-' and numeric index equal to length
  fail. Intermediate '-' has ordinary failed-lookup semantics. For an object,
  '-' is an ordinary key.
- Preserve all unmodified values, duplicate pairs and number spellings. In-place
  output and aliases of source/path/value are safe. Copies remain immutable.

## Limits, ordering and allocation

Validate in this order: source presence/raw UTF-8/grammar/structural/canonical
bounds; path presence/UTF-8/size/full syntax; parent traversal and final mutation
target validity; set value presence/transport/type/encoding validation; resulting
Json structural/encoded bounds; result allocation. Do not let a malformed value
mask an invalid path. Source bounds apply even if removal/replacement could
shrink the result. Empty-root set still validates the source.

Existing recursive limits apply independently to source, set Json value and
result: 1,048,576 raw/canonical bytes, 64 simultaneously open containers and
65,536 total values including root (member names do not count). Every value
operand/path is bounded to 1 MiB raw bytes; typed list value rules remain intact.
The combined result must meet depth/value limits after attachment/replacement;
replacing a subtree does not count the removed subtree toward the result.

Only final encoded Json bytes charge the existing shared 64 MiB list arena.
No text/list budget charge for formatting/parsing intermediate values. Scratch
is bounded and reclaimed. Failed operations leave destination and semantic
allocation counters unchanged. JSON errors suppress operands. Native failures
exit nonzero without result stdout. VM host-OOM recoverability remains unpromised.

Error classes remain AIL-RUNTIME-JSON for malformed/missing runtime data/type,
AIL-RUNTIME-JSON-PATH for malformed/unresolved/ambiguous mutation paths and
AIL-RUNTIME-JSON-LIMIT for size/depth/value/allocation limits. Successful traces
are `set JSON <key>` and `removed JSON <key>`.

## Core and protocol

- `JsonMutation` has no node type. Set attrs exactly json_mutation_kind=set and
  value_type; remove attrs exactly json_mutation_kind=remove.
- Exactly one executable Action owner via `json_mutation_step`, with required
  numeric step_index and optional numeric operation_index. Exactly one each
  of `json_mutation_source` (Json), `json_mutation_destination` (Json), and
  `json_mutation_path` (Text); set additionally has one `json_mutation_value`
  matching value_type. Operand edges have no attrs; only provenance may be added.
- `JSON_SET` operands exactly key,path,source,type,value. `JSON_REMOVE` operands
  exactly key,path,source. STE templates:
  `Set {type} value {value} at JSON path {path} in field {source} into field {key}.`
  `Remove value at JSON path {path} from field {source} into field {key}.`
- Application/C interop only, enforced in Core and direct bytecode/STE. Both
  legacy and executable function bodies, and compiler passes, reject these
  statements. Compile diagnostics AIL-JSON-001, bytecode wrapper AILBC016.
- Preserve source execution order, imports and source/Core/STE round trips.
  Derive secret reads from source/path/value and writes from destination, even
  without descriptive access edges. Imports qualify only field references.

## Implementation and evidence

Reuse recursive parsers/encoders and direct native helpers. No external runtime,
subprocess parser, new dependency or budget-reset trick. Keep typed conversion,
recursive access, text/list/file and function semantics intact. Use independent
Python number-token/object-pair mutation oracles, including replacement/removal
order, root replacement, append, escaped/NUL keys, duplicate ambiguity, all value
types, error priority, aliases, exact limits and shared budgets. Check a complete
file-transform example that removes a field, computes new values, builds a Json
report using set/append and writes it through all six source/Core/STE × VM/native
routes. Run the full suite and unchanged benchmark/corpus.

General function values, more numeric computation and broader Python ecosystem
compatibility remain requirements of the same active goal.
