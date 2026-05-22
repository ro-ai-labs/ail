# Rejected Allocation Unknown Resource Fixture

System component: Packet allocator.

The component uses:

- packet buffer: Buffer

The component owns:

- packet buffer

The component places:

- packet buffer in packet region

The component allocates:

- dma ring: heap

The component requires capability:

- use scheduler

The component performs:

- read packet buffer

The component records:

- PacketBufferAllocated

The component guarantees:

- every allocation declaration names a component resource
