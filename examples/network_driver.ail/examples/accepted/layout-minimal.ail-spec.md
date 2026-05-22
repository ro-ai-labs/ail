# Accepted Layout Fixture

System component: Packet layout.

The component uses:

- packet header: Buffer

The component owns:

- packet header

The component places:

- packet header in packet region

The component lays out:

- packet header: repr(C), align 8

The component requires capability:

- use scheduler

The component performs:

- read packet header

The component records:

- PacketLayoutChecked

The component guarantees:

- packet header layout is ABI visible
