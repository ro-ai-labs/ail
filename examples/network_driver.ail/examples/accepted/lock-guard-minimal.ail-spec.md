# Accepted Lock Guard Fixture

System component: Packet scheduler.

The component guards:

- scheduler state with scheduler lock

The component uses:

- scheduler state: Int
- scheduler lock: Bool

The component owns:

- scheduler state
- scheduler lock

The component places:

- scheduler state in scheduler region
- scheduler lock in scheduler region

The component requires capability:

- use scheduler state

The component performs:

- read scheduler state

The component records:

- PacketSchedulerLocked

The component guarantees:

- scheduler state is protected by scheduler lock
