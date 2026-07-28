rung: R001d-vm-evaluates
tier: 0
action: AddNumbers
targets: vm
blocked-on: evaluator opcode combining two typed runtime values with an operator; opcodes are single-field, fixed-arity today

# R001d — The VM evaluates `a + b`

Fourth rung of the R001-add decomposition. Given `sum.a` and `sum.b`,
compute `sum.total = a + b` and confirm it through the bytecode VM alone
(`targets: vm`). R001a and R001b establish that the checker can accept and
type the expression; R001c establishes that the VM can hold two runtime
values at once. This rung is the first place the interpreter actually has to
read both values and combine them with `+` — today every VM opcode is
single-field and fixed-arity (`ADD_INT_FIELD` takes one field and a
compile-time literal delta); there is no opcode that reads two runtime
fields and produces their sum.

Native codegen for the same computation is deliberately deferred to R001e —
a working interpreter and a working native emitter are different amounts of
work in this codebase (the interpreter gets Rust's `+` for free once it has
both operands; the native backend has to emit the instruction itself), and
this rung isolates the smaller of the two.
