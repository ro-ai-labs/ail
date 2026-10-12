# Executable recursive variants

User-defined variant types can hold builtin or recursive custom values in
Application and C interop programs. Actions and functions can construct variants,
test their tags and extract typed fields through Core, STE, VM and native ELF.

```text
Type: Maybe<T>.
Maybe has variants:
- None
- Some(value: T)

A Data has:
- value: Maybe<Int>
- present: Bool
- number: Int

Action: Inspect.
When Inspect happens:
- the system constructs Maybe<Int> variant Some with value = 42 into data.value
- the system tests Maybe<Int> variant Some of data.value into data.present
- if boolean data.present:
- the system gets field value of Maybe<Int> variant Some from data.value into data.number
- end if
- the system returns data.number
- the system records a trace event named Inspected
```

An empty constructor uses `with none`. Named arguments use semicolons and must
match every payload field exactly. Int arguments accept checked expressions;
other arguments require a field of the exact declared type. A test writes Bool;
an extraction writes the selected payload type. Unknown variants, payload fields
and mismatched types reject even in unreachable code.

Functions use the same statements with `the function` and declared `local.*`
values. Custom types can be parameters, locals and results. Imported type names
are qualified independently of function names; importing the tree library as
`Bst` gives `Bst.Tree<Int>` and functions such as `Bst.insert`.

## Recursive and generic data

```text
Type: Tree<T>.
Tree has variants:
- Empty
- Node(left: Tree<T>, right: Tree<T>, value: T)
```

Generic declarations instantiate into concrete types such as `Tree<Int>` or
`Tree<Text>`. Multiple type parameters and finite mutual recursion are supported.
Each reachable concrete type expands all its variants and fields. Unknown heads,
incorrect argument counts, free type variables and endlessly expanding type
instantiations reject. The nine executable builtin spellings below retain their
transports and take precedence over descriptive declarations with the same name.
Other descriptive type heads are not reserved by this feature.

The executable leaves are Int, Float, Bool, Text, Bytes, List<Int>, List<Float>,
List<Text> and Json. Descriptive Money, Pointer, Secret and arbitrary Thing records
do not automatically become executable payloads. Generic data declarations do not
introduce generic functions. Exhaustive pattern matching, first-class functions,
custom equality and unrestricted nested builtin lists remain separate work.

## Transport and validation

A custom value is a JSON array containing its variant name followed by its
payloads in sorted field-name order. A nested custom payload is another array;
a builtin payload is a JSON string containing its ordinary runtime transport.

| Type/value | Canonical transport |
| --- | --- |
| Maybe<Int>.None | `["None"]` |
| Maybe<Int>.Some(42) | `["Some","42"]` |
| Maybe<Text>.Some(hello) | `["Some","hello"]` |
| Maybe<Float>.Some(NaN) | `["Some","nan"]` |
| Maybe<Bytes>.Some(0xff) | `["Some","hex:ff"]` |
| Tree<Int>.Node with empty children and value 7 | `["Node",["Empty"],["Empty"],"7"]` |

This convention preserves nonfinite Float values, signed zero and binary Bytes.
Json payloads retain their number spellings, member order and duplicate keys.
Ordinary JSON whitespace and escapes are accepted on input and canonicalized on
output. Canonical quoting follows the existing text/JSON rules, including C1
control escapes. The schema determines payload order and types.

Nominal types remain distinct inside checked programs. Equal-shaped types can
share serialized bytes: external input is checked against the expected type.
Every operation validates the full value, including unused payloads. A test
reports false only for a different valid variant. Get on a different variant
raises VALUE. Constructed and returned data is immutable; updates build a new
value. Destination aliasing is allowed when types match and preserves the old
destination on failure.

## Limits and recovery

Custom input is at most **8 MiB**, with **64 custom nesting levels** (root=1)
and **65,536 typed values** across custom instances and builtin payloads. Tags do
not count as values. Builtin leaves retain their own size, item and depth bounds;
Json nesting is counted separately from custom nesting.

Produced canonical custom values are at most 8 MiB. Borrowed input validation and
tag tests can accept input whose canonical form would be larger, allowing the
program to inspect or shrink it. Producing output also checks each embedded
builtin's canonical bound. Constructors, custom extraction and custom function
returns charge one result copy to the existing **64 MiB shared list/Json/Bytes
arena**. Extracted builtins use their normal return rules and arenas; scalar
extractions and Boolean tag tests require no result arena allocation.

Malformed or missing values, bad constructor payloads and wrong-variant extraction
raise catchable `AIL-RUNTIME-VALUE`. Custom size/depth/count/allocation limits raise
fatal `AIL-RUNTIME-VALUE-LIMIT`; internal ABI failures are fatal VALUE-IO.
Embedded builtin size/item/depth violations also use VALUE-LIMIT during custom
value validation; malformed builtin grammar uses VALUE. Standalone builtin
operations, integer-expression errors and builtin result-arena errors keep their
existing categories.
An extracted builtin whose canonical transport exceeds its own value bound also
fails with VALUE-LIMIT before publication; a later arena reservation failure uses
the builtin's normal arena category.
`recover VALUE` selects this category; `recover any` includes it. Recovery preserves
earlier effects and allocations, while a failed operation or call preserves its
destination. Temporary runtime storage is released before a handler runs.

Custom function parameters are validated in sorted name order, including unused
ones, before checking call depth. Parameters borrow input; successful custom
returns copy it. Existing call depth, instruction fuel and local-variable limits
continue to apply. Immutable tree reconstruction repeatedly validates and copies
subtrees; the resource bounds can be reached before a final output becomes large.

## Checked artifacts and STE

Bytecode carries a closed `type_schemas` map from concrete type names to variant
and payload maps. Empty metadata is omitted. Core retains owned Type/Variant/Field
declarations and typed operation operand edges. Packed executable constructor
arguments are not unchecked prose. Source/Core diagnostics use AIL-VALUE-001;
protocol errors use AILBC024, with existing function diagnostics for signatures.
Bytecode checking rejects conflicting typed uses of a custom field, including
across actions and function calls. Unused descriptive declarations retain their
existing behavior; an ambiguous generic head rejects when an executable root uses it.

The fixed instructions are `VALUE_CONSTRUCT`, `VALUE_TEST` and `VALUE_GET`.
STE uses these templates, with separately declared technical data:

```text
Construct type_data variant variant_data with arguments arguments_data into field key_data.
Test type_data variant variant_data of field source_data into field key_data.
Get field field_data of type_data variant variant_data from field source_data into field key_data.
```

Static schema bounds are 4,096 concrete types, 65,536 total variants and payload
fields each, 256 variants per type, 256 fields per variant and a 4 MiB encoded
schema. Type expressions have at most 32 nesting levels, 4,096 bytes per concrete
spelling and 128 characters per identifier segment. Invalid schemas reject before
execution, including schemas referenced only by unreachable instructions.

The [binary tree application](../../examples/binary-tree/README.md) demonstrates
the [reusable library](../../examples/ail_std_tree.ail/README.md). Its
[R019 benchmark](../../bench/R019-binary-tree/intent.md) compares insertion,
in-order traversal, search and height with a frozen independent Python tree.
The [implementation plan](../superpowers/plans/2026-10-11-executable-variants.md)
records verification and review. The full Python-replacement goal remains active.
