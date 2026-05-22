# Rejected Scheduler Task Priority Unknown Task Fixture

System component: Packet scheduler.

The component uses:

- scheduler state: Int

The component owns:

- scheduler state

The component places:

- scheduler state in scheduler region

The component runs in context:

- process

The component sets task priority:

- packet poller: realtime

The component requires capability:

- use scheduler state

The component performs:

- read scheduler state

The component records:

- PacketPollingScheduled

The component guarantees:

- every scheduler task priority names a component task
