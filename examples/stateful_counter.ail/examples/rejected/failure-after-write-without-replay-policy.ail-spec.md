# Rejected Failure After Write Without Replay Policy Fixture

The application Replayable Counter Example manages replayable counter writes.

A Counter has:

- value: Int

Action: Replayable increment counter.

When replayable increment counter after failure happens:

- the system requires the counter to exist
- the system reads counter value
- the system increments counter value by 1
- if StoreWriteFailed
- the system records a trace event named CounterIncremented

Failure StoreWriteFailed happens when the store fails after the counter value is written:

- the caller retries later
- the trace records CounterIncrementReplayDeferred
