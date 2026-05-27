The application Maintenance Runner manages repeated stateful tasks.

A Counter has:

- value: Int

Action: Increment counter.

When increment counter happens:

- the system requires the Counter to exist
- the system reads the Counter value
- the system changes the Counter value by incrementing it by 1
- the system guarantees atomic increment of the Counter value
- the system records a trace event named CounterIncremented
- the system records a trace event named MaintenanceCycleCompleted

Action: Run maintenance cycle.

When run maintenance cycle happens:

- the system requires the Counter to exist
- the system repeats Increment counter 3 times sequentially
- the system records a trace event named MaintenanceCycleCompleted

Failure IncrementCounterFailure happens when the Counter value cannot be incremented:

- the system does not change the Counter value
- the trace records IncrementCounterFailure

Failure RunMaintenanceCycleFailure happens when one or more Increment counter actions fail:

- the system does not record MaintenanceCycleCompleted
- the trace records RunMaintenanceCycleFailure