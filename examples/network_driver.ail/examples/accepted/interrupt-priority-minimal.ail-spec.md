# Accepted Interrupt Priority Fixture

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

The component sets interrupt priority:

- interrupt: high

The component requires capability:

- access timer device

The component performs:

- read timer device
- write tick counter

The component records:

- TimerInterruptHandled

The component guarantees:

- timer interrupts run at declared priority
