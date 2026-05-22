# Rejected Mutate Borrowed Resource Fixture

System component: Network packet receiver.

The component uses:

- rx buffer: Buffer
- network device: Device

The component owns:

- rx buffer

The component borrows:

- rx buffer

The component places:

- rx buffer in packet processing region

The component requires capability:

- access network device

The component performs:

- read network device
- write rx buffer

The component records:

- PacketReceived

The component guarantees:

- every packet read is stored in rx buffer before release
