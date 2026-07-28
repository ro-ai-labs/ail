rung: R002-arithmetic
tier: 0
action: Arithmetic
targets: vm,native
blocked-on: subtract/multiply/divide/modulo opcodes

# R002-arithmetic — The four remaining operations

Given `n.a` and `n.b` (b != 0), compute `out.difference`, `out.product`,
`out.quotient` (truncating toward zero), and `out.remainder`.
