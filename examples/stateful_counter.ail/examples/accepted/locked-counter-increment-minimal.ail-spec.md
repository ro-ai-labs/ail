# Accepted Locked Counter Increment Fixture

The application Shared Counter Example manages concurrent shared counters.

A Counter has:

- value: Int

System component: Counter store.

The component uses:

- counter value: Int
- counter lock: Bool

The component owns:

- counter value
- counter lock

The component places:

- counter value in counter region
- counter lock in counter region

The component guards:

- counter value with counter lock

The component requires capability:

- use counter value

The component performs:

- read counter value

The component records:

- CounterStoreLocked

The component guarantees:

- counter value is protected by counter lock

Action: Increment shared counter.

When concurrent increment shared counter happens:

- the system reads counter value
- the system increments counter value by 1
- the system guarantees shared counter updates are serialized by counter lock
- the system records a trace event named CounterIncremented
