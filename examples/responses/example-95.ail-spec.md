# Stateful Counter AIL-Spec Example

The application Counter Example manages stateful counters.

A Counter has:

- value: Int

Action: Increment counter.

When increment counter happens:

- the system increments the counter value by 1
- the system records a trace event named CounterIncrementedScenario095
