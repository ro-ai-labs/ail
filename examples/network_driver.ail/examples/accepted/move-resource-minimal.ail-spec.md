# Accepted Move Resource Fixture

System component: Packet handoff.

The component uses:

- rx buffer: Buffer

The component owns:

- rx buffer

The component places:

- rx buffer in packet processing region

The component requires capability:

- use scheduler

The component performs:

- move rx buffer

The component records:

- PacketHandedOff

The component guarantees:

- every moved rx buffer leaves the component exactly once
