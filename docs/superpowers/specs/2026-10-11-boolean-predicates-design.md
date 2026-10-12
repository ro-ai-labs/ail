# Boolean predicates through checked Core and STE

The continuing objective is an agent-usable Python replacement with a checked
Specs→Core→ASD-STE100-based STE→VM/native pipeline. The previous turn was verified
progress: JSON inspection, 981 tests, independent oracles and reviews. Current
functions cannot directly combine conditions or compare Text; the recursive
JSON example uses text search to classify kind words. Add reusable Boolean
values and conditions with explicit scalar comparisons and short-circuit logic.

## Design

Use one bounded BoolExpr AST for computation and control in actions/functions.
This keeps static type/access checks independent of runtime short-circuiting.
Desugaring source to ad hoc jumps before Core would hide predicate structure
from reviewers and agents. Adding only Text branches would leave flags and
combined predicates unexpressed. Preserve existing integer/floating syntax and
semantics while adding the complete Boolean predicate layer below.

```text
- the system computes boolean output.accept as data.enabled and integer data.count > 0 and text data.kind == "object"
- if boolean output.accept or floating data.score >= 0.75:
- the system assigns true to output.selected
- end if
- while boolean not data.done and integer data.index < data.count:
- the system computes data.index as data.index + 1
- end while
```

Functions substitute `the function` and use declared locals. Compute Bool
expressions into a Bool local to return/pass them through existing typed calls.
Existing bare-field non-Int argument/return contracts remain. Application and
C interop only; descriptive functions/passes reject new executable forms with
both subjects. Recognized malformed compute/if/while forms are errors.

## Grammar and static semantics

Precedence is `not`, then `and`, then `or`; parentheses group Boolean expressions.
Primary forms are true/false, a declared Bool field, a parenthesized predicate,
or a typed comparison. Keywords are ASCII words, never recognized inside a
quoted JSON string or dotted identifier. Comparisons are:

- `integer <IntExpr> <comparison> <IntExpr>`
- `floating <FloatExpr> <comparison> <FloatExpr>`
- `text <TextOperand> <comparison> <TextOperand>`
- `boolean <BoolAtom> <comparison> <BoolAtom>`

Comparisons support ==, !=, <, <=, >, >=. TextOperand is a declared Text field
or strict JSON string literal. BoolAtom is true/false, a Bool field or a
parenthesized Boolean expression. Thus Boolean comparison can compare grouped
predicates. There is no comparison chaining, implicit numeric coercion,
truthiness or conversion between unrelated scalar types. Bool ordering is
false before true; Text ordering is lexicographic Unicode scalar ordering,
without normalization. All six Float comparisons retain NaN/signed-zero rules.

Existing numeric expression syntax, arithmetic errors and leaf types apply.
The entire predicate shares 65,536 input/encoded bytes, 1,024 AST nodes and depth
128, including embedded numeric trees and comparison nodes. Source parentheses
and unary nesting are bounded even when the normalized AST is small. Saved
Core must enforce combined raw operand bytes before literal normalization as
well as decoded structure. Text literals use JSON escaping and preserve NUL and
Unicode; source/rendered/Core/STE round trips cannot reinterpret string contents.

## Shared AST and transport interfaces

New public module src/bool_expression.rs:

```text
TextOperand = Literal(String) | Field(String)
BoolExpr = Literal(bool) | Field(String) | Not(Box<BoolExpr>)
         | And {left:Box<BoolExpr>,right:Box<BoolExpr>}
         | Or {left:Box<BoolExpr>,right:Box<BoolExpr>}
         | CompareInt(IntCondition) | CompareFloat(FloatCondition)
         | CompareText {left:TextOperand,comparison:IntComparison,right:TextOperand}
         | CompareBool {left:Box<BoolExpr>,comparison:IntComparison,right:Box<BoolExpr>}
```

Methods: parse(&str), from_postfix(&str), validate() -> Result; to_source() and
to_postfix() -> String; eval(&BTreeMap<String,String>) -> Result<bool,String>;
typed_fields() -> Result<BTreeMap<String,String>,String>; map_fields(Fn(&str)->String)
-> Self; uses_float() -> bool. Reject conflicting required types for one key.

Postfix is a typed AST transport, not eager execution: b:true/b:false and
bf:<key>; Int i:<decimal>/if:<key> with i+, i-, i*, i/, i%; Float d:<decimal>/ff:<key>
with f+, f-, f*, f/, f%, fneg; Text s:<JSON-string>/tf:<key>. The tokenizer must
read quoted JSON strings as one token even with whitespace/keywords. Typed
comparison tokens icmp:<op>, fcmp:<op>, tcmp:<op>, bcmp:<op> produce Bool ASTs.
Boolean operators are not, and, or. Exact one Bool result, correct stack types,
full input consumption and all combined bounds are required. Use numeric ASTs
rather than reimplementing integer/Float arithmetic.

