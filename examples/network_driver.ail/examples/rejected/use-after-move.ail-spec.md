# Rejected Use After Move Fixture

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
- read rx buffer

The component records:

- PacketHandedOff

The component guarantees:

- moved rx buffers are not reused by the component
