# AIL-System Profile

## Purpose

AIL-System is the profile for systems programming. It extends AIL-Core with
memory, concurrency, device, ABI, runtime, and lowering obligations while
preserving English explanations and traceability.

## Systems Scope

AIL-System must be able to express a kernel, runtime, driver, scheduler, memory
manager, filesystem, network stack, embedded application, compiler backend, and
performance-critical library over time.

The first implementation may cover small slices, but the model must not close
off those targets.

## Memory And Layout

Memory and layout semantics describe representation, alignment, allocation,
copying, movement, pinning, stack placement, heap placement, device memory, and
ABI-visible structure. Each layout decision must be explainable.

## Ownership And Borrowing

ownership defines who may mutate or release a value. Borrowing defines who may
read or temporarily mutate without taking ownership. The checker rejects use
after release, conflicting mutable access, and hidden sharing that violates the
declared contract.

## Regions And Lifetimes

A region groups values with related lifetime and allocation behavior. Region
rules must explain when values enter a region, when they leave, who can observe
them, and why an escape requires a different allocation strategy.

## Scheduling And Concurrency

Scheduling rules define tasks, interrupts, locks, queues, join points, parallel
branches, cancellation, priority, and fairness. Concurrency semantics must make
read/write conflicts, ordering, and trace joins explicit.

## Device And OS Capabilities

Device and OS interactions use capabilities. A program cannot access a device,
file descriptor, network socket, timer, process, or privileged instruction
without a declared capability and effect.

The first implemented AIL-System slice accepts a regular component surface:

```text
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

The component lays out:

- rx buffer: repr(C), align 8

The component allocates:

- rx buffer: stack

The component guards:

- rx buffer with rx lock

The component runs in context:

- interrupt

The component sets interrupt priority:

- interrupt: high

The component masks interrupt:

- interrupt: mask lower priority interrupts

The component schedules task:

- packet poller: interrupt

The component sets task priority:

- packet poller: realtime

The component sets task timing:

- packet poller: deadline 10 ms, budget 2 ms

The component requires capability:

- access network device
- read packet metadata

The component performs:

- read network device
- read packet metadata
- write rx buffer
- release rx buffer

The component records:

- PacketReceived

The component guarantees:

- every packet read is stored in rx buffer before release
```

The parser lowers this form into a `SystemComponent` node with `Resource`,
`Capability`, `Effect`, `Trace`, and `Guarantee` nodes. Effects that use a
regular resource verb such as `read`, `write`, `access`, `release`, `map`,
`unmap`, `allocate`, `free`, `pin`, `unpin`, `reset`, or `configure` must name
a declared component resource. The elaborator records that binding with a
`targets_resource` edge from the effect to the resource.

Capabilities that use a regular resource verb such as `access`, `read`,
`write`, `use`, `configure`, or `reset` also bind to declared resources. The
elaborator records that binding with an `authorizes_resource` edge from the
capability to the resource. Device resource effects must be covered by a
matching capability for the same device.

Ownership bullets bind the component to resources it may mutate or release.
Mutable resource effects such as `write`, `release`, `free`, `unmap`, `pin`,
`unpin`, `append`, or `delete` must target a resource the component owns or
mutably borrows. Move effects such as `move rx buffer` transfer a resource out
of the component and must target a resource the component owns. The elaborator
records ownership with an `owns_resource` edge from the component to the
resource.

Borrowing bullets bind the component to resources it may read without taking
ownership. Non-device read effects must target a resource the component owns or
borrows. Device reads remain capability-governed because device access is
already checked through matching capabilities. The elaborator records borrowing
with a `borrows_resource` edge from the component to the resource.

Mutable borrowing bullets bind the component to resources it may mutate or
release without taking ownership. Mutable resource effects may target resources
the component owns or mutably borrows, and read effects may also target mutably
borrowed resources. The elaborator records mutable borrowing with a
`mutably_borrows_resource` edge from the component to the resource.

The first borrow-checking rules treat shared borrowing and mutable borrowing as
exclusive access contracts. A component must not mutate a resource while that
resource is declared as borrowed, even if the component also owns that
resource. A component also must not declare the same resource in both
`The component borrows:` and `The component mutably borrows:`. Later lifetime
scopes can refine these coarse rules into shorter active borrow intervals.

The first lifetime rule treats `release` and `free` effects as ending the
usable lifetime of the targeted resource. Later effects in the same component
must not read, write, release, or otherwise target that resource.

The first move rule treats `move` effects as ending the component's local
ownership of the targeted resource. Later effects in the same component must
not read, write, release, move, or otherwise target that resource.

Placement bullets bind component resources to named regions. A non-device
resource used by an effect must be placed in a region so the checker can attach
lifetime and allocation reasoning to that resource. The elaborator records the
component-to-region relationship with a `uses_region` edge and the
resource-to-region relationship with an `in_region` edge.

Layout bullets bind component resources to ABI-visible representation rules.
The first layout surface uses `The component lays out:` bullets shaped as
`<resource>: <layout rule>`, such as `rx buffer: repr(C), align 8`. The
elaborator records the component-to-layout relationship with a `uses_layout`
edge and the layout-to-resource relationship with a `layouts_resource` edge.

