The application Integer Arithmetic manages integer results.

A Sum has:

- a: Int
- b: Int
- total: Int
- mirror_a: Int
- mirror_b: Int
- label: Text

Action: mirror values.

When mirror values happens:

- the system computes sum.mirror_a as sum.a
- the system computes sum.mirror_b as sum.b
- the system returns sum.mirror_a
- the system returns sum.mirror_b
- the system records a trace event named Calculated
