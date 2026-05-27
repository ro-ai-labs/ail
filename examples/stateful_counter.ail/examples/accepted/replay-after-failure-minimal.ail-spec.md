# Accepted Replay After Failure Fixture

The application Replayable Counter Example manages replayable counter writes.

A Counter has:

- value: Int
- replay request id: Text

Action: Replayable increment counter.

When replayable increment counter after failure happens:

- the system requires the replay request id to exist
- the system reads counter value
- the system increments counter value by 1
- if StoreWriteFailed
- the system guarantees failed replay preserves prior counter value and resumes with the request id
- the system records a trace event named CounterIncremented

Failure StoreWriteFailed happens when the store fails after the counter value is written:

- the system preserves the prior counter value
- the trace records CounterIncrementReplayDeferred
