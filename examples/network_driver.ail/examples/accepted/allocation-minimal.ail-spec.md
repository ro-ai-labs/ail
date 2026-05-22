# Accepted Allocation Fixture

System component: Packet allocator.

The component uses:

- packet buffer: Buffer

The component owns:

- packet buffer

The component places:

- packet buffer in packet region

The component allocates:

- packet buffer: stack

The component requires capability:

- use scheduler

The component performs:

- allocate packet buffer

The component records:

- PacketBufferAllocated

The component guarantees:

- packet buffer allocation placement is explicit
