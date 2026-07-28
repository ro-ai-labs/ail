rung: R001e-native-add
tier: 0
action: AddNumbers
targets: native
blocked-on: x86-64 instruction selection/register allocation for two runtime-loaded operands; emitter only adds a compile-time literal

# R001e — Native code adds two runtime values

Fifth and last rung of the R001-add decomposition. Given `sum.a` and
`sum.b`, compute `sum.total = a + b` and confirm it through the compiled
native binary alone (`targets: native`). R001d proves the tree-walking VM
can do this; this rung proves the x86-64 backend can too, by loading both
fields into registers, emitting a real `add`, and storing the result back —
the native emitter has no instruction selection or register allocation for a
binary operation over two runtime-loaded values today, only for adding a
compile-time literal to one field it already holds.

Once R001a through R001e all pass, R001-add itself should pass unmodified —
it is the same program, `sum.total = sum.a + sum.b`, checked and run both
ways at once.
