# Rejected Lock Guard Unknown Resource Fixture

System component: Packet scheduler.

The component guards:

- scheduler state with scheduler lock

The component uses:

- scheduler lock: Bool

The component owns:

- scheduler lock

The component places:

- scheduler lock in scheduler region

The component requires capability:

- use scheduler lock

The component performs:

- read scheduler lock

The component records:

- PacketSchedulerLocked

The component guarantees:

- every lock guard names a declared protected resource
