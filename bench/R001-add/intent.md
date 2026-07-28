rung: R001-add
tier: 0
action: AddNumbers
targets: vm,native
blocked-on: expression IR; runtime (non-literal) operands

# R001-add — Add two integers

Given `sum.a` and `sum.b`, compute `sum.total = a + b`.

The smallest possible program that is not expressible today: AIL's only
arithmetic opcode, ADD_INT_FIELD, requires a compile-time literal operand.
