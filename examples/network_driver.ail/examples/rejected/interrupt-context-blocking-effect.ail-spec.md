# Rejected Interrupt Context Blocking Effect Fixture

System component: Timer interrupt handler.

The component uses:

- timer device: Device

The component runs in context:

- interrupt

The component requires capability:

- access timer device

The component performs:

- read timer device
- wait for scheduler

The component records:

- TimerInterruptHandled

The component guarantees:

- interrupt context does not block
