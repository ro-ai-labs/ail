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
| `TerminationBound` | `value` | `Function`, `Loop` | explicit stack or termination bound must be numeric |
| `TerminationMeasure` | `value` | `Function`, `Loop` | well-founded measure must decrease toward a lower bound |
| `Match` | `value` | `Step`, `Action`, `Function` | finite variants must be exhaustive |
| `Call` | `target` | `Step`, `Action`, `Function` | target must resolve |
| `Return` | `value` | `Step`, `Function`, `Action` | value type must match output |
| `Tool` | `name`, `purpose` | package root | tool needs permission, effect, trace |
| `SystemComponent` | `name` | package root | effects require capabilities |
| `Resource` | `name`, `type` | `SystemComponent` | region required for system effects |
| `Event` | `name` | package root | event payload typed |
| `Rule` | `name` | package root | rule must be required or checked |
| `View` | `name`, `view_kind` | `Application`, package root | reads require permission |
| `Route` | `name`, `path` | package root | reads require permission and trace |
| `Form` | `name` | package root | called action must resolve |
| `Confirmation` | `name` | form | destructive UI action requires confirmation |
| `Dashboard` | `name` | package root | reads require matching permission |
| `Workflow` | `name` | package root | blocked step ordering must hold |
| `Filter` | `name` | dashboard | filter must attach to dashboard |
| `Accessibility` | `name` | form | validation failure trace needs announcement |
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
| `StatusMap` | `code`, target status type | external binding | success and non-failure status mappings preserved |
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
| `requires` | action, tool, view, component, route, dashboard | rule, permission, capability | many | target must be declared |
| `reads` | action, function, tool, view, route, dashboard, pass | field, value, resource | many | read permission or ownership required |
| `writes` | action, tool, component, pass | field, value, resource, effect | many | write permission required |
| `calls` | action, function, tool, form, pass | action, call, effect, external binding | many | target action or effects declared |
| `validates` | form | rule | many | validation rule attached to form |
| `has_accessibility` | form | accessibility | many | error announcement attached to form |
| `requires_confirmation` | form | confirmation | many | destructive action confirmation attached to form |
| `filters` | dashboard | filter | many | filter attached to dashboard |
| `blocks_before` | workflow step | workflow step | many | blocked step must appear after prerequisite |
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
| `maps_status` | external binding | status map | many | non-failure status mapping preserved |
| `handles_failure` | action, tool, component | failure | many | handler side effects declared |
| `guarantees` | action, tool, component, package | guarantee | many | guarantee has check boundary |
| `requires_approval` | action, tool | approval | many | approval has rule and trace |
| `protects_secret` | action, tool, field, output | secret | many | redaction declared |
| `grants_permission` | action, approval, package | permission | many | scope declared |
| `records_trace` | action, tool, failure, loop, call, route, form, dashboard, workflow | trace | many | trace name stable |
| `has_termination_bound` | function, loop | termination bound | one or more | bound is numeric and profile-compatible |
| `has_termination_measure` | function, loop | termination measure | one or more | measure is decreasing and well-founded |
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
      "op": "remove_node",
      "target": "Rule:TemporaryReviewNote"
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
    },
    {
      "op": "declare_provenance",
      "target": "Action:CloseTicket",
      "provenance": ["flow:action-card:close-ticket.reviewed"]
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
- `add_edge`
- `remove_edge`
- `replace_edge_attributes`
- `replace_node_attributes`
- `declare_provenance`

`package` is optional for compatibility with early patch fixtures. When present,
it must be a string and must match the checked AIL-Core package name before any
operation runs.
`base_hash` is required. It uses the form `ail-core:fnv64:<hex>`, is computed
from the canonical checked AIL-Core rendering after parsing, and is exposed in
AIL-Flow as top-level `coreHash`. It is not computed from raw terminal output
bytes. The canonical rendering sorts nodes by semantic labels and emits edges
by their checked semantic ordinal, so raw graph vector storage order is not
part of the hash. The stage-0 patch applier rejects the patch before running any
operation when `base_hash` does not match the checked graph.

After the hash gate, the applier resolves node labels such as
`Action:CloseTicket` against the checked Core graph. AIL-Flow exposes these
labels as `coreLabel` on node-backed objects so editors do not need to invent
label syntax from display names. AIL-Flow objects that render graph
relationships expose checked edge references as `edgeRefs`; each entry carries
the patch `kind`, `source`, `target`, display `targetName`, and edge
`attributes` for the current graph edge. `add_node` creates a new node, rejects
an existing node with the same kind and name instead of treating the operation
as a no-op, and writes node provenance as `Provenance` nodes plus
`has_provenance` edges. `remove_node` resolves a checked node label, deletes
only detached nodes, and rejects nodes with incident edges so the patch must
remove relationships explicitly first. `add_edge` stores edge provenance as an
edge attribute and rejects an existing edge with the same kind, source, and
target instead of treating the operation as a no-op. `remove_edge` resolves the
same source and target labels, deletes the existing edge of that kind, and
rejects missing edges instead of treating them as no-ops.
`replace_edge_attributes` merges the listed string attributes into the resolved
edge, rewrites the stable edge id, and rejects missing edges instead of
treating them as no-ops.
`replace_node_attributes` merges the listed string attributes into the target
node, may replace the node `type`, rewrites the target node's stable id when
attributes change, and rewires existing edges to the updated node before the
checker runs. `declare_provenance` resolves a checked node label and attaches
one or more `Provenance` nodes through `has_provenance` edges without changing
semantic attributes. The patch applier returns the patched Core artifact only
after the resulting graph passes the AIL-Core checker, and the CLI prints only
that checked result.

Reserved target operations:

- `move_ordered_child`

A patch is accepted only when its optional `package` matches the checked Core
package, its `base_hash` matches the checked graph, every operation validates
against this schema, and the resulting graph passes the checker.

## Normalization Algorithm

1. Parse the package envelope and reject unknown schema versions with
   `AIL-SCHEMA-003`.
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
AIL-TYPE-001 field Ticket.score has unknown type 'UnknownScore'
```

## Compatibility Rules

Schema additions must go through the evolution protocol. A compatible schema
extension may add optional attributes, new node kinds mapped to existing core
concepts, or stricter diagnostics that do not change accepted behavior. An
incompatible extension changes canonical hashing, executable semantics, or
accepted graph meaning and requires a new schema version.

## Executable integer extension

`IntegerExpression` nodes and `left_operand`, `right_operand`, `expression_field`,
and `returns_field` edges extend the stage-0 catalog. Writes name typed roots
through an `expression` attribute. Action writes, calls and repeats carry a
unique contiguous `step_index`, preserving source execution order and repeated
operations. See [integer computation](34-integer-expressions.md) for the exact
shape, limits and backward compatibility with Core lacking step indices.

`Control` nodes and `IntegerComparison : Bool` nodes extend this representation
with structured blocks. Ordered `control_step` edges connect ordinary actions
to their control markers; compiler-pass actions cannot carry them. Conditional
markers have a `condition` edge, and each comparison has `comparison_left` and
`comparison_right` integer operands. These edges require valid endpoints and
explicit action step indices. See [control flow](35-control-flow.md).


Executable integer functions use `FunctionStep` and `ValueCall` nodes, owned
local `Field` nodes, and `function_step`, `integer_call_step`,
`function_parameter`, `function_local`, `function_expression`,
`function_destination`, `function_condition`, `call_target`, `call_destination`
and named `call_argument` edges. Function ordering, scope, signatures and
operand multiplicity are checked. See [integer functions](36-integer-functions.md).

`TextOperation` nodes add `text_kind=join|length|slice|find|split|join_list|replace|trim`. Ordered `text_step`
edges connect them to executable actions; `text_source`, `text_other` and
`text_destination` connect typed global fields, while `text_start` and `text_end`
connect integer-expression slice bounds. Operand multiplicity, ownership,
types and ordering are checked. Secret-access checks derive reads and writes
from these executable edges even when descriptive access edges are absent.
Replace also reads a field through `text_replacement`. Split produces `List<Text>`;
list join reads that type. Exact operation metadata and attribute-free operand
edges are checked. See [text operations](37-text-operations.md) and
[text transformations](49-text-transformations.md).

`ListOperation` nodes add `list_kind=empty|length|get|append|set|concat|slice|reverse|sort` and
`element_type=Int|Float|Text`. Ordered `list_step` edges connect them to actions.
Sort additionally requires exact `sort_order=ascending|descending`. Concat adds
`list_other`; slice adds `list_start` and `list_end` Int expression roots. See
[list transformations](50-list-transformations.md) for semantics and validation.
The `list_source` and `list_destination` edges reference global fields; `list_index`
references an integer expression; `list_value` references an integer or floating expression
or a text field. The checker validates exact attributes, operands, ownership,
types, order and supported profiles. Secret accesses derive from these operands
even when descriptive access edges are absent. See [typed lists](38-typed-lists.md).

`LiteralAssignment` nodes carry `value_type` and `literal` attributes. A
`literal_step` edge orders the assignment within its owning action, and one
`literal_destination` references a global field with the same declared type.
The checker validates the entire JSON literal, exact attributes and edges,
ownership, order, profile and semantic secret write access. See
[typed literals](39-typed-literals.md).


`FileOperation` nodes carry exactly `file_kind` and `resource`. A `file_step`
edge orders each operation under one executable action. Reads use one
`file_destination`; writes use one `file_source`. Operands must be declared
global Text fields for `read`/`write` or Bytes fields for `read_bytes`/`write_bytes`.
These edges imply semantic field access for secret checks.
The resource name is a static identifier, not a filesystem path or authority;
runtime callers provide separate read/write bindings. See
[file resources](40-file-resources.md).


`JsonConversion` nodes carry exactly `json_kind=parse|format` and `value_type`,
with no node type. One `json_step` edge orders each conversion under an executable
action. Exactly one `json_source` and `json_destination` link global fields; the
parse source and format destination are Text, while the other field matches
value_type. Supported types are Int, Float, Bool, Text, List<Int>, List<Float>,
List<Text> and Json. Operand
edges imply semantic access for secret checks. See [JSON conversion](41-json-conversion.md).

`JsonAccess` nodes carry exactly `json_access_kind=get|count|kind|keys` and `value_type`,
with no node type. One executable action owns each node through an ordered
`json_access_step`. Global operands use `json_access_source` (Json),
`json_access_destination` (result type), and `json_access_path` (Text, get only).
Count requires Int, kind inspection requires Text, and key enumeration requires
List<Text>. Only get has a path operand. See [JSON inspection](47-json-inspection.md).
Exact attributes, multiplicities, ownership,
profile and field types are checked; these operand edges imply secret accesses.
The built-in Json type accepts recursive values and is supported by literal
assignments and JSON conversions. See [recursive JSON](42-recursive-json.md).

`JsonMutation` has no node type. Set nodes carry exactly
`json_mutation_kind=set` and `value_type`; remove nodes carry only
`json_mutation_kind=remove`. One `json_mutation_step` edge binds each node to an
executable action with a numeric `step_index` and optional `operation_index`.
Global Json source/destination and Text path fields are linked by
`json_mutation_source`, `json_mutation_destination` and `json_mutation_path`.
Set also requires `json_mutation_value`, whose field type matches `value_type`;
remove forbids it. Operand edges have no attributes. Only provenance edges may
be added. Actual operands imply secret reads/writes. Application and C interop
profiles support mutation; functions and compiler passes reject it. See
[JSON mutation](43-json-mutation.md) for semantics and limits.


## Executable value functions

A `Function` with `executable_value=true` owns typed `Field` nodes through
`value_parameter` and `value_local`. Their `function_owner` and `local_name`
attributes, types and names must agree with the single owner and its declared
inputs/output. The output name does not implicitly declare a local. Conflicting
integer/value markers and markers that disagree with the recomputed source
classification are rejected.

`value_function_step` edges carry contiguous `step_index` values and uniquely
own `ValueFunctionStep` nodes. Their checked `step_kind` and opcode-specific
metadata describe operations, calls, returns, control and explicit traces.
`value_field_operand{name=role}` and `value_expression_operand{name=role}` carry
explicit operands; field types and expression leaves must match local storage.
`value_condition` binds a checked `IntegerComparison` or `FloatComparison`.
Expression operand roles can bind `IntegerExpression` or `FloatExpression`,
according to the opcode's declared type. `value_call_target` and
`value_call_argument{name=parameter}` describe typed calls. Operand multiplicity
is preserved, including when different roles reference the same field.

Action calls use `ValueFunctionCall` with an ordered `integer_call_step` edge,
sharing source order with existing integer calls. Actual arguments imply secret
reads, including integer expression leaves; destinations imply writes.
Checks reject extra attributes, wrong types, hidden global access, foreign
locals, missing/duplicate edges and orphan/shared steps. See
[value functions](44-value-functions.md) for the full execution contract.

## Floating computation

`FloatOperation` has no node type and carries exactly
`float_operation_kind=compute|int_to_float|float_to_int`. One ordered
`float_step` edge attaches it to an action. A compute node has one
`float_destination` and `float_expression`; conversion nodes have one
`float_destination` and `float_source`. Field types must match the operation.

`FloatExpression: Float` has `expression_kind=literal|field|binary|negate`.
Literal nodes store canonical numeric `value`; binary nodes store `operator`.
Field nodes use `expression_field`, binary nodes `left_operand`/`right_operand`,
and negation nodes `unary_operand`. `FloatComparison: Bool` has a `comparison`
attribute and `comparison_left`/`comparison_right` Float trees. Action controls
use `control_kind=if_float|while_float` with the existing `condition` edge.
Each expression tree node has one parent; extra metadata, dangling, shared and
orphan operands are rejected. Local trees have an exact `function_owner` and
can reference only uniquely declared local storage. Global operands require one
declaring Thing and an unambiguous runtime key. See
[floating-point computation](45-float-computation.md) for the execution contract.

## Boolean predicates

`BooleanOperation` has an ordered `boolean_step`, `boolean_destination` and
`boolean_expression`. Its `BooleanExpression:Bool` tree uses `boolean_operand`,
`boolean_left`, `boolean_right` and `boolean_field`. Typed comparisons reuse
existing numeric expression nodes or `TextOperand:Text` leaves (`value` or
`text_field`). Controls use `if_boolean|while_boolean` and existing condition
edges. Function operands retain exact local ownership and typed scope.
Saved-Core TextOperand `value` uses JSON string quoting; the in-memory graph
retains the decoded value. Aggregate validation preserves raw escape lengths.
Combined raw-byte/node/depth checks apply across embedded numeric trees;
each tree node has one appropriate parent. See
[Boolean predicates](48-boolean-predicates.md) for exact attributes and limits.

## Runtime recovery controls

Action Control nodes add `control_kind=try|recover|end_try`. Recover alone has
`recovery_errors` (`any` or sorted unique semicolon-separated data categories)
and one attribute-free `recovery_destination` edge to a genuine Text field.
Functions retain owned ValueFunctionStep control nodes and
`value_field_operand(name=key)` for their local destination. Exact metadata,
unique ordered owners and semantic write permissions are mandatory. Lowering
produces structured TRY_BEGIN/RECOVER/TRY_END scopes; ordinary jumps cannot
enter protected/handler interiors. See [runtime recovery](51-runtime-recovery.md).

## Standard-stream operations

`StandardStreamOperation` has only `stream_kind=stdin_read|stdout_write|stderr_write`
and no type annotation. Exactly one `stream_step` links it from an executable
Action, with numeric `step_index` and optional numeric `operation_index`. Reads
have one attribute-free `stream_destination`; writes have one attribute-free
`stream_source`. Operands must be genuine global Text declarations. Extra
metadata, hidden edges, duplicate/foreign ownership, local substitutes and
unsupported profiles fail with `AIL-STDIO-001`. Provenance remains permitted.
Derived accesses participate in secret-field checks independently of descriptive
access edges. See [standard streams](52-standard-streams.md).

## ProcessOperation

[Process resources](53-process-resources.md#core-and-ste) define the exact ProcessOperation attributes, six typed operand edges, unique process_step ownership and independent semantic access checks.

[Directory resources](54-directory-resources.md#core-and-ste) define DirectoryOperation with exact directory_kind/resource attributes, one ordered directory_step, a typed directory_path, and either directory_destination or directory_source. Core validates unique global ownership and derives semantic accesses from these roles.

`BytesOperation` nodes and their `bytes_step`/operand edges are described in
[Bytes Core and STE](56-bytes.md#core-and-ste). Numeric operands use owned
IntegerExpression trees; field operands use exact declared Bytes/Text/Int
types. The checker rejects ambiguous identities/provenance, unexpected roles
and edges, malformed order and incompatible profiles. Typed function steps
reuse scoped operand trees. Saved Core retains structural duplicates so that
they remain rejectable.

## Explicit raise control

`Control` nodes with `control_kind=raise` carry exactly one `error_category`
attribute and no operand edges. They retain ordinary ordered control ownership
and provenance. Functions use their existing scoped control-step schema. One of
the fifteen catchable category literals is required, including for unreachable
steps. See [explicit errors](57-explicit-errors.md).

## Executable variants

[Executable variant values](59-executable-variants.md) retain owned Type, Variant
and Field declarations and instantiate a closed concrete schema for execution.
Construction, tag tests and extraction carry typed operand edges and ordered
action/function ownership. Named constructor arguments have individual named,
typed operand edges; no packed arguments string bypasses Core validation.
Reachable type declarations reject duplicate identities, missing ownership,
foreign fields and inconsistent provenance before map projection.
