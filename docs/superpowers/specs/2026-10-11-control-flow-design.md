# Typed comparisons and source control flow

The continuing goal is STE between specifications and binaries, then a practical
Python replacement for AI agents. The verified baseline is 8/16 frozen capability
rungs with checked i64 expressions. This stage implements runtime decisions and
source loops, not only new report artifacts or benchmark special cases.

Source action bullets add `if <integer comparison>:`, `otherwise:`, `end if`,
`while <integer comparison>:`, `end while`, `break`, and `continue`. Every line
remains a bullet; indentation is cosmetic. Comparisons are `== != < <= > >=`
between full integer expressions. Nested blocks and skipped branches have real
runtime behavior: an unchosen arithmetic expression is never evaluated.
Requirements, reads, result declarations, guarantees and trace declarations stay
outside control blocks; only writes, action calls, repeated calls, and control
statements belong inside. `returns` remains output selection, not an early exit.
Reject unmatched/duplicate delimiters, break/continue outside loops, ambiguous
comparison chains, non-Int comparison operands, and excessive nesting.

Use `AilControl::{If(IntCondition), Else, EndIf, While(IntCondition), EndWhile,
Break, Continue}` and `AilActionStep::Control(usize)`. Add `controls` to AilAction.
Store control nodes with `control_kind`, linked from Action by `control_step` and
ordered with the same checked `step_index` used by writes/calls/repeats. Conditions
are typed `IntegerComparison : Bool` nodes with `comparison` and operand edges to
existing typed IntegerExpression nodes. Core graphs, not provenance text, decide
execution. Reconstruct the structured source and preserve imports and saved Core.

Lower structured control to deterministic LABEL/JUMP plus `BRANCH_INT`. Its
operands are `left` (postfix IntExpr), `comparison` (symbol), `right` (postfix),
and `label`. The branch jumps when true. Existing VM/native execution fuel bounds
loops. Native comparisons use signed CPU flags without subtracting operands in
host code, including MIN/MAX. Source expressions stay checked i64; this stage
does not claim Python integer compatibility or full ASD-STE100 conformity.

STE template: `If integer comparison {left} {comparison} {right} is true, go to
label {label}.` Operands are declared data identifiers. Bytecode verifies both
expression encodings, the comparator, and the branch label before execution.
Native inlining prefixes BRANCH_INT labels and preserves nested output scopes.
Unsupported native mixtures remain explicit compilation errors.

New module `condition.rs`: public `IntComparison::{Equal, NotEqual, Less,
LessEqual, Greater, GreaterEqual}`, `token()`, `from_token(&str)->Result<Self,String>`;
public `IntCondition { left:IntExpr, comparison:IntComparison, right:IntExpr }`,
`parse`, `from_postfix(left,comparison,right)`, `to_source`, `eval`, `fields`.
`condition` is independent of ail; reuse expression limits and checked evaluation.

Root owns `control_flow.rs`, Core/parser/order/VM/STE integration and end-to-end
cases. Independent workers own condition.rs and the native BRANCH_INT extension.
Preserve existing work and frozen benchmark inputs; only add implementations for
R003/R004/R007/R008/R009/R010. Check signed boundaries, graph tampering, imports,
branch omission, nested loops, break/continue/calls, saved artifacts and fuel.
