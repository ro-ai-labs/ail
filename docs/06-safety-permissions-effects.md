# Safety, Permissions, and Effects

EIGL aims for Rust-like safety without exposing Rust-like complexity to non-programmers.

The human-facing model is based on responsibility and permissions.

## Human safety language

Humans should understand safety through simple rules:

```text
One action is responsible for a thing at a time.
Many actions may read a thing at the same time.
Only one action may change a thing at a time.
A thing cannot be used after it has been consumed.
A temporary permission cannot outlive the thing it refers to.
A secret cannot be shown, logged, copied, or sent unless a rule permits it.
An external system cannot be called unless the action has permission.
A failure must be handled, returned, retried, compensated, or marked impossible.
```

Compiler terms map to human terms:

```text
responsibility         -> ownership
read permission        -> shared immutable borrow
change permission      -> exclusive mutable borrow
consume                -> move / affine use
temporary permission   -> lifetime-bounded borrow
secret boundary        -> information-flow constraint
external permission    -> capability
failure handling       -> typed Result / failure edge
```

## Permission kinds

```text
Own<T>          full responsibility for T
Read<T>         shared read-only permission
Change<T>       exclusive mutable permission
Move<T>         transfer responsibility
Consume<T>      use exactly once, then unavailable
Share<T>        safe cross-task sharing
Secret<T>       value with restricted flows
Capability<E>  permission to perform effect E
```

## Central aliasing rule

At any point in the execution graph, a value may have either:

```text
many Read permissions
```

or:

```text
one Change permission
```

but never both, unless a synchronization primitive or policy explicitly permits it.

## Permissions inferred from RIF

Human/RSL:

```text
Confirming an order changes the order.
```

RIF:

```text
permissions:
  change order
```

EIG-Core:

```text
PermissionRequirement(Change<Order>)
```

Backend:

```text
exclusive mutable access / transaction lock / versioned write / linear capability
```

The exact backend mechanism depends on whether the thing is an in-memory value, a database entity, an actor resource, or an external system.

## Effect kinds

```text
pure
read<Resource>
write<Resource>
call<Service>
emit<Event>
allocate<Resource>
release<Resource>
time
random
secret_read
secret_write
unsafe<Reason>
```

Pure operations:

```text
cannot read mutable state
cannot write mutable state
cannot call external systems
cannot read time or randomness
can be cached, reordered, inlined, vectorized, and parallelized
```

Effectful operations:

```text
must declare effects
must have required capabilities
must obey effect ordering
must expose failures
```

## Capability system

External powers must be explicit.

Example:

```text
action Send password reset email:
  needs permission to send email
  needs permission to read the user's email address
  must not read the user's password
```

Compiler form:

```text
requires:
  Capability<Email.Send>
  Read<User.email>

forbidden:
  Read<User.password_hash>
```

## Failure model

EIGL should not use implicit exceptions as the default.

Failures are visible and typed.

Example:

```text
action Capture payment:
  may fail with:
    CardDeclined
    PaymentProviderUnavailable
    FraudCheckFailed
```

Callers must handle failures:

```text
if payment capture fails because card is declined:
  mark the order as PaymentFailed
  tell the customer to use another payment method

if payment capture fails because provider is unavailable:
  retry up to 3 times
  then mark the order as PaymentPendingRetry
```

Compiler rule:

```text
Every declared failure must be handled, returned, retried, compensated, or marked impossible with proof.
```

## Secrets and information flow

Secret values have restricted flows.

Example:

```text
new_password: Secret<Text>
payment_method: Secret<PaymentMethod>
```

Rules:

```text
Secrets may not be logged.
Secrets may not be rendered in normal explanations.
Secrets may not be sent to unauthorized external services.
Secrets may not be copied into public fields.
Secrets may be transformed only by approved operations.
```

Example allowed flow:

```text
new_password -> Hash.password -> password_hash
```

Example forbidden flow:

```text
new_password -> EventLog.emit
```

## Trusted capsules

Unsafe or foreign low-level behavior must be isolated in trusted capsules.

Human-facing form:

```text
trusted capsule Fast image resize:
  reason:
    uses platform-specific SIMD instructions

  allowed to:
    read input image memory
    write output image memory

  must guarantee:
    does not read outside the input image
    does not write outside the output image
    does not keep references after it returns
    produces a valid image buffer
```

Compiler-facing form:

```text
foreign unsafe operation FastImageResize
requires proof_or_audit
effects read<InputBuffer>, write<OutputBuffer>
obligations:
  no_out_of_bounds_read
  no_out_of_bounds_write
  no_reference_escape
  output_validity
```

Trusted capsules must be visible in safety and security views.

## No hidden unsafe behavior

Safe EIGL cannot contain:

```text
raw pointer operations
unchecked memory access
unchecked casts
hidden dynamic code execution
unauthorized external calls
untracked mutation
untracked secret flows
```

Unsafe behavior may exist only inside trusted capsules with declared contracts.

## Concurrency safety

If two steps are unordered or parallel, their permissions must not conflict.

Invalid:

```text
Step A changes Account.balance.
Step B changes Account.balance.
Step A and Step B run at the same time.
```

Diagnostic:

```text
This is not safe.

Step A and Step B both change Account.balance at the same time.
Only one step may change Account.balance at a time.

Choose one:
  run Step A before Step B
  run Step B before Step A
  combine the changes into one step
  protect the account with a synchronization rule
```

## Persistent-state safety

For database or durable entities, `Change<T>` may lower to:

```text
transactional write lock
optimistic concurrency version check
actor mailbox ownership
linear write token
event-sourced command permission
```

The safety rule remains the same:

```text
one changer at a time
```

## Performance model

To be performant, EIGL should avoid mandatory:

```text
garbage collection
dynamic dispatch
runtime reflection
implicit boxing
implicit exceptions
stringly typed runtime lookup
runtime type guessing
```

The compiler should support:

```text
static dispatch
monomorphization
stack allocation
region allocation
move semantics
copy elision
inlining
SIMD/vectorization
partial-order scheduling
native code
Wasm
accelerator lowering
```

## Cost view

Humans should not have to write low-level performance annotations, but experts should be able to inspect them.

Example:

```text
cost view for BuildInvoice:

allocation:
  Invoice is created as a value.
  Invoice lines are created from order items.
  The number of invoice lines equals the number of order items.

dispatch:
  static dispatch

memory:
  order is read, not copied
  invoice is returned to caller

parallelism:
  line calculations may run in parallel if tax calculation has no external effects
```

## Safety theorem target

Safe EIGL should aim to prove:

```text
A well-typed, well-permissioned, well-effect-checked Safe EIGL program cannot:

  use a value after it has been moved or consumed
  read or write through a dangling reference
  double-free a resource
  mutate a value while it is being read by another live permission
  create a data race
  access a missing value as if it existed
  call an external capability it was not granted
  leak a Secret value through an unauthorized path
  ignore a declared failure in strict mode
  break a declared invariant except inside an audited trusted capsule
```

This is a design target. A real implementation would need a formal core calculus, checker proofs, compiler tests, and backend validation before making strong claims.
