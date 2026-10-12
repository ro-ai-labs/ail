The application Integer Arithmetic manages integer results.

A N has:

- a: Int
- b: Int

An Out has:

- difference: Int
- product: Int
- quotient: Int
- remainder: Int

Action: arithmetic.

When arithmetic happens:

- the system computes out.difference as n.a - n.b
- the system computes out.product as n.a * n.b
- the system computes out.quotient as n.a / n.b
- the system computes out.remainder as n.a % n.b
- the system returns out.difference
- the system returns out.product
- the system returns out.quotient
- the system returns out.remainder
- the system records a trace event named Calculated
