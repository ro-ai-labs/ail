Function: measured countdown.

The function needs:

- n: Int

The function produces:

- result: Int

When measured countdown runs:

- if n is 0, the function returns 0
- otherwise the function calls measured countdown with n
- the function has a well-founded termination measure n that decreases to 0 on every recursive call
- the function returns the recursive result
- the function records a trace event named MeasuredCountdownCalled
