rung: R009-collatz
tier: 0
action: Collatz
targets: vm,native
blocked-on: data-dependent loop bound

# R009-collatz — Collatz step count

Given `n.value` (n >= 1), set `out.steps` to the number of steps to reach 1,
where even -> n/2 and odd -> 3n+1. The trip count is not known at compile time,
so this cannot be satisfied by unrolling.
