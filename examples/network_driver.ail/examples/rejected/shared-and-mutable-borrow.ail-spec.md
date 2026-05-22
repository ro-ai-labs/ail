# Rejected Shared And Mutable Borrow Fixture

System component: Network packet receiver.

The component uses:

- dma ring: Buffer

The component borrows:

- dma ring

The component mutably borrows:

- dma ring

The component places:

- dma ring in packet processing region

The component requires capability:

- read dma ring

The component performs:

- read dma ring

The component records:

- PacketReceived

The component guarantees:

- every dma ring read stays inside the packet processing region
