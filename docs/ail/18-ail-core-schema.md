# AIL-Core Schema Reference

## Purpose

This document is the machine-checkable contract for AIL-Core. It defines the
canonical graph shape, node catalog, edge catalog, attributes, graph patch
format, normalization algorithm, and stable hash algorithm used by the checker,
renderers, round-trip tests, prompt harness, bytecode compiler, and native
backends.

## Canonical Package Envelope

Authority: Target schema.

The target schema envelope for serialized AIL-Core packages is:

```json
{
  "schema": "ail-core.schema.v0",
  "package": {
    "name": "support-ticket",
    "version": "0.1.0",
    "profile": "Application",
    "imports": []
  },
  "graph": {
    "nodes": [],
    "edges": []
  },
  "conformance": {
    "level": "first-slice",
    "features": []
  }
}
```

All keys are sorted lexicographically during canonical serialization. Arrays
are sorted by the stable ordering rules below unless their schema says order is
semantic, such as `Step` sequence or branch outcomes.

## Stage-0 Text Artifact

Authority: Normative for the bootstrap implementation.

The current toolchain serializes checked AIL-Core as deterministic
line-oriented text with package metadata, sorted `nodes:`, and sorted `edges:`.
That text artifact is the accepted compiler and review boundary for
`ail-core`, `ail-spec --core-file`, `ail-lower --core-file`,
`ail-compile --core-file`, and `ail-build --core-file`.

The line-oriented artifact must preserve the same package metadata, node
catalog, edge catalog, attributes, and provenance described by this schema.
Promoting the JSON envelope to the default serialized artifact requires a
versioned migration note and equivalent render/reparse conformance fixtures.

## Node Schema

Every node has:

```json
{
  "id": "node-kind:package/module/name",
  "kind": "Action",
  "name": "CloseTicket",
  "attributes": {},
  "provenance": []
}
```

Required attributes for all nodes:

- `id`: stable ID scoped by package, module, profile, and declaration
- `kind`: one accepted node kind
- `name`: human-readable canonical name
- `provenance`: at least one provenance entry

Optional attributes for all nodes:

- `label`
- `profile`
- `type`
- `visibility`
- `effect_class`
- `ordering`
- `diagnostic_code`
- `conformance_level`
- `package_version`

## Node Catalog

| Node kind | Required attributes | Allowed parents | Checker rule |
| --- | --- | --- | --- |
| `Application` | `name`, `profile` | package root | one per Application package |
| `User` | `name` | `Application`, package root | user role name stable |
| `Thing` | `name` | `Application`, package root | fields use `has_field` |
| `Field` | `name`, `type` | `Thing` | type must resolve |
| `Action` | `name` | `Application`, package root | executable action needs trace |
| `Function` | `name` | package root, module | inputs and outputs must be typed |
| `Input` | `name`, `type` | `Action`, `Function`, `Tool` | secret flag follows type |
| `Output` | `name`, `type` | `Action`, `Function`, `Tool` | secret output requires reveal policy |
| `Step` | `name`, `ordering` | `Action`, `Function`, `CompilerPass` | order is semantic |
| `Branch` | `condition`, `ordering` | `Step`, `Action`, `Function` | must have exhaustive outcomes |
| `Loop` | `loop_kind`, `termination_policy` | `Step`, `Action`, `Function` | policy checked by profile |
| `Match` | `value` | `Step`, `Action`, `Function` | finite variants must be exhaustive |
| `Call` | `target` | `Step`, `Action`, `Function` | target must resolve |
| `Return` | `value` | `Step`, `Function`, `Action` | value type must match output |
| `Tool` | `name`, `purpose` | package root | tool needs permission, effect, trace |
| `SystemComponent` | `name` | package root | effects require capabilities |
| `Resource` | `name`, `type` | `SystemComponent` | region required for system effects |
| `Event` | `name` | package root | event payload typed |
| `Rule` | `name` | package root | rule must be required or checked |
| `View` | `name`, `view_kind` | `Application`, package root | reads require permission |
| `Value` | `name`, `type` | package root, action, function | type must resolve |
| `Capability` | `name` | package root | used by effects or bindings |
| `Permission` | `name` | package root | grants scope and subject |
| `Effect` | `name`, `effect_class` | action, tool, component | target and capability required |
| `Failure` | `name` | package root | handler and trace required |
| `Guarantee` | `name` | package root | attached to action/tool/component |
| `Secret` | `name`, `redaction` | package root | protected by field/input/output |
| `Approval` | `name`, `threshold` | action, tool | rule and trace required |
| `Trace` | `name` | package root | recorded by action/failure/tool |
| `Region` | `name` | system component | resources must enter region |
| `Layout` | `repr`, `alignment` | system component | linked to resource |
| `Allocation` | `placement` | system component | linked to resource |
| `ExecutionContext` | `name`, `context_kind` | system component | effects checked by context |
| `InterruptPriority` | `priority` | system component | targets context |
| `InterruptMask` | `mask` | system component | targets context |
| `SchedulerTask` | `name`, `task_kind` | system component | runs in context |
| `SchedulerTaskPriority` | `priority` | system component | targets task |
| `SchedulerTaskTiming` | `deadline`, `budget` | system component | targets task |
| `LockGuard` | `resource`, `lock` | system component | targets protected resource and lock |
| `Lowering` | `target` | package root | backend must support target |
| `Diagnostic` | `code`, `severity` | checker package | code must be stable |
| `Package` | `name`, `version` | package root | import compatibility checked |
| `ExternalBinding` | `binding_kind`, `target` | package root | sandbox or ABI contract required |
| `Prompt` | `name`, `version` | agent package | output schema required |
| `CorpusFixture` | `fixture_kind`, `expected_result` | conformance package | expected diagnostics declared |

