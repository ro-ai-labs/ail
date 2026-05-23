# AIL-Core Schema Reference

## Purpose

This document is the machine-checkable contract for AIL-Core. It defines the
canonical graph shape, node catalog, edge catalog, attributes, graph patch
format, normalization algorithm, and stable hash algorithm used by the checker,
renderers, round-trip tests, prompt harness, bytecode compiler, and native
backends.

## Canonical Package Envelope

Every serialized AIL-Core package uses this envelope:

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
| `runs_in_context` | action, component, task | execution context | one | effects legal in context |
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
      "node": {
        "id": "Rule:support-ticket/TicketNotClosed",
        "kind": "Rule",
        "name": "TicketNotClosed",
        "provenance": ["flow:action-card:close-ticket"]
      }
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
- `remove_node`
- `replace_node_attributes`
- `add_edge`
- `remove_edge`
- `replace_edge_attributes`
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
