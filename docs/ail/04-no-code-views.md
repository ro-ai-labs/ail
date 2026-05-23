# AIL-Flow No-Code Views

## Purpose

AIL-Flow is the no-code projection of AIL-Core. It lets humans inspect and edit
applications, tools, system components, rules, permissions, capabilities,
failures, traces, and low-level obligations without reading compiler syntax.

## View Types

Initial view types are:

- Application Map
- Action Cards
- Data Tables
- Rule Lists
- Permission Views
- Failure Maps
- Trace Views
- Tool Capability Views
- System Component Views
- Lowering Views
- Diagnostic Views
- Route Maps
- Form Blocks
- Component Cards
- C Interop Blocks
- Backend Manifest Views

## Application Map

The application map shows actors, things, actions, events, external systems,
tools, system components, and views. It is a deterministic projection of
AIL-Core and must render the same graph the same way apart from layout
preferences.

## Action Cards

Action Cards show one action at a time: trigger, inputs, requirements, reads,
writes, calls, failures, approvals, guarantees, and traces.

Editing an action card creates a graph patch. The patch is checked before it is
accepted.

## Block Model

AIL-Flow uses deterministic blocks and sockets:

- Application Map nodes: actors, things, actions, external systems, tools,
  views, system components, and packages
- Action Card blocks: trigger, inputs, requirements, reads, writes, calls,
  failures, approvals, guarantees, and traces
- Rule blocks: condition, scope, source, dependent actions
- Failure blocks: trigger, compensation, response, trace
- Permission blocks: actor, capability, resource, effect, audience
- System blocks: resource, owner, borrow, region, layout, allocation, context,
  effect
- Compiler blocks: pass input, pass output, graph pattern, rewrite, diagnostic
- UI blocks: route, form, component, event, state, accessibility
- C interop blocks: function, pointer, ownership, errno mapping, callback,
  symbol, ABI layout

Sockets declare accepted node and edge kinds. A connector between sockets is a
candidate graph edge and must satisfy the schema catalog.

## Data Tables

Data tables show things, fields, types, secrecy, ownership, persistence, and
visibility. They can propose field additions, type changes, visibility changes,
and migrations as graph patches.

## Rule Lists

Rule lists show preconditions, invariants, permission rules, approval rules,
and guarantee rules. Rules must show their provenance and the actions or tools
that depend on them.

## Permission Views

Permission Views show who or what may read, write, call, approve, disclose, or
own a resource. Secret flows and capability boundaries must be visible.

## System Component Views

System Component Views show low-level components, resources, required
capabilities, effects, guarantees, and traces. They make device or OS access
visible before lowering to target code.

## Failure Maps

Failure maps show named failures, triggering conditions, compensation, user
messages, retries, audit events, and affected guarantees.

## Trace Views

Trace Views show runtime execution in semantic terms: action entry, rule checks,
reads, writes, calls, branches, failures, approvals, guarantees, and low-level
obligations.

## Editing Through Views

No-code views do not perform opaque text edits. They produce graph patches
against AIL-Core. Each patch declares the nodes, edges, attributes, and
provenance it adds, changes, or removes.

Every AIL-Flow projection includes top-level `package` and `coreHash` fields.
Visual editors and AI agents copy those values into patch `package` and
`base_hash` so an edit cannot target a different package or overwrite a newer
checked Core graph. Node-backed Flow objects such as things, fields, actions,
tools, compiler passes, and system components also include `coreLabel`, which
is the exact checked Core node label to use in patch `source`, `target`, or
`target` references. Flow objects that expose graph relationships keep their
human-readable arrays such as `requires`, `writes`, `effects`, and `traces`,
and also include `edgeRefs` for the same checked graph edges. Each edge
reference names the patch `kind`, `source`, `target`, display `targetName`, and
current edge `attributes`, so a visual editor can construct `remove_edge` or
`replace_edge_attributes` operations without guessing target kinds.

Patch payload:

```json
{
  "schema": "ail-core.patch.v0",
  "package": "support-ticket",
  "base_hash": "ail-core:fnv64:...",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {
      "op": "add_edge",
      "kind": "requires",
      "source": "Action:CloseTicket",
      "target": "Rule:TicketNotClosed",
      "provenance": ["flow:ActionCard:CloseTicket"]
    },
    {
      "op": "replace_node_attributes",
      "target": "Action:CloseTicket",
      "attributes": {
        "label": "Resolve ticket"
      },
      "provenance": ["flow:ActionCard:CloseTicket.label"]
    }
  ]
}
```

The stage-0 CLI accepts this JSON patch subset with:

```bash
ail ail-patch --core-file checked.ail-core.txt edit.ail-core.patch.json
```

It currently supports `add_node`, `remove_node`, `add_edge`, `remove_edge`,
`replace_edge_attributes`, `replace_node_attributes`, and
`declare_provenance` operations. The patch must include `base_hash` for the
canonical checked AIL-Core artifact; stale patches are rejected before any
operation runs. When the patch includes `package`, it must match the checked
Core package name before any operation runs. The CLI then runs the AIL-Core
checker before printing the patched Core artifact. Node removals reject nodes
with incident edges, so visual editors remove relationships first. Edge
removals and edge attribute edits reject missing source, target, or edge
references instead of silently accepting a no-op. Attribute edits rewire
changed stable ids before checking, so existing rules, traces, failures, and
provenance stay attached to the edited node or edge. `declare_provenance`
attaches reviewed provenance to an existing node without changing semantic
attributes. The patched Core can be rendered back to AIL-Spec with
`ail-spec --core-file`.

Direct visual edits are allowed for fields, rules, trace names, form bindings,
view filters, and declared permissions when all required semantics are present.
Edits that add external calls, secret access, money movement, unsafe interop,
or system effects require agent clarification and human confirmation.

## Validation Of View Patches

View patches are checked with the same authority as structured-English patches.
The checker validates type flow, permissions, effects, failures, guarantees,
secrets, approvals, trace obligations, profile rules, and round-trip
equivalence before acceptance.

## Validation States

AIL-Flow renders validation states:

- accepted
- needs clarification
- checker rejected
- high-risk confirmation required
- expert-mode capability required
- projection drift detected

Each state links to diagnostic codes and affected graph nodes or edges.
