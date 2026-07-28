rung: R001c-typed-values
tier: 0
action: MirrorValues
targets: vm
blocked-on: typed runtime value model (Value::Int vs raw text); state is BTreeMap<String,String> today

# R001c — Hold two typed runtime values at once

Third rung of the R001-add decomposition. Given `sum.a` and `sum.b`, set
`sum.mirror_a` to the value of `sum.a` and `sum.mirror_b` to the value of
`sum.b`. No operator at all — this isolates the value model from the
evaluator (R001d), which needs the same two runtime values held
simultaneously before it can combine them with `+`.

The proof is canonicalization, not addition. Case-03 passes `sum.a=007` and
`sum.b=-0`. A VM that moves field text around as bytes reproduces `007` and
`-0` verbatim. A VM that reads a field as a typed `Int` and writes a typed
`Int` reproduces `7` and `0`. There is no opcode today that reads one runtime
field and writes a different one — `ADD_INT_FIELD` only ever adds a
compile-time literal to the field it already holds — so this program cannot
be expressed at all yet, let alone run.

`targets: vm` — native codegen for value routing is folded into R001e, which
tests the harder case (an operator) rather than a separate rung for plain
copies.
