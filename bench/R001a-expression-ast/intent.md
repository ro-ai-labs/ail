rung: R001a-expression-ast
tier: 0
action: AddNumbers
targets: vm
blocked-on: expression IR node (operator + 2 operand refs); AIL-EXPR-001 hard-rejects non-literal operands

# R001a — Represent `a + b` as an expression, not a string

First rung of the R001-add decomposition. Given `sum.a` and `sum.b`, compute
`sum.total = a + b`. Same computation as R001-add itself; this rung isolates
the first blocking capability instead of demanding the whole interlocking
pipeline at once.

The checker must represent `sum.a + sum.b` as a typed expression node with an
operator and two operand references — not an English sentence stashed in a
`value=` attribute, and not a hard rejection. Today
`check_action_arithmetic_operand_literals` (AIL-EXPR-001) rejects any
non-literal arithmetic operand outright, so there is nothing downstream to
test yet: the program cannot get past `ail-check`.

`targets: vm` — native codegen for this same expression is a separate rung
(R001e). Reaching a `pass` on R001a still requires the VM to produce the
correct sum, since final program state is the only channel `ail_bench.py` can
observe; R001b through R001e each narrow which pipeline stage has to be real
to keep producing that same observable result. See the ladder report for why
an internal-only property like "is this really an AST node" cannot be scored
more directly than that under the current runner contract.