## Edge Schema

Every edge has:

```json
{
  "id": "edge:package/source/kind/target",
  "kind": "reads",
  "source": "Action:support-ticket/CloseTicket",
  "target": "Field:support-ticket/Ticket.status",
  "attributes": {},
  "provenance": []
}
```

Required attributes for all edges:

- `id`
- `kind`
- `source`
- `target`
- `provenance`

## Edge Catalog

| Edge kind | Source kinds | Target kinds | Cardinality | Checker rule |
| --- | --- | --- | --- | --- |
| `contains` | package, application, action, function | declaration, step | many | target belongs to source |
| `has_field` | `Thing` | `Field` | many | field name unique per thing |
| `requires` | action, tool, view, component | rule, permission, capability | many | target must be declared |
| `reads` | action, function, tool, view, pass | field, value, resource | many | read permission or ownership required |
| `writes` | action, tool, component, pass | field, value, resource, effect | many | write permission required |
| `calls` | action, function, tool, pass | call, effect, external binding | many | target effects declared |
| `performs` | component, action, tool | effect | many | capability and target required |
| `uses_resource` | component | resource | many | resource declared |
| `targets_resource` | effect | resource | one | resource declared |
| `authorizes_resource` | capability | resource | many | capability grants effect target |
| `owns_resource` | component | resource | many | one owner at a time |
| `borrows_resource` | component | resource | many | shared reads only |
| `mutably_borrows_resource` | component | resource | zero or one per resource | no shared borrow conflict |
| `uses_region` | component | region | many | region declared |
| `in_region` | resource | region | one | required for system resource effects |
| `uses_layout` | component | layout | many | linked to resource |
| `layouts_resource` | layout | resource | one | `repr` and alignment valid |
| `uses_allocation` | component | allocation | many | linked to resource |
| `allocates_resource` | allocation | resource | one | placement valid for target |
| `uses_lock_guard` | component | lock guard | many | protected resource and lock declared |
| `guards_resource` | lock guard | resource | one | guarded resource declared |
| `uses_lock_resource` | lock guard | resource | one | lock resource declared |
| `runs_in_context` | action, component, task | execution context | one | effects legal in context |
| `prioritizes_context` | interrupt priority | execution context | one | context declared |
| `masks_context` | interrupt mask | execution context | one | context declared |
| `schedules_task` | component | scheduler task | many | context declared |
| `task_runs_in_context` | scheduler task | execution context | one | context declared |
| `uses_task_priority` | component | scheduler task priority | many | task declared |
| `prioritizes_task` | scheduler task priority | scheduler task | one | task declared |
| `uses_task_timing` | component | scheduler task timing | many | task declared |
| `times_task` | scheduler task timing | scheduler task | one | task declared |
| `emits` | action, function, loop, failure | event | many | event payload typed |
| `may_fail_with` | action, tool, external binding | failure | many | handler required when blocking |
| `handles_failure` | action, tool, component | failure | many | handler side effects declared |
| `guarantees` | action, tool, component, package | guarantee | many | guarantee has check boundary |
| `requires_approval` | action, tool | approval | many | approval has rule and trace |
| `protects_secret` | action, tool, field, output | secret | many | redaction declared |
| `grants_permission` | action, approval, package | permission | many | scope declared |
| `records_trace` | action, tool, failure, loop, call | trace | many | trace name stable |
| `has_provenance` | any node or edge | provenance | one or more | human vs agent source tagged |
| `projects_to` | core node or edge | flow/spec item | many | projection reversible or lossy |
| `lowers_to` | core node or edge | bytecode/backend item | many | backend report required |
| `depends_on` | package, action, function | package, action, function | many | version constraints valid |

