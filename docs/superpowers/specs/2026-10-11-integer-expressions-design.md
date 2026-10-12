# Typed expressions and runtime results

The full goal remains ASD-STE100 between specifications and binaries, then a
Python replacement usable by AI agents. The prior STE stage was progress; the
starting computation score was 1/16. This stage implements actual runtime
integer computation and does not redefine the full goal.

## Design

Add `the system computes sum.total as sum.a + sum.b` to action specifications.
Expressions support signed 64-bit literals, declared dotted field references,
parentheses, unary minus, and `+ - * / %` with usual precedence. Division
truncates toward zero, as specified by the frozen R002 oracle. All operations
are checked: malformed/missing integers, overflow, and division by zero fail
deterministically. This bounded Int type does not provide Python arbitrary
precision; that remains a future capability.

Core stores expression nodes (literal, field reference, binary operator) and
operand edges. Assignments connect the action, target field, and expression.
The checker resolves all operand and destination types, rejects unknown fields
and non-Int arithmetic, and checks graph structure. Core lowering reads this
structure; source prose is not the expression's authoritative representation.
Multiple writes to the same field preserve source order.

STE gains `Evaluate integer expression {expression} into field {key}.` The
expression data is a deterministic postfix AST encoding, parsed and verified
before execution. No native source compiler or Python runtime is introduced.
The VM evaluates typed values. Its existing string state remains the transport
boundary for legacy tooling, while arithmetic operands are explicitly Int.

Add `the system returns sum.total` (one field per bullet). This becomes
`OUTPUT_FIELD key` / `Select output field {key}.` A run retains its complete
internal state and separately identifies returned fields; CLI output uses those
fields when present. Text outputs preserve text exactly. Nested action outputs
do not replace the caller's result selection.

Native expression programs use real mutable field storage, not accumulated
compile-time deltas over argv. Arithmetic reads prior writes, values survive
calls and branches, and native outputs agree with VM results. Legacy native
programs continue on their existing path until that backend is consolidated.
Unsupported mixed operations fail compilation explicitly.

## Interfaces and implementation boundaries

`src/expression.rs`: `IntExpr::{Literal(i64), Field(String), Binary { op:
IntOperator, left: Box<IntExpr>, right: Box<IntExpr> }}`;
`IntOperator::{Add, Subtract, Multiply, Divide, Remainder}`;
`IntExpr::parse`, `from_postfix`, `to_postfix`, `to_source`, `fields`, and `eval`.
Parsing and graph reconstruction have explicit size/depth limits.

`src/native_expr.rs` is a child module of `ail`; `emit(action, failures)` returns
native ELF bytes using the existing X64Code helpers. `EVAL_INT` operands are
`key`, `expression` (postfix). `OUTPUT_FIELD` has `key`. Root returns select
output; inlined callee selection is saved, validated, and then restored. Errors go to stderr with nonzero
exit status. Native integer parsing rejects trailing junk and overflow.

The root implementer owns Core elaboration/checking/serialization, action
syntax/results, VM integration, and shared CLI changes. Expression parsing and
native emission can be implemented independently against these interfaces.

## Evidence

Add programs beneath frozen R001 and R002 benchmark directories without editing
their oracles or cases. Include typed text-preservation and integer-copy rungs.
Require VM/native/Python agreement, independently calculated edge cases,
Core/STE round trips, rejected type errors and corrupted Core/bytecode, repeated
assignments, nested expression use, and runtime arithmetic failures. Run the
existing full suite and static checks before recording any score increase.

Review additions: preserve ordering across writes/calls/repeats via checked
step indices; retain repeated edges and resolve forward action references.
Reject expression lowering errors instead of falling through to opaque writes.
