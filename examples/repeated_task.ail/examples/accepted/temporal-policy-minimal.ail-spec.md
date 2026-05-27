The application Maintenance Runner manages repeated stateful tasks.

A Counter has:
- value: Int

Action: Increment counter.
When increment counter happens:
- the system increments the counter value by 1
- the system records a trace event named CounterIncremented

Action: Run maintenance cycle.
When run maintenance cycle happens:
- the system repeats IncrementCounter 3 times
- the system claims scheduler behavior for daily maintenance
- the system uses temporal policy daily maintenance window
- the system records a trace event named MaintenanceCycleCompleted
