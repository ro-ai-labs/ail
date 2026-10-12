# Text transformations through checked Core and STE

The continuing user objective is an agent-usable Python replacement through
Specs → checked Core → ASD-STE100-based STE → executable VM/native artifacts.
The previous turn is verified progress: Boolean predicates and 1,026 tests,
independent reviews/oracles, and working file filters. The roadmap's R020 names
split/list-join as missing string operations; current Text only supports two-field
concatenation, length, slice and find. Add direct transformations for practical
line/column parsing, cleanup and reports, then exercise them in a real TSV report.
This is an architectural language extension; the complete original goal remains
active and no narrower readiness gate substitutes for full Python replacement.

## Chosen design

Extend the existing typed TextOperation rather than encode opaque library calls
or require every program to build split/replace loops by hand. All four operations
are visible to Core, imports, access analysis, STE and both runtimes. Reuse the
existing List<Text> codec, bounds and arenas at the Text/list boundary. Preserve
existing Text operations and function contracts. No dependency or interpreter.

### Source and fixed interfaces

All operands are declared fields. Actions use `the system`; executable functions
use `the function` and declared `local.*` fields. Examples:

```text
- the system splits text data.raw by data.separator into data.parts
- the system joins list data.parts with data.separator into data.raw
- the system replaces text data.needle in data.raw with data.replacement into data.raw
- the system trims text data.raw into data.raw
```

Add AilTextOperation variants:

```text
Split { key:String, source:String, separator:String }
JoinList { key:String, source:String, separator:String }
Replace { key:String, source:String, needle:String, replacement:String }
Trim { key:String, source:String }
```

Split destination is List<Text>; JoinList source is List<Text>; every other
operand/destination is Text. New bytecode uses exact operands:

| Opcode | Operands |
| --- | --- |
| TEXT_SPLIT | key,source,separator |
| TEXT_JOIN_LIST | key,source,separator |
| TEXT_REPLACE | key,source,needle,replacement |
| TEXT_TRIM | key,source |

`text_operations::execute(instruction,state,text_allocated,list_allocated)`
replaces its current three-argument signature, preserving old behavior. Reusable
codec helpers in list_operations are narrow: parse_text_items(&str) ->
Result<Vec<String>,String>, and make_text_list(&[String],&mut usize) ->
Result<String,String>. The latter validates/encodes first, then charges one
owned canonical list; failure preserves its counter. No change to Float modes.

### Operation semantics

- Split matches a nonempty complete separator left to right without overlap,
  retaining leading/trailing/consecutive empty parts; empty input gives `[""]`.
  Empty separator is an error, including for empty input.
- JoinList inserts the exact separator between decoded items, never at the ends;
  an empty list gives empty Text. Empty separators are allowed.
- Replace substitutes every nonoverlapping occurrence left to right. Replacement
  text is never searched again. Empty needle inserts replacement at every Unicode
  scalar boundary, including both ends; empty input has one boundary.
- Trim removes leading/trailing characters from this fixed whitespace set:
  U+0009–000D, U+001C–0020, U+0085, U+00A0, U+1680, U+2000–200A,
  U+2028–2029, U+202F, U+205F, U+3000. Internal characters remain unchanged.
  NUL, U+200B and U+FEFF are not whitespace. Pinning this 29-scalar set keeps
  backends deterministic independently of installed Unicode versions.

