# Accepted Mutable Borrow Fixture

System component: DMA writer.

The component mutably borrows:

- dma ring

The component uses:

- dma ring: Buffer

The component places:

- dma ring in dma region

The component requires capability:

- use scheduler

The component performs:

- write dma ring

The component records:

- DmaRingWritten

The component guarantees:

- every dma ring write stays inside the dma region
