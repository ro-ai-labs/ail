# Accepted Persistent Increment Fixture

The application Persistent Counter Example manages persistent stateful counters.

A Counter has:

- value: Int

Action: Persist counter increment.

When persist counter increment happens:

- the system requires the counter to exist
- the system reads counter value
- the system increments counter value by 1
- the system guarantees counter value is persisted before the next replay
- the system records a trace event named CounterIncremented