These choices follow the corresponding explicit-separator split, join,
all-occurrence replace and default strip behavior in the
[Python string reference](https://docs.python.org/3/library/stdtypes.html#string-methods).
The whitespace list includes the 25 entries in
[Unicode 17 White_Space](https://www.unicode.org/Public/17.0.0/ucd/PropList.txt)
plus U+001C–001F covered by Python's bidi whitespace definition.
All operations preserve NUL, Unicode spelling and combining sequences; they do
not normalize or interpret markup. Native split/replace must avoid quadratic
repetitive-needle search; use a bounded linear matcher and copy final output once.

### Validation, limits, errors and ownership

Validation order is observable and matches both backends:

1. Split: source Text, separator Text, nonempty separator, list result limits,
   then list arena.
2. JoinList: entire source List<Text> (including unselected/trailing malformed
   items), separator Text, Text result size, then Text arena.
3. Replace: source Text, needle Text, replacement Text, result size, then Text arena.
4. Trim: source Text, result size, then Text arena.

Text inputs/results remain at most 1,048,576 bytes of valid UTF-8. List inputs
remain at most 1,048,576 raw bytes and 65,536 items. Split output must also fit
1,048,576 canonical bytes and 65,536 items, including empty parts. JoinList accepts
valid borrowed lists whose canonical re-encoding would expand over the bound;
it need not produce a list. Existing strict JSON string decoding applies.

Each produced Text charges max(result bytes,1) once to the shared 67,108,864-byte
Text arena, including unchanged and empty results. Split charges canonical output
bytes once to the shared 67,108,864-byte list/Json arena and never charges Text
for individual items. Scratch parsing/matching has no result-arena charge.
Function return adds the existing one-copy charge for its returned Text/list.
No budget refund when replacing a destination. Read all inputs before mutation;
direct-protocol aliases must work and old results survive later operations.
Failures preserve destination and both counters.

Missing/malformed/oversized Text uses existing TEXT/TEXT-LIMIT runtime families;
empty split separator uses AIL-RUNTIME-TEXT. Malformed/missing JoinList source
uses LIST; raw/item bounds use LIST-LIMIT. Split result bounds/allocation use
LIST-LIMIT; JoinList/Replace/Trim output bounds/allocation use TEXT-LIMIT.
Do not put operand contents in diagnostics.

### Core, profiles, functions and STE

Reuse untyped TextOperation with exact text_kind=split|join_list|replace|trim.
Split/JoinList use text_source, text_other (separator), text_destination.
Replace uses text_source, text_other (needle), new text_replacement, and
text_destination. Trim uses source/destination. Every operand edge is unique,
attribute-free and points at a genuine uniquely declared global Field with the
correct type; operation attributes/type/owner/order remain checked. Apply the
same strictness to existing Text operations where the shared decoder is touched.
Local operations use existing ValueFunctionStep/typed value_field_operand schema,
with exact local ownership. All three Replace reads count for secret checks.

Application/C interop only. Legacy descriptive functions and compiler passes
reject all new recognized forms, including malformed forms and both subjects.
Preserve existing descriptive text behavior and unrelated identifiers.
Imports rename field keys only. Keep function classification/entry validation,
shared depth/fuel, result lifetimes and Float dependency discovery intact.

STE templates (technical operands stay separately declared):

```text
Split text in field {source} by field {separator} into field {key}.
Join text list in field {source} with field {separator} into field {key}.
Replace text from field {needle} in field {source} with field {replacement} into field {key}.
Trim whitespace from field {source} into field {key}.
```

Reuse static AIL-TEXT-001 and bytecode AILBC012 diagnostics. Action traces remain
`computed text <key>` (the existing Text operation trace); local computations
have no implicit trace. No new implicit field outputs.

## Verification and deliverables

Focused RED→GREEN tests before production hooks; independent Python expectations
for split/join/replace/strip, all Unicode whitespace, scalars and boundaries;
invalid native UTF-8, exact/over limits, pathological repetitive needles,
immutable aliases, error priority, arena charge and function return accounting.
Test source/saved Core/rendered source/STE through VM/native, namespaced imports,
secret operands, bad type/owner/metadata/edges/profiles and exact protocol.
Build examples/tabular-report plus exported STE using all four operations in
local functions and action loops, with Python-generated expected file reports.
Update docs/ail/49-text-transformations.md and reference/schema/agent-facing indices.
Do not change frozen benchmark inputs/answers/SCORE.lock. Run prior oracles,
full serial Rust suite, strict Clippy/fmt, frozen16/16 and independent reviews.
