rung: R005-factorial
tier: 0
action: Factorial
targets: vm,native
blocked-on: calls that return values; call frames

# R005-factorial — Recursive factorial

Given `n.value` (0 <= n <= 20), set `out.result` to n!.

The corpus already contains a 'recursive factorial' example. It does not compute
a factorial: FUNCTION_CALL and FUNCTION_RETURN push a trace line and fall through.
