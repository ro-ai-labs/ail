# Rejected Read Effect Without Borrow Fixture

System component: Network packet receiver.

The component uses:

- rx buffer: Buffer
- packet metadata: Buffer
- network device: Device

The component owns:

- rx buffer

The component requires capability:

- access network device
- read packet metadata

The component places:

- rx buffer in packet processing region
- packet metadata in packet processing region

The component performs:

- read network device
- read packet metadata
- write rx buffer

The component records:

- PacketReceived

The component guarantees:

- every packet read is stored in rx buffer before release
