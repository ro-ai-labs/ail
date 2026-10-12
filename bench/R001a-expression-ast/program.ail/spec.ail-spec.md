The application Integer Arithmetic manages integer results.

A Sum has:

- a: Int
- b: Int
- total: Int
- mirror_a: Int
- mirror_b: Int
- label: Text

Action: add numbers.

When add numbers happens:

- the system computes sum.total as sum.a + sum.b
- the system returns sum.total
- the system records a trace event named Calculated
