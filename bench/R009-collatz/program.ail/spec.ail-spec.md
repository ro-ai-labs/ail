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

- steps: Int

Action: collatz.

When collatz happens:

- the system computes out.steps as 0
- the system computes work.value as n.value
- while work.value != 1:
- if work.value % 2 == 0:
- the system computes work.value as work.value / 2
- otherwise:
- the system computes work.value as 3 * work.value + 1
- end if
- the system computes out.steps as out.steps + 1
- end while
- the system returns out.steps
- the system records a trace event named Calculated
