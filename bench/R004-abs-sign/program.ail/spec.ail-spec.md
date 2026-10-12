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

- absolute: Int
- sign: Int

Action: abs sign.

When abs sign happens:

- the system computes out.absolute as n.value
- the system computes out.sign as 0
- if n.value < 0:
- the system computes out.absolute as -n.value
- the system computes out.sign as -1
- otherwise:
- if n.value > 0:
- the system computes out.sign as 1
- end if
- end if
- the system returns out.absolute
- the system returns out.sign
- the system records a trace event named Calculated
