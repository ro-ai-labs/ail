# Accepted Interrupt Context Fixture

System component: Timer interrupt handler.

The component uses:

- timer device: Device
- tick counter: Int

The component owns:

- tick counter

The component places:

- tick counter in interrupt region

The component runs in context:

- interrupt

The component requires capability:

- access timer device

The component performs:

- read timer device
- write tick counter

The component records:

- TimerInterruptHandled

The component guarantees:

- timer interrupts update the tick counter without blocking
