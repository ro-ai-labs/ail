# Rejected Retryable Increment Without Idempotency Key Fixture

The application Retryable Counter Example manages retryable counter requests.

A Counter has:

- value: Int

Action: Retryable increment counter.

When retryable increment counter after network retry happens:

- the system requires the counter to exist
- the system reads counter value
- the system increments counter value by 1
- the system records a trace event named CounterIncremented
