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

- total: Int

Action: sum to n.

When sum to n happens:

- the system computes work.i as 1
- the system computes out.total as 0
- while work.i <= n.value:
- the system computes out.total as out.total + work.i
- the system computes work.i as work.i + 1
- end while
- the system returns out.total
- the system records a trace event named Calculated
