# Rejected Layout Unknown Resource Fixture

System component: Packet layout.

The component uses:

- packet header: Buffer

The component owns:

- packet header

The component places:

- packet header in packet region

The component lays out:

- dma ring: repr(C), align 64

The component requires capability:

- use scheduler

The component performs:

- read packet header

The component records:

- PacketLayoutChecked

The component guarantees:

- every layout declaration names a component resource
