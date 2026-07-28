rung: R007-sum-to-n
tier: 0
action: SumToN
targets: vm,native
blocked-on: loops reachable from source

# R007-sum-to-n — Sum 1..n

Given `n.value` (n >= 0), set `out.total` to the sum of the integers 1 through n.

Requires a loop the compiler can emit. LABEL/JUMP exist in the VM but no compiler
path emits them, and the native backend rejects backward jumps.
