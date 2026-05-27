# Rejected Shared Counter Without Lock Fixture

The application Shared Counter Example manages concurrent shared counters.

A Counter has:

- value: Int

Action: Increment shared counter.

When concurrent increment shared counter happens:

- the system requires the counter to exist
- the system reads counter value
- the system increments counter value by 1
- the system records a trace event named CounterIncremented
