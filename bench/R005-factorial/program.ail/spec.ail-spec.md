The application Factorial manages integer results.

A N has:

- value: Int

An Out has:

- result: Int

Function: factorial.

The function needs:

- n: Int

The function produces:

- result: Int

The function stores:

- partial: Int

The function executes:

- if local.n <= 1:
- the function returns 1
- end if
- the function calls factorial with n = local.n - 1 into local.partial
- the function returns local.n * local.partial

Action: Factorial.

When Factorial happens:

- the system calls factorial with n = n.value into out.result
- the system returns out.result
- the system records a trace event named Calculated
