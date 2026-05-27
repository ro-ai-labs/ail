# Accepted Idempotent Increment Request Fixture

The application Idempotent Counter Example manages retryable counter requests.

A Counter has:

- value: Int
- processed request id: Text

A IncrementRequest has:

- id: Text

Action: Idempotent increment counter.

When idempotent increment counter happens:

- the system requires the increment request id to exist
- the system requires the increment request id not to be processed request id
- the system reads counter value
- the system reads increment request id
- the system increments counter value by 1
- the system changes counter processed request id to Applied
- the system guarantees the increment request id is used as the idempotency key
- the system guarantees duplicate retries do not increment the counter twice
- the system records a trace event named CounterIncremented
