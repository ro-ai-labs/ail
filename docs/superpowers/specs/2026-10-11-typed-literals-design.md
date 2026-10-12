# Typed literal assignments

The active goal remains an agent-usable Python replacement with executable STE
between specifications and binaries. Runtime inputs currently carry arbitrary
text/lists, while legacy prose assignments retain only a first token. Programs
need reliable source constants to construct their own data. This increment adds
an explicit typed literal statement across source, Core, STE, VM and native ELF.

## Source and semantics

```text
- the system assigns "Hello, world!\n🙂" to output.message
- the system assigns -9223372036854775808 to data.minimum
- the system assigns true to output.ready
- the system assigns [1, -2, 3] to data.numbers
- the system assigns ["é", "\ud83d\ude42"] to data.names
- the system assigns [] to data.names
```

The destination's declared type is authoritative: `Int`, `Bool`, `Text`,
`List<Int>` or `List<Text>`. Values are strict JSON literals of that type;
integers are checked i64 JSON integers (no plus, leading zeros, fraction or
exponent). Text uses JSON string escaping including paired surrogates. Lists
reuse the bounded flat-list parser. Reject null, objects, nested/mixed lists,
malformed strings, trailing content and mismatched types before execution.
Only ASCII JSON whitespace is allowed around values. No implicit coercion.

Assignments replace their destination at that exact action step. They may occur
inside branches and loops or imported/called actions. They read no runtime field
and previous copies retain their values. Empty text and lists are valid. Source
rendering uses canonical JSON literals, preserving Unicode, NUL, quotes,
newlines, punctuation and text containing source keywords. The statement uses
the last ` to ` delimiter, followed by a validated global field key.

Limits: at most 1,048,576 decoded UTF-8 bytes per text constant; at most 65,536
list items and 1,048,576 bytes in both input and canonical list JSON. Bound a raw
literal token to 6,291,458 bytes (six bytes per decoded control plus quotes).
Constants live in program data, so assignments do not consume runtime text/list
result budgets; later list/text operations retain their existing charges.
Repeated assignment of the same static value must not exhaust allocation
budgets. Source/Core/STE validation errors use `AIL-LITERAL-001` (the bytecode
verifier also adds its existing diagnostic wrapper).

Legacy prose assignment behavior is preserved. This explicit form is the path
for arbitrary literals; it does not silently reinterpret existing prose.
Application and C interop actions support it; integer functions stay integer-only
with their existing expression syntax. Other profiles and compiler-pass actions
must reject literal assignments instead of omitting them.

## Representation and interfaces

New `src/literal_assignment.rs` owns parsing, checked Core and value decoding.
Public AST: `AilLiteralAssignment::Assign { key: String, literal: String }` and
`Invalid(String)`. `AilAction.literal_assignments` plus
`AilActionStep::Literal(index)` preserve statement order. `source()` returns the
complete source statement. Namespace only the destination, never literal text.

Core `LiteralAssignment` nodes have exactly `value_type` and `literal`
attributes. `literal_step` connects exactly one executable Action owner with a
contiguous `step_index`; `literal_destination` connects exactly one global Field
whose declared type equals value_type. Other operand edges/attributes, missing
or duplicate edges, function-local destinations and invalid values are errors.
Provenance edges remain allowed. Derive secret writes from literal_destination,
even if descriptive writes are absent. Preserve duplicate saved-Core edges so
validation can reject them. Canonical source elaboration normalizes the literal;
saved Core may use any valid equivalent JSON, decoded/rendered canonically.

Bytecode `ASSIGN_LITERAL` has exactly `key`, `type`, `value` operands; `value`
holds the JSON literal (including quotes for Text), not decoded transport.
The STE template is `Assign literal {value} of type {type} to field {key}.`
Verification validates key syntax, supported type and the entire literal.
VM stores decoded text, canonical integer/bool, or canonical list transport.
Native compile validates/decodes once and embeds immutable bytes; execution
assigns pointer/length just as other static program data. Native dispatch must
use the scalar backend even for a program containing only literal assignments.

Module APIs, all pub(super) unless public AST:
- `parse_bullet(&str) -> Option<Result<AilLiteralAssignment, String>>`
- `elaborate(&mut Graph, &AilDocument)`
- `from_core(&AilCore, &Node) -> Vec<AilLiteralAssignment>`
- `check(&AilCore) -> Vec<AilDiagnostic>`
- `namespace(&AilLiteralAssignment, &str) -> AilLiteralAssignment`
- `instruction(&AilLiteralAssignment, &AilDocument) -> Result<AilBytecodeInstruction, String>`
- `verify(&AilBytecodeInstruction) -> Result<(), String>`
- `runtime_value(&AilBytecodeInstruction) -> Result<String, String>`
- `execute(&AilBytecodeInstruction, &mut BTreeMap<String, String>) -> Result<(), String>`
- `accesses(&AilCore, &str) -> Vec<(&Edge, &str)>` (only writes).

Expose one list-module wrapper for parse+canonicalize; do not duplicate JSON
list parsing or call the generic recursive JSON value parser for literals.

## Verification

Start with a failing source program creating its own Unicode/text/list/bool/Int
constants. Check exact observable values through source, saved Core, rendered
source, STE and real ELF. Compose with list/text operations and branch/loop/function
execution. Probe string keyword delimiters, escaped NUL and surrogate pairs,
immutable copies, imported destination names and constant text resembling fields.
Reject invalid syntax/type/profile/ownership/duplicate edges and secret writes
without descriptive edges. Check exact byte/item boundaries and repeated constants
without allocation charges. Keep frozen numeric cases/oracles unchanged, run
full Rust suite, Clippy, both differential scripts and independent literal probes.
