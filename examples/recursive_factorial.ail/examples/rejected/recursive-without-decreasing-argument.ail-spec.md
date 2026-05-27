Function: stuck retry.

The function needs:

- n: Int

The function produces:

- result: Int

When stuck retry runs:

- if n is 0, the function returns 0
- otherwise the function calls stuck retry with n
- the function returns the recursive result
- the function records a trace event named StuckRetryCalled
