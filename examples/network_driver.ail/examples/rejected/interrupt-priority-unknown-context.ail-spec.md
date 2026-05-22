# Rejected Interrupt Priority Unknown Context Fixture

System component: Timer priority.

The component uses:

- timer device: Device

The component sets interrupt priority:

- interrupt: high

The component requires capability:

- access timer device

The component performs:

- read timer device

The component records:

- TimerInterruptHandled

The component guarantees:

- every interrupt priority declaration names a component context
