# EIG-Core: Semantic Graph Specification

EIG-Core is the canonical program representation.

RSL and RIF are textual projections. Visual diagrams are projections. EIG-Core is the semantic source of truth.

## Definition

An EIG-Core program is a typed, attributed, directed semantic graph.

```text
Program = {
  modules,
  types,
  operations,
  graph,
  contracts,
  views,
  metadata
}
```

More formally:

```text
Program P = (Modules, Types, Nodes, Edges, Contracts, Views, Metadata)
```

## Core primitives

EIG-Core v0.1 should use a small set of primitives:

```text
Type
Value
Operation
Region
Edge
Permission
Effect
Contract
View
```

Everything else is a specialization.

Mapping from human concepts:

```text
Thing   -> Type
Rule    -> Contract
Action  -> Operation with effect/process region
Step    -> Operation invocation
State   -> Type + state-machine region
Event   -> Type + emission effect
View    -> View query/projection
```

## Node

```text
Node {
  id: StableId
  kind: NodeKind
  name: Symbol
  type: TypeRef?
  ports: Port[]
  attributes: Map
  regions: Region[]
  contracts: ContractRef[]
  metadata: Metadata
}
```

## Port

```text
Port {
  name: Symbol
  direction: in | out | inout
  type: TypeRef
  permission: PermissionKind?
  multiplicity: one | optional | many
  effect: EffectRef?
}
```

## Edge

```text
Edge {
  id: StableId
  kind: EdgeKind
  from: PortRef | NodeRef
  to: PortRef | NodeRef
  guard: PredicateRef?
  attributes: Map
  metadata: Metadata
}
```

## Region

```text
Region {
  kind: pure | effect | process | state_machine | failure | proof | view
  nodes: NodeRef[]
  edges: EdgeRef[]
  schedule: sequential | parallel | partial_order | unordered
}
```

## Stable identity

Graph nodes should have stable IDs. IDs may be based on normalized semantic content where possible.

Human-readable names are aliases. Semantic identity belongs to the graph.

This enables:

```text
semantic diffs
content-addressed definitions
cache reuse
proof reuse
safe renaming
stable visualization
LLM graph patches
```

## Node kinds

Initial node kinds:

```text
module
thing/type
field
state
state_set
event
operation
operation_invocation
intent
step
value
failure
guarantee
contract
permission
effect
capability
view
trusted_capsule
```

A smaller implementation may model these as specializations of the core primitives.

## Edge kinds

Initial edge kinds:

```text
contains
has_field
has_state
has_type
input
output
data
order
calls
reads
changes
owns
consumes
produces
requires
ensures
may_fail
handles_failure
compensates
emits
transitions_to
refines
traces_to
renders
```

## Type model

Core types:

```text
Bool
Int<bits>
Float<bits>
Decimal<precision, scale>
Text
Bytes
Time
Duration
Money<Currency>
Id<Thing>
Ref<Thing>
Record
Variant
List<T>
Array<T, N>
Set<T>
Map<K, V>
Option<T>
Result<T, Error>
Secret<T>
Capability<E>
Tensor<T, Shape>
```

No implicit null.

Missing values use:

```text
Option<T> = Some<T> | None
```

Fallible operations use:

```text
Result<T, E> = Success<T> | Failure<E>
```

## Operation

```text
Operation {
  id: StableId
  name: Symbol
  inputs: Port[]
  outputs: Port[]
  permissions: PermissionRequirement[]
  effects: Effect[]
  preconditions: Contract[]
  body: Region
  failure_cases: FailureCase[]
  postconditions: Contract[]
  cost_model: CostModel
  metadata: Metadata
}
```

## Permission

```text
PermissionKind =
  Own<T>
  Read<T>
  Change<T>
  Move<T>
  Consume<T>
  Share<T>
  Secret<T>
  Capability<E>
```

## Effect

```text
Effect =
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

## Contract

Contracts are graph nodes, not comments.

```text
Contract {
  kind: precondition | postcondition | invariant | temporal | refinement | proof_obligation
  expression: Expression
  scope: NodeRef[]
  diagnostic: DiagnosticTemplate?
  metadata: Metadata
}
```

Contracts can compile to:

```text
static proof obligations
runtime assertions
property tests
database constraints
monitoring rules
audit checks
```

## Failure case

```text
FailureCase {
  source: OperationInvocationRef
  failure: FailureType
  handler: Region
  result: returned | retried | compensated | stopped | marked_impossible
  contracts: Contract[]
}
```

## EIG-Core graph example

For ConfirmOrder:

```text
nodes:
  intent ConfirmOrder
  input order: Order
  step ReserveInventory
  step CapturePayment
  step CreateShipmentRequest
  step MarkOrderConfirmed
  failure PaymentFailed
  failure ShipmentFailed
  guarantee OrderConfirmed

edges:
  ConfirmOrder contains ReserveInventory
  order.items data -> ReserveInventory.items
  ReserveInventory produces reservation
  reservation data -> CreateShipmentRequest.reservation
  order.payment_method data -> CapturePayment.method
  order.total data -> CapturePayment.amount
  CapturePayment may_fail PaymentFailed
  PaymentFailed handles_failure -> ReleaseReservation
  CreateShipmentRequest may_fail ShipmentFailed
  ShipmentFailed handles_failure -> SetPaidAwaitingFulfillmentRetry
  MarkOrderConfirmed ensures OrderConfirmed
```

## JSON-like serialization

```json
{
  "eig_version": "0.1",
  "module": "commerce.order",
  "nodes": [
    {
      "id": "intent:ConfirmOrder",
      "kind": "intent",
      "name": "ConfirmOrder"
    },
    {
      "id": "step:ReserveInventory",
      "kind": "operation_invocation",
      "name": "Reserve inventory",
      "call": "Inventory.reserve"
    }
  ],
  "edges": [
    {
      "kind": "contains",
      "from": "intent:ConfirmOrder",
      "to": "step:ReserveInventory"
    }
  ]
}
```

## Graph validity rules

A valid EIG-Core graph must satisfy:

```text
every edge source exists
every edge target exists
every port type exists
every operation invocation resolves to an operation
every data edge type-checks
every required permission is satisfiable
every effect is authorized by a capability or policy
every declared failure is handled, returned, retried, compensated, or proven impossible
every guarantee is well-formed
every trusted capsule is explicit and audited
```

## Graph patches

LLMs should edit EIG-Core/RIF through patches, not arbitrary rewrites.

Example:

```json
{
  "patch": "AddShippingFailurePolicy",
  "target": "intent:ConfirmOrder",
  "operations": [
    {
      "op": "add_failure_handler",
      "source": "step:CreateShipmentRequest",
      "failure": "ShipmentFailed",
      "handler": [
        "set order.status = PaidAwaitingFulfillmentRetry",
        "stop with FulfillmentRetryNeeded"
      ]
    }
  ]
}
```

Patch validation rules:

```text
target exists
added references resolve
new graph remains well-typed
new permissions are valid
new effects are authorized
new failure behavior is complete
new contracts do not contradict existing contracts
```

## EIG-Core design rule

EIG-Core must be expressive enough for compilers, visualizers, checkers, and explanation generators to operate on the same semantic object.

No semantic behavior should exist only in a diagram, only in a comment, or only in a backend.
