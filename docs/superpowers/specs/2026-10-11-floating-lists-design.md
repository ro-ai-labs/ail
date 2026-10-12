# Floating lists through source, STE and standalone native execution

The active objective is an agent-usable Python replacement with checked specs
lowering through ASD-STE100-based controlled English to binaries. The previous
goal turn was verified progress: scalar Float, 923 tests, independent Python
oracles and reviews. Current code supports Int/Text lists but not List<Float>.
This extension enables reusable numerical dataset transformations and statistics
without serializing every intermediate element through Json.

## Approach

Extend the existing immutable flat-list implementation with a third element
type Float. Reuse its five operations, limits, ownership and allocation model.
This keeps old Int/Text behavior and the shared scalar binary64 contract.
A separate numerical array subsystem would duplicate collection semantics;
restricting Float lists to finite values would discard valid scalar values and
prevent Python-like numerical programs from retaining NaN/infinity. Therefore
runtime Float lists support every Float value, while JSON boundaries remain
strict and finite. Generic nested collections are subsequent work, not part of
this flat-list extension.

## Source, Core and protocol

Declare `List<Float>` for global fields, function parameters/locals/results,
literal destinations and JSON bridges. Existing empty, count and get syntax
infers the element type. Add `appends floating <FloatExpr> to list ...` and
`sets item <IntExpr> in list ... to floating <FloatExpr> into ...`.
Indices remain checked Int expressions; Float values use the existing bounded
FloatExpr. Empty/append/set destination is List<Float>; count is Int; get Float.
There is no implicit conversion of Int fields to Float.

Extend AilListValue with Floating(FloatExpr), and Core ListOperation
element_type=Float. Action list_value binds a FloatExpression tree, with exactly
one parent. Extend the Float ownership checker to permit this specific parent;
all metadata, profile, global unique-storage and local ownership checks remain.
Value-function list operations use existing value_expression_operand typed
Float roles. Secret reads include Float expression leaves under unary negation.
Imports qualify globals/targets while preserving local fields.

Reuse LIST_EMPTY/LENGTH/GET/APPEND/SET and existing STE templates with type=Float.
The append/set value operand is Float postfix; the index stays Int postfix.
Malformed list instructions retain AILBC013 / AIL-LIST-001 diagnostics.
Functions continue VALUE_FUNCTION/CALL_VALUE/RETURN_VALUE; list arguments and
returns are bare typed fields. Compiler-pass/descriptive bodies do not acquire
execution support; supported profiles remain Application/C interop.

## Runtime transport and semantics

Float list input is a flat bracketed, comma-separated sequence of Float tokens,
with only ASCII JSON whitespace around tokens/separators. Each token uses the
existing Float scalar transport grammar, including signed decimals, fractions,
exponents and exact lowercase inf/+inf/-inf/nan/+nan/-nan. Examples:
`[1.25,-0,5e-324,nan,-inf]`, `[+001,.5,1.]`. Empty lists are `[]`.
Whitespace inside a token, nested values, strings, booleans, null, trailing
commas, missing elements, comments and extra data fail LIST. This is an internal
numeric transport; callers use explicit JSON conversion when they need JSON.
Int/Text list grammars remain unchanged.

Each parsed item carries normalized binary64 bits. Canonical producers emit
compact brackets/commas with canonical Float item transport: scientific finite
numbers, inf/-inf/nan, preserving signed zero. Read-only operations do not rewrite
the source. Get returns canonical Float in scalar storage, without arena charges.
Append/set evaluate arbitrary Float expressions and retain IEEE special values.

Validate the whole list before evaluating index, then bounds, then replacement
value, then result limits/allocation. Bad source is LIST/LIST-LIMIT; index uses
INT/INDEX; value expressions retain FLOAT/FLOAT-LIMIT/DIVZERO errors. Errors do
not reveal runtime values. Failed operations preserve destination and accounting.
Aliased source/destination and prior copies retain immutable semantics.

Preserve 65,536 items, 1,048,576 raw/produced bytes, and shared 67,108,864-byte
list/Json result arena. Empty costs2; append/set cost final encoded length; get
and length cost0. Borrowed function parameters validate raw shape/count but allow
canonical expansion beyond1MiB; successful returns canonicalize and charge one
copy, failing if canonical output exceeds the bound. Depth64 and sorted argument
validation before depth are unchanged. No intermediate arena charge.

## Strict JSON and literal boundary

List<Float> literal assignment and JSON parse/get require a flat array of strict
JSON number tokens, each converting to finite binary64. Leading plus/zeros,
leading dot, trailing decimal dot, inf/nan, or overflow to infinity fail. Negative
zero/subnormals/underflow remain valid. JSON format/set validates runtime Float
list grammar and rejects nonfinite members before encoding finite numbers.
These operations use JSON/JSON-LIMIT; literals use their existing static family.
General Json retains original number tokens; typed conversion alone rounds.
Direct flat-list conversion allows65,536 items, like existing lists. Json lookup
still applies the source document's existing total-value/depth limits first.

Shared allocation-free float_number::parse_json(&str)->Result<u64,Error> checks
one strict JSON number token (no surrounding whitespace), rejects nonfinite
results and preserves Float rounding. Export its native ABI as
ail_float_parse_json(ptr,len,out_bits)->status, with the same statuses as parse.
VM/native list parsers use this helper in strict parsing mode; formatting parses
runtime tokens and then checks finiteness. This avoids a recursive Json scratch
allocation for every numeric item and preserves the flat-list item limit.

## Native implementation

Existing list descriptors hold bits for Float in the same16-byte element slot;
native element code0 Int,1 Text,2 Float. Add strict/finite validation paths for
JSON as needed without changing normal runtime semantics. Generated Float list
operations use existing Float expression/runtime helpers. Include embedded
runtime for Float-list opcodes, List<Float> signatures/body-only operations,
literal and JSON bridges. Programs without Float keep their prior code path;
do not add unresolved Float helper references to integer-only list programs.
Function list parameters borrow descriptors after full validation; returns own
one canonical copy in the shared list arena. Keep64-byte field slots, private
numeric buffers, SysV alignment, root arenas and fuel/depth registers intact.

## Verification

Observe meaningful red tests, then require source/Core/saved/rendered/STE and
VM/native agreement. Cover ordinary values, all IEEE specials, signed zero,
subnormals, negative indices, error priority, aliases, list functions/recursion,
unused parameter validation, exact input/item/result/allocation bounds and
canonical expansion. Cover strict JSON/literal rejection, finite round trips,
all destination/source aliases and untouched lossless Json neighbors.
Malformed Core and protocol must fail before execution; secret operands and
imports remain checked. Add an independent Python oracle, a numerical file
example plus exported STE and guide. Run full serial tests, strict Clippy/fmt,
prior list/Float/JSON/value oracles and unchanged16/16 benchmark. Independent
reviews close frontend/root and native scopes. Keep the broader goal active.