## Runtime

Evaluate operands left to right. And evaluates its right only if left is true;
Or evaluates right only if left is false. Not negates a Bool. Every comparison
evaluates/validates left then right. Skipped subtrees perform no runtime reads,
arithmetic, UTF-8 checks or allocation. Static checking still checks every
operand, including skipped types, secret access and malformed graph structure.

Bool transport is exactly true/false; missing/malformed reached fields use
AIL-RUNTIME-BOOL. Reached Text operands validate UTF-8 and the existing 1 MiB
limit before comparison, using TEXT/TEXT-LIMIT. Int/Float operands retain their
existing error families and omit operand contents. Predicate results use scalar
true/false storage with no text/list arena charge. Failed computation preserves
destination. Function parameters are still all validated in sorted order before
call depth; short-circuiting inside a function does not suppress this boundary.

## Core representation and API

New AilBooleanOperation::Compute {key:String,expression:BoolExpr} plus Invalid.
BooleanOperation has exactly boolean_operation_kind=compute, no type, an ordered
boolean_step under its action, boolean_destination (Bool Field), and
boolean_expression (BooleanExpression). BooleanExpression nodes have type Bool,
expression_kind literal/field/not/and/or/integer/floating/text/boolean; literal
has value true/false; comparison variants have comparison. Local nodes also
carry exactly function_owner. Binary children use boolean_left/boolean_right;
not uses boolean_operand; Bool field uses boolean_field. Integer/floating
comparison children are existing IntegerExpression/FloatExpression trees.
Text comparison children are new TextOperand nodes, type Text, operand_kind
literal/field; literals carry decoded value, fields use text_field. Boolean
comparison children are BooleanExpression. Each expression has exactly one
appropriate parent, no hidden/shared/orphan trees or extra metadata/edges.

Use existing numeric tree builders/decoders and genuine scoped Field checks;
add the narrowly typed Boolean parent to FloatExpression checking. Global fields
must belong to one genuine Thing declaration and have unique runtime keys.
Function-local trees must belong to the current executable value function.
Controls gain IfBoolean/WhileBoolean, control_kind if_boolean/while_boolean,
and bind BooleanExpression via condition (action) or value_condition (function).
Local computation uses existing value_expression_operand role expression with
OperandKind::BooleanExpression. Secret reads traverse every predicate leaf;
imports qualify field leaves without touching strings or local references.

New bool_operations module provides parse_bullet, instruction, verify, execute,
namespace, elaborate/from_core/check/accesses and build_expression/decode_expression
with the same owner/local signatures as float_operations. Expose existing scoped
Int decoding narrowly if needed; it must retain unique storage checks. The
Core checker independently validates raw aggregate bytes and full tree ownership.

## Instructions, STE and native

EVAL_BOOL requires exactly key,expression; BRANCH_BOOL exactly expression,label.
Source/Core diagnostics use AIL-BOOL-001; malformed bytecode uses AILBC018.
Computation trace is `computed boolean <key>`; branches have no implicit trace,
matching Float controls. STE templates:

```text
Compute Boolean expression {expression} into field {key}.
If Boolean expression {expression} is true, go to label {label}.
```

Native compiles the AST with conditional jumps, not eager postfix evaluation.
Text literals are static data; reached text fields use existing UTF-8 validation.
Compare valid UTF-8 lexicographically as unsigned bytes (same scalar ordering).
Keep field layout, owned numeric buffers, arenas, depth/fuel and helper ABIs.
Bool computations need no floating runtime unless a Float subtree is present;
collect typed leaf dependencies even in function-body-only predicates. No
new dependencies, interpreters, runtime files or AIL compile subprocesses.

## Verification

Require meaningful RED tests, then independent public source/Core/rendered/STE
and VM/native agreement. Cover precedence, grouping, quoted opaque strings,
all comparisons/types, Unicode/NUL, signed-zero/NaN, safe short-circuit of
missing/invalid fields, division by zero and native invalid UTF-8. Reached
operands must fail in left-to-right order. Cover global/local typing, stale
metadata, shared/orphan/foreign-owner trees, skipped secret operands, imports,
profiles, byte/node/depth bounds, malformed transport and static/dynamic aliasing.
Function entry still validates unused parameters; repeated predicates must not
consume arenas. Add a file-filtering example with a reusable Bool-returning
predicate and update the generic JSON example to direct text predicates. Run
independent Python oracle, full serial suite, strict lint/fmt, prior numeric,
JSON/function oracles and frozen benchmark. Keep the full goal active.
