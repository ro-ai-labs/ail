# Accepted Packet Transmit Fixture

System component: Network packet transmitter.

The component uses:

- tx descriptor: Buffer
- tx queue: Buffer
- network device: Device

The component owns:

- tx descriptor
- network device

The component borrows:

- tx queue

The component places:

- tx descriptor in packet transmit region
- tx queue in packet transmit region

The component requires capability:

- access network device
- read tx queue

The component performs:

- read tx queue
- write network device
- release tx descriptor

The component records:

- PacketTransmitted

The component guarantees:

- queued packets are written to the network device before descriptor release
