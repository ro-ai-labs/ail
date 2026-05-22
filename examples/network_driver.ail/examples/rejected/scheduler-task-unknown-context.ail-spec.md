# Rejected Scheduler Task Unknown Context Fixture

System component: Packet scheduler.

The component uses:

- scheduler state: Int

The component owns:

- scheduler state

The component places:

- scheduler state in scheduler region

The component schedules task:

- packet poller: process

The component requires capability:

- use scheduler state

The component performs:

- read scheduler state

The component records:

- PacketPollingScheduled

The component guarantees:

- every scheduled task names a component context
