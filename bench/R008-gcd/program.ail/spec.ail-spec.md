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

- result: Int

Action: gcd.

When gcd happens:

- the system computes work.a as n.a
- the system computes work.b as n.b
- if work.a < 0:
- the system computes work.a as -work.a
- end if
- if work.b < 0:
- the system computes work.b as -work.b
- end if
- while work.b != 0:
- the system computes work.temp as work.a % work.b
- the system computes work.a as work.b
- the system computes work.b as work.temp
- end while
- the system computes out.result as work.a
- the system returns out.result
- the system records a trace event named Calculated
