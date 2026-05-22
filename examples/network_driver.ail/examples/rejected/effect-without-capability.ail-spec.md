# Rejected Network Driver Fixture

System component: Network packet receiver.

The component uses:

- rx buffer: Buffer
- network device: Device

The component performs:

- read network device

The component records:

- PacketReceived

The component guarantees:

- every packet read is stored in rx buffer before release
