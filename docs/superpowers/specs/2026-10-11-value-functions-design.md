# General value functions

Continue the active STE-to-binary and agent-usable Python replacement goal.
The previous JSON mutation milestone is verified progress. Programs now need
reusable record/text/list transformations with isolated storage, recursive calls
and typed results through source, checked Core, STE, VM and native ELF.

## Source and compatibility

Keep `The function executes:` and `The function stores:`. Parameters, locals and
the single declared result may be Int, Bool, Text, List<Int>, List<Text> or Json.
Names remain distinct ASCII identifiers; output names do not implicitly declare
locals and may coincide with an explicitly declared local. Only parameters are
initialized on entry. Existing integer compute/control/call/return/trace syntax
continues to work.

Add local literal, text, list, JSON conversion/access/mutation statements by
replacing their existing `the system` prefix with `the function`. All referenced
storage must be declared `local.name`. No file I/O, global storage access,
stateful action calls, tool operations or implicit output selection is allowed.
Legacy descriptive bodies and `the system` statements remain rejected in an
executable function. Do not weaken existing action/global operation checks.

Calls retain named `name = expression` arguments and `with none`; Int arguments
and returns accept checked integer expressions, while all other types require
a bare field reference of the exact declared type. A returned value immediately
exits the invocation. Action callers write the destination only after success.
Mutual/forward recursion, aliases and repeated calls must preserve caller values.

Keep AilFunction.integer/AilIntegerFunction/AilIntegerCall as compatible AST
containers. Extend IntegerStep with ValueOperation wrapping existing operation
ASTs. Classify a function as general when its signature/local declarations are
non-Int or its body contains ValueOperation; then promote every executable
caller that can reach a general function. Pure integer graphs retain existing
INTEGER_FUNCTION/CALL_INT/RETURN_INT lowering and performance. General bodies
can call general functions or existing integer functions. Source mutual
recursion through a general function therefore uses general frames throughout.

## Call boundary and execution contract

Evaluate arguments in sorted parameter-name order in the caller. Evaluate Int
expressions once. Validate every other argument against its declared type, but
retain the original immutable transport and charge no semantic arena bytes.
Text is bounded valid UTF-8; Bool is exactly true/false. Lists use the existing
list input grammar and raw/item/decoded limits, without requiring the canonical
encoding to fit until a canonical result is requested. Json validates complete
grammar, depth/count and canonical-size bounds. Invalid unused arguments still
fail. The callee cannot mutate caller storage while borrowing an argument.

At return, Int is a checked i64 and Bool is canonical true/false; neither charges
an arena. Text returns copy raw UTF-8 into the shared text arena. List/Json returns
canonicalize and copy into the shared list arena. Charge exactly one return copy
per invocation, including forwarding an existing allocated value; the caller
does not charge again. Empty Text costs one byte. The successful return copy
must outlive the callee frame and any subsequent caller writes. Body operations
retain their existing budgets, limits and error ordering; literals remain free.
Arguments and scratch do not charge result budgets or reset allocation counters.

Use existing type-specific runtime errors: INT (and arithmetic DIVZERO/OVERFLOW),
TEXT/TEXT-LIMIT, LIST/LIST-LIMIT/INDEX, and JSON/JSON-LIMIT/JSON-PATH. Bool boundary
errors use JSON/JSON-LIMIT. Missing non-Int operands use their type's error family.
Diagnostics never expose operand contents. Caller destination is unchanged on
failure; earlier executed body operations and trace events are not rolled back.

Preserve 256 combined parameters/locals, 4,096 source steps, 64 simultaneously
active executable function frames and per-invocation max(instructions*1024,1024)
fuel. Mixed general-to-integer calls share the 64-frame count. Pure integer
functions cannot call VALUE_FUNCTION directly in bytecode; source promotion
handles such call graphs. Falling through fails RETURN; depth and fuel use
existing errors. No implicit per-operation/call/return function traces: only
explicit function trace statements produce events. Native failures exit nonzero
without result stdout. VM host-OOM recovery remains unpromised.

## Typed protocol

