The application Integer Control Flow manages computed decisions and loops.

A N has:

- a: Int
- b: Int
- value: Int

A Work has:

- a: Int
- b: Int
- i: Int
- temp: Int
- value: Int
- divisor: Int

A Out has:

- prime: Int

Action: is prime.

When is prime happens:

- the system computes out.prime as 0
- if n.value >= 2:
- the system computes out.prime as 1
- the system computes work.divisor as 2
- while work.divisor <= n.value / work.divisor:
- if n.value % work.divisor == 0:
- the system computes out.prime as 0
- break
- end if
- the system computes work.divisor as work.divisor + 1
- end while
- end if
- the system returns out.prime
- the system records a trace event named Calculated
