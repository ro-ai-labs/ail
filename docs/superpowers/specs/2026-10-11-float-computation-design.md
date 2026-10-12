# Floating-point computation through STE and native ELF

The active objective remains an agent-usable Python replacement with ASD-STE100
based controlled English between checked specifications and binary emission.
The previous typed-function milestone is verified progress (868 tests). The next
missing capability is binary64 computation for fractions, averages and numeric
records, with the same semantics in the VM and standalone native output.

## Approach and rationale

Add a concrete `Float` type representing IEEE binary64, including signed zero,
subnormals, infinity and canonical NaN. Keep existing checked i64 semantics.
Use explicit floating expressions and conversions; no implicit Int/Float coercion.
This preserves existing integer source/Core/protocol and makes an agent's type
choices visible. Json arrays can supply sequences of Float values via typed
lookup; List<Float> remains a subsequent collection extension, not a substitute
for completing this end-to-end scalar computation work.

Native arithmetic may use SSE2, with an embedded allocation-free Rust numeric
runtime for exact decimal parsing/formatting and remainder. A throwaway probe in
`/tmp/ail-float-probe` successfully links core parsing/formatting into an ELF and
runs without libc, an interpreter or a dynamic loader. Hand-writing a second
correctly-rounded parser/formatter would create avoidable divergent semantics;
calling a host runtime from generated programs would break standalone output.
The selected runtime is built from checked-in source by the existing Rust
compiler at Cargo build time, then embedded. AIL compilation does not invoke a
compiler or linker and emitted programs require no external runtime files.

## Scalar transport and arithmetic

Use strict ASCII decimal float transport: optional leading sign, one or more
digits with optional fraction or a leading dot followed by digits, and optional
e/E signed decimal exponent with digits. No whitespace, underscores, hex or
trailing data. Also accept `inf`, `+inf`, `-inf`, `nan`, `+nan`, `-nan`; other
spellings are errors. Inputs are bounded to 1 MiB. Decimal conversion uses
binary64 round-to-nearest/ties-to-even, preserving signed zero and subnormals;
overflow becomes signed infinity, underflow becomes signed zero. Normalize all
NaNs to bits 0x7ff8000000000000. Diagnostics never include operand contents.

Canonical Float output is shortest round-trip lowercase scientific notation,
using Rust LowerExp for finite values (`1.5e0`, `-0e0`, `5e-324`), and `inf`,
`-inf`, `nan` otherwise. It fits in 32 bytes and uses numeric storage without
text/list arena charges. Both VM and native use the same explicit format contract.

Arithmetic operators are +, -, *, / and %, plus unary signs and parentheses.
Use binary64 operations without fused contraction or fast-math. Division or
remainder by either signed zero fails `AIL-RUNTIME-DIVZERO`. Other infinities,
overflow and NaNs follow IEEE operations; normalize NaN results. Remainder uses
Python's divisor-sign modulo (fmod then sign correction), including signed zero
and infinite operands. Comparisons ==, !=, <, <=, >, >= follow IEEE unordered
semantics: NaN compares unequal to everything, including itself; only != is true.

Conversions are explicit: Int to Float rounds to nearest/ties-to-even; Float
to Int truncates toward zero and rejects NaN/infinity and values outside i64.
Bounds must be checked before cast (2^63 is out of range; -2^63 is valid).
Invalid/missing float transport fails FLOAT; excessive input fails FLOAT-LIMIT;
out-of-range conversion fails FLOAT. Existing integer operand errors retain INT.

## Source, AST and typed Core

Add these ordered statements to actions and local value-function bodies:

```
the system computes floating output.mean as data.total / data.count
the system converts integer data.count to floating data.countFloat
the system converts floating data.mean to integer output.whole
```

Functions use `the function` and declared local fields. Add floating controls:
`if floating <expression> <comparison> <expression>:` and
`while floating <expression> <comparison> <expression>:`. Existing otherwise,
end if/end while, break and continue work across mixed integer/float nesting.
All field leaves of a floating expression must have type Float; integer-valued
literal tokens inside one are Float literals. Conversion source/destination
must be exactly Int/Float in the stated direction.

Float expressions have the existing 65536 byte, 1024 node and depth128 bounds;
conditions count the comparison node and combined inputs as existing conditions
do. Build a separate FloatExpr (literal bits, field, binary, unary negation),
FloatCondition, and typed FloatExpression/FloatComparison Core trees. Reject
unknown metadata, wrong leaf types, dangling/duplicate/shared or foreign-local
operands and malformed expression trees. Preserve opaque provenance and original
execution order. Actual global operands imply secret reads/writes; imported
names qualify globals/targets and preserve locals. Reuse the stricter unique
Thing/global-field ownership check established for general calls.

