# Rejected Resource Without Region Fixture

System component: Network packet receiver.

The component uses:

- rx buffer: Buffer
- packet metadata: Buffer
- network device: Device

The component owns:

- rx buffer

The component borrows:

- packet metadata

The component requires capability:

- access network device
- read packet metadata

The component performs:

- read network device
- read packet metadata

The component records:

- PacketReceived

The component guarantees:

- every packet read is stored in rx buffer before release
