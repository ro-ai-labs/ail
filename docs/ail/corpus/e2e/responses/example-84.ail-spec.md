# Repeated Task AIL-Spec Example

The application Maintenance Runner manages repeated stateful tasks.

A Counter has:

- value: Int

Action: Increment counter.

When increment counter happens:

- the system increments the counter value by 1
- the system records a trace event named CounterIncrementedScenario084

Action: Run maintenance cycle.

When run maintenance cycle happens:

- the system repeats IncrementCounter 3 times
- the system records a trace event named MaintenanceCycleCompletedScenario084
