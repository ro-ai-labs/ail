# Network Driver AIL-Spec Example

System component: Network packet receiver.

The component uses:

- rx buffer: Buffer
- packet metadata: Buffer
- network device: Device

The component owns:

- rx buffer

The component borrows:

- packet metadata

The component places:

- rx buffer in packet processing region
- packet metadata in packet processing region

The component requires capability:

- access network device
- read packet metadata

The component performs:

- read network device
- read packet metadata
- write rx buffer
- release rx buffer

The component records:

- PacketReceivedScenario073

The component guarantees:

- every packet read is stored in rx buffer before release
