# Rejected Interrupt Mask Unknown Context Fixture

System component: Timer mask.

The component masks interrupt:

- interrupt: mask lower priority interrupts

The component uses:

- timer device: Device

The component requires capability:

- access timer device

The component performs:

- read timer device

The component records:

- TimerMaskConfigured

The component guarantees:

- every interrupt mask declaration names a component context
