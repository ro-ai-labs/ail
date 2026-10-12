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

- max: Int

Action: max.

When max happens:

- if n.a >= n.b:
- the system computes out.max as n.a
- otherwise:
- the system computes out.max as n.b
- end if
- the system returns out.max
- the system records a trace event named Calculated
