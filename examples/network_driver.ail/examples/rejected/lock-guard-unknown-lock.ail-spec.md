# Rejected Lock Guard Unknown Lock Fixture

System component: Packet scheduler.

The component guards:

- scheduler state with scheduler lock

The component uses:

- scheduler state: Int

The component owns:

- scheduler state

The component places:

- scheduler state in scheduler region

The component requires capability:

- use scheduler state

The component performs:

- read scheduler state

The component records:

- PacketSchedulerLocked

The component guarantees:

- every lock guard names a declared lock resource