## Graph Patch Schema

AIL-Flow, AIL-Agent, and canonical spec edits use the same graph patch schema:

```json
{
  "schema": "ail-core.patch.v0",
  "package": "support-ticket",
  "base_hash": "ail-core:fnv64:3f2c...",
  "ops": [
    {
      "op": "add_node",
      "kind": "Rule",
      "name": "TicketNotClosed",
      "provenance": ["flow:action-card:close-ticket"]
    },
    {
      "op": "add_edge",
      "kind": "requires",
      "source": "Action:CloseTicket",
      "target": "Rule:TicketNotClosed",
      "provenance": ["flow:action-card:close-ticket"]
    },
    {
      "op": "remove_edge",
      "kind": "has_provenance",
      "source": "Action:CloseTicket",
      "target": "Provenance:flow:action-card:close-ticket.transient-note"
    },
    {
      "op": "replace_edge_attributes",
      "kind": "requires",
      "source": "Action:CloseTicket",
      "target": "Rule:TicketNotClosed",
      "attributes": {
        "provenance": "flow:action-card:close-ticket.reviewed"
      }
    },
    {
      "op": "replace_node_attributes",
      "target": "Action:CloseTicket",
      "attributes": {
        "label": "Resolve ticket"
      },
      "provenance": ["flow:action-card:close-ticket.label"]
    }
  ],
  "review": {
    "author": "human",
    "confirmed_semantics": true
  }
}
```

Patch operations:

- `add_node`
- `add_edge`
- `remove_edge`
- `replace_edge_attributes`
- `replace_node_attributes`

`base_hash` is required. It uses the form `ail-core:fnv64:<hex>`, is computed
from the canonical checked AIL-Core rendering after parsing, and is exposed in
AIL-Flow as top-level `coreHash`. It is not computed from raw terminal output
bytes. The stage-0 patch applier rejects the patch before running any
operation when `base_hash` does not match the checked graph.

After the hash gate, the applier resolves node labels such as
`Action:CloseTicket` against the checked Core graph. AIL-Flow exposes these
labels as `coreLabel` on node-backed objects so editors do not need to invent
label syntax from display names. AIL-Flow objects that render graph
relationships expose checked edge references as `edgeRefs`; each entry carries
the patch `kind`, `source`, `target`, display `targetName`, and edge
`attributes` for the current graph edge. `add_node` writes node provenance as
`Provenance` nodes plus
`has_provenance` edges. `add_edge` stores edge provenance as an edge attribute.
`remove_edge` resolves the same source and target labels, deletes the existing
edge of that kind, and rejects missing edges instead of treating them as
no-ops. `replace_edge_attributes` merges the listed string attributes into the
resolved edge, rewrites the stable edge id, and rejects missing edges instead
of treating them as no-ops.
`replace_node_attributes` merges the listed string attributes into the target
node, may replace the node `type`, rewrites the target node's stable id when
attributes change, and rewires existing edges to the updated node before the
checker runs. The CLI prints the patched Core artifact only after the resulting
graph passes the AIL-Core checker.

