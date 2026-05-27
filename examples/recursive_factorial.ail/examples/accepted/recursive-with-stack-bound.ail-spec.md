Function: bounded retry.

The function needs:

- n: Int

The function produces:

- result: Int

When bounded retry runs:

- the function calls bounded retry with n
- the function has a maximum recursion depth of 64
- the function returns the recursive result
- the function records a trace event named BoundedRetryCalled
