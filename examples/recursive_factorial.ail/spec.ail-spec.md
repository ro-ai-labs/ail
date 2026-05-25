# Recursive Factorial AIL-Spec Example

The application Calculator manages arithmetic workflows.

Function: factorial.

The function needs:

- n: Int

The function produces:

- result: Int

When factorial runs:

- if n is 0, the function returns 1
- otherwise the function calls factorial with n minus 1
- the function returns n multiplied by the recursive result
- the function records a trace event named FactorialCalled
