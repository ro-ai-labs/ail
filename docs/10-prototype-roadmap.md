# Prototype Roadmap

This roadmap turns the specification into a practical implementation sequence.

## Guiding principle

Build the explicit layer first.

Do not begin with unrestricted natural language.

```text
RIF first.
EIG-Core second.
Checkers third.
Views fourth.
Execution fifth.
RSL sixth.
Compiler generation seventh.
Self-hosting eighth.
```

## Phase 1: EIG-Core schema

Implement the graph schema.

Deliverables:

```text
core_model.py or core_model.rs
Program
Module
Node
Edge
Type
Operation
Port
Region
Permission
Effect
Contract
FailureCase
View
JSON serialization
stable IDs
```

Acceptance criteria:

```text
Can construct a graph for ConfirmOrder.
Can serialize and deserialize the graph.
Can validate edge source/target existence.
Can produce a stable semantic hash for graph nodes.
```

## Phase 2: RIF parser

Implement a restricted RIF parser.

Deliverables:

```text
rif_model
rif_parser
examples/confirm_order.rif.md
examples/password_reset.rif.md
examples/bad_parallel_update.rif.md
```

Start with section-based parsing. Unknown lines may be preserved as raw semantic text.

Acceptance criteria:

```text
Can parse intent name.
Can parse subject.
Can parse requires.
Can parse steps.
Can parse call/output/changes/may fail/compensation.
Can parse failure behavior.
Can parse guarantees.
```

## Phase 3: RIF to EIG-Core builder

Implement graph construction from RIF.

Deliverables:

```text
graph_builder
reference resolver
provenance records
```

Acceptance criteria:

```text
Every RIF intent becomes an operation/process node.
Every step becomes an operation invocation node.
Every call becomes a calls edge.
Every changes line becomes a permission/effect node or edge.
Every failure behavior becomes a failure edge.
Every guarantee becomes a contract node.
```

## Phase 4: Basic checker

Implement initial safety checks.

Checks:

```text
reference resolution
type existence
field existence
state existence
known operation calls
failure completeness
permission conflicts
effect authorization
secret flow basics
guarantee well-formedness
compensation path availability
```

Acceptance criteria:

```text
confirm_order.rif.md passes.
bad_parallel_update.rif.md fails with a clear diagnostic.
password_reset.rif.md passes and detects secret handling constraints.
```

## Phase 5: Views and explanations

Generate human-readable views from RIF/EIG-Core.

Deliverables:

```text
explanations.py
views.py
flow view
failure view
permission view
effect view
security view
cost placeholder view
```

Acceptance criteria:

```text
Can explain ConfirmOrder in plain English.
Can render plain-text flow view.
Can render plain-text failure view.
Can render plain-text permission view.
Can render JSON view model for future diagrams.
```

## Phase 6: Simple interpreter

Implement a simple EIG process interpreter or simulator.

Deliverables:

```text
interpreter.py
mock resources
mock operation outcomes
trace generation
```

Acceptance criteria:

```text
Can simulate successful ConfirmOrder.
Can simulate payment failure and show compensation.
Can simulate shipping failure and show final retry state.
Can produce trace view.
```

## Phase 7: Bytecode or Wasm lowering

Choose a minimal executable backend.

Option A: custom bytecode interpreter.

```text
CHECK_REQUIRES
CALL_EFFECT
STORE_OUTPUT
REGISTER_COMPENSATION
HANDLE_FAILURE
SET_FIELD
ASSERT_GUARANTEE
RETURN
```

Option B: Wasm lowering for pure/simple effect-free operations first.

Acceptance criteria:

```text
Can lower a simple pure action.
Can lower a simple sequential process to bytecode.
Can execute bytecode and produce a trace.
```

## Phase 8: RSL subset

Implement compact RSL only after RIF works.

Deliverables:

```text
phrase dictionary
phrase pattern matcher
RSL to RIF elaborator
ambiguity diagnostics
```

Initial syntax:

```text
Confirm order:
  Draft -> Confirmed
  by reserve inventory, capture payment, create shipment
  on shipping failure -> PaidAwaitingFulfillmentRetry
```

Acceptance criteria:

```text
Can elaborate compact RSL into RIF.
Can report unknown phrase.
Can report ambiguous phrase.
Can record provenance.
```

## Phase 9: EIG-Meta subset

Implement compiler-generation metadata.

Deliverables:

```text
phrase pattern definitions
checking rule definitions
diagnostic templates
view templates
```

Acceptance criteria:

```text
Can define a phrase pattern in EIG-Meta.
Can generate a phrase matcher from it.
Can define a diagnostic template and use it in checker output.
```

## Phase 10: Self-hosting preparation

Start writing compiler passes in RIF.

Candidate passes:

```text
BuildCoreGraphFromRIF
CheckReferences
InferPermissions
CheckEffects
CheckFailures
RenderExplanation
RenderFlowView
LowerProcessToBytecode
```

Acceptance criteria:

```text
At least one checker pass is specified in RIF.
Seed compiler can parse it.
Seed compiler can build EIG-Core for it.
```

## Suggested first examples

### Confirm order

Demonstrates:

```text
state transition
external calls
failure handling
compensation
permissions
effects
visual flow
```

### Password reset

Demonstrates:

```text
secrets
one-time token
state mutation
session revocation
guarantees
security view
```

### Bad parallel update

Demonstrates:

```text
permission conflict
diagnostic quality
human-friendly borrow-checker-like error
```

## Minimal CLI

```text
eigl parse examples/confirm_order.rif.md
eigl graph examples/confirm_order.rif.md --json
eigl check examples/confirm_order.rif.md
eigl explain examples/confirm_order.rif.md
eigl view examples/confirm_order.rif.md --flow
eigl view examples/confirm_order.rif.md --failure
eigl view examples/confirm_order.rif.md --permissions
eigl simulate examples/confirm_order.rif.md --scenario success
eigl simulate examples/confirm_order.rif.md --scenario payment_failure
```

## Non-goals for first prototype

Do not implement yet:

```text
full arbitrary English parsing
full native codegen
advanced theorem proving
full visual editor
distributed runtime
LLM integration
self-hosting
formal proof of safety
```

## First prototype success definition

The prototype is successful when it proves this loop:

```text
compact or explicit intent
  -> readable normalized representation
  -> semantic graph
  -> safety checks
  -> explanation
  -> visualization
  -> execution trace
```

Even without native code generation, this validates the language architecture.