Allocation bullets bind component resources to explicit placement decisions.
The first allocation surface uses `The component allocates:` bullets shaped as
`<resource>: <allocation placement>`, such as `rx buffer: stack`. The
elaborator records the component-to-allocation relationship with a
`uses_allocation` edge and the allocation-to-resource relationship with an
`allocates_resource` edge.

Lock guard bullets bind a protected resource to the component resource that
guards access to it. The first lock-guard surface uses
`The component guards:` bullets shaped as `<resource> with <lock resource>`,
such as `rx buffer with rx lock`. The elaborator records the
component-to-guard relationship with a `uses_lock_guard` edge, the
guard-to-protected-resource relationship with a `guards_resource` edge, and
the guard-to-lock-resource relationship with a `uses_lock_resource` edge.

Execution context bullets bind a component to the runtime context in which its
effects run. The first execution-context surface uses
`The component runs in context:` bullets shaped as `<context>`, such as
`interrupt`. The elaborator records the component-to-context relationship with
a `runs_in_context` edge. The first interrupt-context rule rejects blocking
effects such as `wait`, `sleep`, `block`, or `park` because interrupt handlers
must not suspend while holding hardware context.

Interrupt priority bullets bind a declared execution context to its scheduling
priority. The first priority surface uses
`The component sets interrupt priority:` bullets shaped as
`<context>: <priority>`, such as `interrupt: high`. The elaborator records the
component-to-priority relationship with a `uses_interrupt_priority` edge and
the priority-to-context relationship with a `prioritizes_context` edge.

Interrupt mask bullets bind a declared execution context to an interrupt
masking rule. The first mask surface uses
`The component masks interrupt:` bullets shaped as `<context>: <mask rule>`,
such as `interrupt: mask lower priority interrupts`. The elaborator records
the component-to-mask relationship with a `uses_interrupt_mask` edge and the
mask-to-context relationship with a `masks_context` edge.

Scheduler task bullets bind a named task to a declared execution context. The
first scheduler surface uses `The component schedules task:` bullets shaped as
`<task>: <context>`, such as `packet poller: interrupt`. The elaborator records
the component-to-task relationship with a `schedules_task` edge and the
task-to-context relationship with a `task_runs_in_context` edge.

Scheduler task priority bullets bind a declared scheduler task to its
scheduler priority. The first task-priority surface uses
`The component sets task priority:` bullets shaped as `<task>: <priority>`,
such as `packet poller: realtime`. The elaborator records the
component-to-priority relationship with a `uses_task_priority` edge and the
priority-to-task relationship with a `prioritizes_task` edge.

Scheduler task timing bullets bind a declared scheduler task to its real-time
deadline and execution budget. The first task-timing surface uses
`The component sets task timing:` bullets shaped as
`<task>: deadline <duration>, budget <duration>`, such as
`packet poller: deadline 10 ms, budget 2 ms`. The elaborator records the
component-to-timing relationship with a `uses_task_timing` edge and the
timing-to-task relationship with a `times_task` edge.

The checker rejects a component that performs effects without at least one
declared capability using stable diagnostic `AIL021`. It rejects a resource
effect that does not target a declared resource using stable diagnostic
`AIL022`. It rejects a device effect whose target resource is not authorized by
a matching component capability using stable diagnostic `AIL023`. It rejects a
mutable resource effect without matching component ownership or mutable
borrowing using stable diagnostic `AIL024`. It rejects a non-device read effect
without ownership or borrowing using stable diagnostic `AIL025`. It rejects a
non-device resource effect whose resource has no region placement using stable
diagnostic `AIL026`. It rejects a mutable effect against a borrowed resource
using stable diagnostic `AIL027`. It rejects later use of a released resource
using stable diagnostic `AIL028`. It rejects a resource declared as both shared
and mutable borrowed using stable diagnostic `AIL029`. It rejects later use of
a moved resource using stable diagnostic `AIL030`. It rejects a layout
declaration whose resource is not declared by the component using stable
diagnostic `AIL031`. It rejects an allocation declaration whose resource is not
declared by the component using stable diagnostic `AIL032`. It rejects a
blocking effect in interrupt context using stable diagnostic `AIL033`. It
rejects an interrupt priority declaration whose context is not declared by the
component using stable diagnostic `AIL034`. It rejects a scheduler task whose
context is not declared by the component using stable diagnostic `AIL035`. It
rejects a scheduler task priority whose task is not declared by the component
using stable diagnostic `AIL036`. It rejects a scheduler task timing whose task
is not declared by the component using stable diagnostic `AIL037`. It rejects a
lock guard whose protected resource is not declared by the component using
stable diagnostic `AIL038`. It rejects a lock guard whose lock resource is not
declared by the component using stable diagnostic `AIL039`. It rejects an
interrupt mask declaration whose context is not declared by the component using
stable diagnostic `AIL040`.

## Lowering Obligations

lowering obligations connect AIL-System semantics to target code. They include
memory layout, ownership preservation, call ABI, synchronization, allocation,
panic or failure strategy, trace preservation, and backend diagnostics.

## Human Explanations For Low-Level Semantics

AIL-System must explain low-level behavior in English:

```text
This buffer is owned by the network driver while the packet is being processed.
The packet parser may read the buffer but may not change it.
The driver releases the buffer when the packet has been handled.
```