Reserved target operations:

- `remove_node`
- `move_ordered_child`
- `declare_provenance`

A patch is accepted only when its `base_hash` matches the checked graph, every
operation validates against this schema, and the resulting graph passes the
checker.

## Normalization Algorithm

1. Parse the package envelope and reject unknown schema versions.
2. Resolve imports and package aliases to canonical package IDs.
3. Normalize type aliases through the package dependency graph.
4. Expand profile defaults into explicit nodes and edges with package-default
   provenance.
5. Normalize names to stable IDs without changing human labels.
6. Sort unordered nodes by `(kind, package, module, name, id)`.
7. Sort unordered edges by `(kind, source, target, id)`.
8. Preserve semantic order for `Step`, branch outcomes, function parameters,
   tuple fields, and trace sequence declarations.
9. Canonicalize attribute keys and scalar values.
10. Remove projection-only layout metadata.
11. Emit canonical serialization.

## Stable Hash Algorithm

The first canonical hash is:

```text
canonical_text = canonical_json_without_hash(package)
hash = fnv64(canonical_text as UTF-8)
semantic_hash = "ail-core:fnv64:" + lower_hex_16(hash)
```

The hash excludes timestamps, UI coordinates, friendly explanations, diagnostic
rendering text, and non-semantic layout hints. It includes package identity,
imports, all normalized nodes, all normalized edges, checker-relevant
attributes, and provenance category.

## Stable Examples

Application:

```text
Package support-ticket profile Application
Node Application SupportTickets
Node Thing Ticket
Node Field Ticket.status : State<New, Open, Closed>
Node Action CloseTicket
Edge CloseTicket reads Ticket.status
Edge CloseTicket writes Ticket.status
Edge CloseTicket records_trace TicketClosed
```

AgentTool:

```text
Package refund-tool profile AgentTool
Node Tool RefundCustomerPayment
Node Input payment_token : Secret<Text>
Node Effect PaymentProvider.refund effect_class=network
Node Permission requester may create refunds
Edge RefundCustomerPayment requires Permission:requester may create refunds
Edge RefundCustomerPayment calls Effect:PaymentProvider.refund
Edge RefundCustomerPayment protects_secret Secret:PaymentToken
```

Compiler:

```text
Package ail-meta-permissions profile Compiler
Node Action InferReadPermissions kind=CompilerPass
Node Value input_graph : AIL-Core
Node Value output_graph : AIL-Core
Node Diagnostic AIL-PERMISSION-READ-MISSING
Edge InferReadPermissions reads input_graph
Edge InferReadPermissions writes output_graph
```

System:

```text
Package network-driver profile System
Node SystemComponent NetworkPacketReceiver
Node Resource rx_buffer : Buffer
Node Capability access network device
Node Effect read network device effect_class=device
Edge NetworkPacketReceiver owns_resource rx_buffer
Edge NetworkPacketReceiver requires Capability:access network device
Edge NetworkPacketReceiver performs Effect:read network device
```

## Invalid Graph Examples

Missing trace:

```text
Node Action CloseTicket
Edge CloseTicket writes Ticket.status
```

Expected diagnostic:

```text
AIL-TRACE-001 action CloseTicket is missing trace coverage
```

Secret read without protection:

```text
Node Action EmailTicket
Node Field Ticket.internal_notes : Secret<List<Text>>
Edge EmailTicket reads Ticket.internal_notes
```

Expected diagnostic:

```text
AIL-SECRET-READ-001 secret read requires explicit protection and permission
```

Unknown target type:

```text
Node Field Ticket.score : UnknownScore
```

Expected diagnostic:

```text
AIL-TYPE-001 type UnknownScore is not declared or imported
```

## Compatibility Rules

Schema additions must go through the evolution protocol. A compatible schema
extension may add optional attributes, new node kinds mapped to existing core
concepts, or stricter diagnostics that do not change accepted behavior. An
incompatible extension changes canonical hashing, executable semantics, or
accepted graph meaning and requires a new schema version.