An AilFloatOperation records compute/conversions and is represented by FloatOperation
Core nodes, ordered float_step edges, float_destination/float_source/float_expression
roles. Functions embed it as ValueOperation::Float with existing typed local
operand-role machinery. Extend AilControl with IfFloat/WhileFloat, preserving
all existing integer forms. Parsing/rendering Core back to source is canonical.

## Bytecode, functions, literals and JSON

- EVAL_FLOAT has exactly key,expression; postfix float literals use `d:<decimal>`,
  fields `f:<key>`, binary operators + - * / %, unary negation `neg`.
- BRANCH_FLOAT has exactly left,comparison,right,label.
- INT_TO_FLOAT and FLOAT_TO_INT have exactly source,key.
- Float functions use VALUE_FUNCTION/CALL_VALUE/RETURN_VALUE. Float arguments
  and returns are bare typed fields, like other non-Int values; compute an
  expression into a Float local before passing or returning it. Validate every
  parameter in sorted name order before depth entry. Float is a scalar and
  incurs no return arena charge; native packets carry bits, never borrowed
  numeric-buffer pointers. Existing depth64/fuel/ownership checks apply.
- Float ASSIGN_LITERAL accepts only a strict JSON number which converts to a
  finite binary64 value. Negative zero remains signed. Arithmetic expressions
  can explicitly contain inf/nan; JSON literals do not add nonstandard tokens.
- JSON_PARSE/FORMAT/GET/SET support Float. Parse/get require a strict JSON number
  whose conversion is finite; format/set reject nonfinite Float, and encode
  finite values in canonical scientific form. Json itself continues preserving
  original number tokens. Existing JSON grammar/bounds/error priority/budgets
  remain unchanged; these conversions use JSON/JSON-LIMIT errors.

Application/C interop profiles only. Compiler passes and legacy descriptive
functions do not gain executable numeric behavior. Invalid source/Core uses
AIL-FLOAT-001 (function wrappers may use AIL-FUNCTION-001); bytecode uses
AILBC017. Direct STE enforces all operand/syntax/profile/scoped rules.

STE templates:

```
Compute floating expression {expression} into field {key}.
If floating comparison {left} {comparison} {right} is true, go to label {label}.
Convert integer field {source} into floating field {key}.
Convert floating field {source} into integer field {key}.
```

## Native runtime contract

Build a deterministic fixed-address Linux x86-64 ET_EXEC runtime at 0x10000000
with Rust core only. Keep symbol addresses and PT_LOAD data in generated build
metadata. Validate embedded ELF architecture, segment bounds, non-overlap and
absence of dynamic/interpreter requirements. Runtime segment addresses and
permissions remain fixed when appended to emitted ELF; add program headers
without changing RIP-relative references within the existing generated code.
No user path or source controls this build or embedded object. Runtime functions
obey SysV ABI; the emitter must align RSP before calls and preserve its own
RBX/RBP/R14/R15 and any live caller-saved registers. Panic exits nonzero without
result stdout. Include only for programs that actually need float helpers.

Shared numeric module must work under no_std with no allocation. Export parse,
format, binary arithmetic/comparison and conversion helpers using integer bits
and explicit status/output pointers, avoiding f64 ABI ambiguity. No implicit
heap allocation or host file/process effects. Existing root arenas and function
frames remain intact. Native numeric field buffers already have 32 bytes.

## Evidence

Tests must cover decimal ties, all exponent ranges, largest finite values,
smallest subnormals, signed zeros, NaN/infinity, unordered comparisons, zero
errors, modulo sign, integer conversion edges and malformed transports.
Independent Python comparisons must check result bits/classification rather than
assuming Python's decimal spelling equals AIL's scientific spelling. Verify
thousands of seeded random values and operations, source/Core/STE × VM/native,
local recursive calls, JSON data processing, immutable caller storage, mixed
controls, validation/type/profile/import/secret checks and native standalone ELF
metadata. Keep prior 868 tests and frozen16 benchmark semantics unchanged.

Primary references checked during design: Rust core f64 documentation
https://doc.rust-lang.org/core/primitive.f64.html and Python numeric type semantics
https://docs.python.org/3/library/stdtypes.html#numeric-types-int-float-complex.
Build/runtime behavior is verified against the installed stable Rust toolchain,
not inferred from newer documentation APIs.
