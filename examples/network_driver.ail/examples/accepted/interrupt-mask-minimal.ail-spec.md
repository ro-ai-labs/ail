# Accepted Interrupt Mask Fixture

System component: Timer interrupt handler.

The component uses:

- tick counter: Int

The component owns:

- tick counter

The component places:

- tick counter in interrupt region

The component runs in context:

- interrupt

The component masks interrupt:

- interrupt: mask lower priority interrupts

The component requires capability:

- use tick counter

The component performs:

- read tick counter
- write tick counter

The component records:

- TimerInterruptHandled

The component guarantees:

- timer interrupts run with declared masking
