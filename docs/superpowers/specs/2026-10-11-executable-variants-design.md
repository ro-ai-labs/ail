# Executable recursive variant values

## Intent and current evidence

The user wants Specs -> ASD-STE100 -> binary, followed by a language that can
replace Python and be used by an AI agent. The existing executable STE profile
and built-in computations are working. The previous explicit-error/RLE milestone
is verified at 18/18; all 313 final recorded inputs still match. This is an
architectural addition of executable user-defined data, a remaining generality
gap identified by the capability ladder. It does not redefine the full goal.

The next deliverable is recursive variant construction, inspection and extraction,
with exact nominal type checking inside programs and typed function boundaries.
A pure AIL binary search tree proves insertion and in-order traversal (R019).
Exhaustive pattern matching, generic functions, first-class functions, unrestricted
nested built-in lists and arbitrary-precision arithmetic remain subsequent work.

## Source contract

Use existing declarations, including finite concrete instantiation of generics:

```text
Type: Tree<T>.
Tree has variants:
- Empty
- Node(left: Tree<T>, right: Tree<T>, value: T)

- the system constructs Tree<Int> variant Empty with none into data.empty
- the system constructs Tree<Int> variant Node with left = data.empty; right = data.empty; value = 7 into data.tree
- the system tests Tree<Int> variant Node of data.tree into data.is_node
- the system gets field value of Tree<Int> variant Node from data.tree into data.value
```

Functions use `the function` and declared `local.*` fields. Constructor arguments
are named, complete, unique and checked against the selected variant. Int payloads
accept checked integer expressions; other payloads require bare fields of the
exact type. Tests require a Bool destination. Gets require the exact payload
destination type. Unknown variants, fields, missing/extra arguments and nominal
type mismatches reject statically, including unreachable code.

Operations work in Application and C interop execution profiles. Imports qualify
user type heads, nested arguments, payload types, function locals and operation
types while preserving local field names and bound generic parameters. Generic
type parsing must split commas at the correct nesting depth and reject duplicate
declarations, variants and fields. Existing descriptive declarations and functions
keep their previous behavior outside executable roots.

## Closed runtime schemas

Add optional `type_schemas` program metadata, a sorted map from concrete type name
to variant-name map to payload-name/type map. Canonical empty metadata is omitted,
preserving existing artifact output. Example:

```json
{"Tree<Int>":{"Empty":{},"Node":{"left":"Tree<Int>","right":"Tree<Int>","value":"Int"}}}
```

Roots are custom types used by executable function signatures/locals and explicit
variant operations. Resolve every variant and payload transitively with structural
substitution, a worklist and a visited set. Reject malformed names/arity, free type
variables in concrete schemas, unknown heads, ambiguous declarations and unbounded
instantiation. Recursive types need not be productive. All current executable
builtins can be leaves: Int, Float, Bool, Text, Bytes, List<Int>, List<Float>,
List<Text>, Json. These nine executable builtin spellings keep precedence over
descriptive declarations; qualified user types can provide distinct collection
families. Other descriptive heads are not newly reserved by this feature.

Bounds: type-expression depth 32 and concrete type spelling at most 4,096 bytes;
at most 4,096 concrete custom types, 65,536 total variants and 65,536 total payload
fields; at most 256 variants per type and 256 fields per variant; each identifier
segment at most 128 ASCII identifier characters; encoded schema at most 4 MiB. Schema and operation limits
are static diagnostics, not catchable runtime errors.

Core retains existing Type/Variant/Field declarations. Validate the entire
reachable declaration subgraph, ownership, identities, attributes, provenance and
operand edges before reconstructing maps. Constructor arguments must be explicit
named, typed, owned operand edges, never unchecked packed executable metadata.
Action operations and function-local operations participate in source ordering,
read/write/secret analysis and exact round trips.

Bytecode operations have exactly these operands:

- `VALUE_CONSTRUCT {type, variant, arguments, key}`
- `VALUE_TEST {type, variant, source, key}`
- `VALUE_GET {type, variant, field, source, key}`

Arguments use the existing sorted named postfix encoding. Whole-program checking
validates all schemas, function signatures and operation types before execution.
STE declares every operand separately and uses fixed templates:

```text
Construct type_data variant variant_data with arguments arguments_data into field key_data.
Test type_data variant variant_data of field source_data into field key_data.
Get field field_data of type_data variant variant_data from field source_data into field key_data.
```

Source/Core/schema diagnostics use `AIL-VALUE-001`; protocol verification uses
`AILBC024` (verified unused in the current diagnostics registry).
Existing function diagnostic families remain applicable to signature/call errors.

## Values and runtime behavior

