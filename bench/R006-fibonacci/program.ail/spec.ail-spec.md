The application Fibonacci manages integer results.

A N has:

- value: Int

An Out has:

- result: Int

Function: fibonacci.

The function needs:

- n: Int

The function produces:

- result: Int

The function stores:

- left: Int
- right: Int

The function executes:

- if local.n <= 1:
- the function returns local.n
- end if
- the function calls fibonacci with n = local.n - 1 into local.left
- the function calls fibonacci with n = local.n - 2 into local.right
- the function returns local.left + local.right

Action: Fibonacci.

When Fibonacci happens:

- the system calls fibonacci with n = n.value into out.result
- the system returns out.result
- the system records a trace event named Calculated
