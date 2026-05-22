# Rejected Move Without Ownership Fixture

System component: Packet handoff.

The component uses:

- rx buffer: Buffer

The component places:

- rx buffer in packet processing region

The component requires capability:

- use scheduler

The component performs:

- move rx buffer

The component records:

- PacketHandedOff

The component guarantees:

- only owned rx buffers are moved out of the component