- VALUE_FUNCTION begins its procedure, with exactly parameters,locals,result.
  Parameters/locals are sorted unique `name:Type` entries separated by semicolons,
  or empty strings. The sets are disjoint and total at most 256. Result is one
  supported type. Direct general bytecode permits at most 16,384 body instructions.
- CALL_VALUE has exactly target,arguments,key; arguments reuse existing named
  postfix encoding. Its target must be VALUE_FUNCTION, and all names/types match.
- RETURN_VALUE has exactly expression,type. The type equals the signature result;
  non-Int expressions must be one field reference. It is valid only in a general
  function. A general function uses RETURN_VALUE even for an Int result.
- General bodies allow existing EVAL_INT, BRANCH_INT, LABEL, JUMP, EMIT_TRACE,
  CALL_INT plus the local literal/text/list/JSON operations and new value protocol.
  Every local read/write must be declared with the exact required type, including
  expression leaves, JSON value types, list elements and callee signatures.
- Procedures require action callers, including CLI/all-actions/native bundles.
  Application/C interop only. Direct bytecode and STE enforce profile and scope.
  Compile errors use AIL-FUNCTION-001; protocol errors use AILBC011.

STE templates:

```
Start value function with parameters {parameters}, locals {locals}, and result type {result}.
Call value function {target} with arguments {arguments} into field {key}.
Return {type} value from expression {expression}.
```

## Core

Use Function(executable_value=true) and separate ValueFunctionStep nodes with
ordered value_function_step edges. Parameters/locals are typed owned Field nodes
with function_owner/local_name, connected by value_parameter/value_local. Existing
has_input/has_output metadata must match those declarations and the one result.
Use explicit typed value_field_operand{name=role} and
value_expression_operand{name=role} edges for operation data, keeping only
opcode-specific non-field metadata in attributes. Integer expressions remain
typed Core expression trees. Calls have explicit target/destination/argument
edges; non-Int arguments/returns reference typed fields. Checks reject extra
metadata, missing/duplicate operands, shared/orphan steps, cross-owner local
fields and conflicting executable markers. Preserve source control structures
for rendering; never store executable body prose as an unchecked attribute.

The Core executable marker must match classification recomputed from the decoded
source graph. An all-Int general Core function with no general operations or
callees is rejected instead of silently downgraded. Direct VALUE_FUNCTION
bytecode may still declare an all-Int signature.

Action general calls use ValueFunctionCall but retain integer_call_step for
ordering in the existing AilAction.integer_calls vector. Elaborate and decode
both call kinds in original vector/edge order. Existing integer checks explicitly
delegate only well-formed general owners/call nodes; new checks own those nodes.
Actual action call operands imply secret reads, including Int expression leaves,
and destination writes, independent of descriptive reads/writes. Imports qualify
function targets and action globals while leaving local identifiers and literal
contents unchanged.

## Implementation boundaries and evidence

Use separate value_function, value_protocol, value_vm and native_value_function
modules. Reuse existing parsers, operation execution and native helpers. Native
general slots use real pointer/length transport and caller-owned numeric buffers;
do not treat old integer-frame initialization flags as pointers. Keep root arena
metadata stable across frames. Helpers used only in function bodies must still
be emitted, and return/call labels must be unique across procedures.

VM can borrow parameter strings with Cow and own operation/return results;
avoid recursively cloning every unmodified large parameter. Execute existing
operation helpers using only relevant operand values, not a clone of the whole
frame. Expose the old integer machine's depth-aware entry for general callers.

Test all six types, recursion/interoperability, locals, lifetime, early returns,
strict Core/protocol/profile/import/secret checks, exact structure/byte/budget
limits and malformed unused arguments. Build a file-report example using a
Json-to-Json transformation function, integer helper and Text/List helpers.
Compare source/Core/STE × VM/native with independent Python data transformations;
run full serial suite, strict lint/format, previous oracles and frozen benchmarks.

This milestone does not complete the broader Python replacement goal. Numeric
expansion, broader library/ecosystem support and STE conformity work remain.
