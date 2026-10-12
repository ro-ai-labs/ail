# Boolean predicates

Boolean expressions combine flags and typed scalar comparisons. They run through
checked source/Core, STE, the VM and standalone Linux x86-64 ELF.

```text
- the system computes boolean output.accept as data.enabled and integer data.count > 0 and text data.kind == "task"
- if boolean output.accept or floating data.score >= 0.75:
- the system records a trace event named Selected
- end if
- while boolean not data.done and integer data.index < data.count:
- the system computes data.index as data.index + 1
- end while
```

Declare computed destinations as `Bool`. A bare field in a predicate must also
be `Bool`. `true`, `false`, `not`, `and`, `or` and parentheses are supported.
Precedence is `not`, then `and`, then `or`; parentheses override it. There is
no truthiness or implicit type conversion.

## Typed comparisons

Each comparison starts with its operand type and uses one of
`==`, `!=`, `<`, `<=`, `>`, `>=`.

| Prefix | Operands | Ordering |
| --- | --- | --- |
| `integer` | Existing Int arithmetic expressions | Checked signed 64-bit values |
| `floating` | Existing Float arithmetic expressions | Binary64, with existing NaN/signed-zero rules |
| `text` | Declared Text fields or JSON string literals | Unicode scalar lexicographic order |
| `boolean` | Bool fields, literals or parenthesized predicates | `false < true` |

For example, `boolean (integer data.count > 0) == data.enabled` compares two
predicates. `text data.name == "and (data.name)"` compares a field to that exact
string; words inside quotes are data. Escapes, NUL and Unicode are preserved.
Text ordering performs no normalization. Chained comparisons are rejected.
Float NaN compares unequal to every value, including itself; its ordered
comparisons are false. Positive and negative zero compare equal.

`and` and `or` evaluate left to right and skip unnecessary right operands.
`false and integer data.count / 0 > 1` returns false without division. Reached
comparisons evaluate the left operand before the right. Static checks still
inspect every operand, including skipped types, ownership and secret access.

## Functions and runtime limits

Use `the function computes boolean local.result as ...` in an executable
function with declared locals. Boolean conditions work in functions whose
parameters/results are otherwise all Int. A Bool result or argument continues
to use the existing bare-field call/return contract: first compute a predicate
into a declared Bool local. Function entry validates every supplied argument,
including arguments unused by a short-circuited predicate.

Expressions are limited to **65,536 source/encoded bytes, 1,024 nodes and depth
128**, counting embedded numeric trees and comparisons together. Source nesting
is bounded even when redundant parentheses would disappear from the AST.
Core checks count raw stored operand attributes before normalization, including
original JSON escape spellings in saved-Core Text operands.

A reached Bool transport must be exactly `true` or `false`; missing or malformed
values report `AIL-RUNTIME-BOOL`. Reached Text fields must contain at most
1,048,576 bytes of UTF-8, using the existing `AIL-RUNTIME-TEXT` and
`AIL-RUNTIME-TEXT-LIMIT` families. Numeric failures retain their existing
families. Diagnostics omit operand contents. A failed computation preserves
its destination. Boolean results consume no Text/list result-arena budget.
Skipped subtrees perform no runtime field reads, arithmetic or value allocation.

Only Application and C interop profiles support these executable forms.
Descriptive function and compiler-pass bodies reject them. Action computations
record `computed boolean <key>`; Boolean branches add no implicit trace.

## Core, bytecode and STE

An untyped `BooleanOperation` carries `boolean_operation_kind=compute`, an ordered
`boolean_step`, a Bool `boolean_destination` and a `boolean_expression` root.
`BooleanExpression:Bool` carries `expression_kind=literal|field|not|and|or|integer|floating|text|boolean`.
Literals have `value`; comparisons have `comparison`. Unary negation uses
`boolean_operand`; binary nodes use `boolean_left`/`boolean_right`; field leaves
use `boolean_field`. Numeric comparisons reuse IntegerExpression/FloatExpression
trees. `TextOperand:Text` uses `operand_kind=literal|field`, with `value` or
`text_field`. Each operand has exactly one suitable parent and scope.

Controls use `control_kind=if_boolean|while_boolean` with the existing `condition`
edge. Function controls use `value_condition`; local computations use
`value_expression_operand`. Local tree nodes carry the exact `function_owner`.
Extra, dangling, shared, orphan or foreign-owned nodes/edges are rejected.

`EVAL_BOOL` requires exactly `key,expression`; `BRANCH_BOOL` requires exactly
`expression,label`. Source/Core errors use `AIL-BOOL-001`; bytecode validation
uses `AILBC018`. Typed postfix preserves the AST so both runtimes evaluate lazily:

| Values/operators | Transport |
| --- | --- |
| Bool / Bool field | `b:true`, `b:false`, `bf:data.enabled` |
| Int / Int field | `i:1`, `if:data.count`; `i+ i- i* i/ i%` |
| Float / Float field | `d:0.75`, `ff:data.score`; `f+ f- f* f/ f% fneg` |
| Text / Text field | `s:"text with spaces"`, `tf:data.name` |
| Typed comparison | `icmp:>=`, `fcmp:==`, `tcmp:!=`, `bcmp:<` |
| Boolean operators | `not`, `and`, `or` |

```text
Compute Boolean expression predicate_data into field destination_data.
If Boolean expression predicate_data is true, go to label selected_label.
```

STE declares technical data separately and reconstructs the exact operands.
Native output emits conditional jumps over skipped subtrees, retains the normal
field layout and adds Float runtime support only when a predicate needs it.

## Examples and verification

The [record filter](../../examples/boolean-filter) combines a Bool flag, Text
alternatives, an Int bound and a Float threshold in a reusable function. Its
sample writes a report with **2 accepted and 3 rejected** records. The
[JSON inspection example](../../examples/json-inspection) now uses direct Text
predicates to classify containers and escape dynamic JSON Pointer tokens.

Run `python3 scripts/ail_bool_differential.py` after `cargo build` for the
independent oracle. The [implementation record](../superpowers/plans/2026-10-11-boolean-predicates.md)
tracks tests and reviews. The wider Python-replacement goal remains active.

The independent oracle checks 29,400 predicate results per backend, 94 generated
predicates across the public pipelines and 48 record-filter datasets. Review
probes additionally cover exact raw-byte limits, 10,000 generated AST/near-limit
roundtrips, 954 scalar comparisons and 208 native boundary cases. The frozen
capability ladder remains 16/16; full Python replacement is still unproved.

Final verification passes **1,026 Rust tests across 21 targets**, strict Clippy,
formatting, the independent oracles and all three code reviews. The implementation
record links the evidence and the fixes found by those reviews.