Transport is a JSON array containing a variant name followed by payloads in sorted
payload-name order. Custom payloads are nested arrays; builtin payloads are JSON
strings containing their ordinary canonical runtime transport. For example:
`["Node",["Empty"],["Empty"],"7"]`. Text is quoted directly; Float retains
nonfinite values and signed zero; Bytes retains lowercase hex; Json retains number
spellings, member order and duplicate keys under its existing rules.

Input permits ordinary JSON whitespace and string escapes. Fully validate and
canonicalize every payload, including unselected branches. Never accept a
malformed tree merely because a test would return false. Field order and types
come from the schema. Equal-shaped nominal types may share external transport;
untyped input is validated against the expected type, while checked programs
cannot mix nominal types.

Bounds: 8 MiB raw and produced canonical transport; custom nesting depth 64 (root depth 1);
65,536 total custom values plus builtin payload values. Tags and JSON quoting do
not count as values. Builtin contents retain their existing internal size/item/
depth rules independently. Escaping can make an otherwise valid builtin too large
to embed in a custom value. Borrowed validation and tests permit canonical
expansion beyond the output bound so a program can inspect or shrink such input;
producing a result enforces its canonical bound and each embedded builtin's
canonical transport bound. No codec/tree-specific smaller bound is allowed.

New catchable `VALUE` (append index 14; `any` mask 32767) covers missing/malformed
custom data, invalid builtin payloads during construction, and wrong variant on
get. `VALUE-LIMIT` is fatal for custom transport/depth/count/allocation limits.
Embedded builtin size/item/depth violations also map to VALUE-LIMIT; invalid
builtin grammar or type maps to VALUE. Standalone builtin operations keep their
existing error families.
Engine-detected canonical bounds, including a selected builtin whose canonical
transport is too large to extract, use VALUE-LIMIT. Only subsequent result-arena
reservation uses that extracted builtin's existing arena error family.
Existing category indices stay fixed. Int expression failures during argument
evaluation retain their existing categories. Limits on extracted builtin result
arenas retain those builtins' existing fatal categories. Diagnostics omit data.

Validate constructor arguments in sorted field-name order. Validate custom
function arguments in sorted parameter-name order, even unused ones, before
checking call depth. Borrow valid arguments without semantic allocation. One
successful constructor/custom return owns one canonical copy charged to the
existing 64 MiB shared list/Json/Bytes arena. Test-to-Bool does not charge an arena.
Get owns a copy under the selected type's normal return allocation rules; custom
gets charge the shared arena. Failed operations/calls preserve their destination;
earlier effects, allocations and traces remain. Aliasing source/destination is
allowed when types permit it. Existing call depth, fuel and temporary cleanup
rules continue to apply.

## VM and native implementation

Use a shared allocation-free, no_std transport engine with strict bounded string
decoding, leaf validation/canonicalization and schema-driven recursive traversal.
Reuse the existing Float and Bytes engines. Validate compatibility of new leaf
handling against existing Int/Text/List/Json behavior; avoid unrelated rewrites.
Measurement and writing passes must complete before result publication. Scratch
storage is bounded and does not charge semantic arenas.

Embed the engine into the existing statically linked Rust runtime at Cargo build
time. Native AIL compilation emits calls to that embedded engine and static schema
tables; it invokes no compiler, subprocess or host codec. Both native entry paths,
including function-free actions, must retain schema context. Engine status returns
reach the generated adapter before recovery jumps; do not unwind through Rust
callbacks. Temporary mappings are registered with existing recovery cleanup.

The exact shared ABI and schema encoding are recorded in the implementation plan
before the compiler and native owners start their dependent implementation.

## Independent evidence

Freeze R019 intent, Python reference and cases before its AIL implementation.
Contract: input `data.values: List<Int>` and `data.query: Int`; insert every value
into a binary search tree, retaining duplicates by node multiplicity; return
`out.sorted: List<Int>` (ascending, duplicates retained), `out.present: Bool`
(whether query occurs), and `out.height: Int` (empty=0, root-only=1). The reference
uses an independent iterative Python tree. The AIL program must use declared
recursive variants and ordinary functions, with no sorting opcode or host tree.

Cover empty/single/duplicates, ascending/descending and balanced inputs, negative
and i64 extrema, absent/present queries and deterministic generated shapes. The
differential suite additionally covers every builtin leaf, multi-parameter and
mutually recursive generics, import qualification, same-shaped nominal rejection,
malformed unused payloads, wrong variants, bounds, allocation recovery/atomicity,
source/Core/rendered/STE/VM/native routes, and mutations that corrupt tree values
or bypass typed validation. Keep existing rung/oracle inputs frozen.

All implementation pieces receive independent review. Final gates include fmt,
diff checks, strict Clippy, all Rust tests, every prior differential suite, the new
suite, full benchmark and broad comparison. Promote only the new R019 rung after
actual independent VM/native success. Keep the full user goal active afterward.
