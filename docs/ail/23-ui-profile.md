# AIL UI Profile

## Purpose

The UI profile defines semantics for advanced user-facing applications:
routes, navigation, views, components, forms, validation, events,
accessibility, responsive layout constraints, state, optimistic updates,
permissions, error states, and user-interaction traces.

UI declarations are not presentation-only metadata. They bind visible behavior
to AIL-Core actions, permissions, failures, guarantees, and traces.

## Routes And Navigation

Canonical form:

```text
Route: Ticket detail.

The route path is:

- /tickets/:ticket_id

The route reads:

- Ticket.id
- Ticket.status
- Ticket.public_updates

The route requires permission:

- requester may read ticket

The route records trace:

- RouteTicketDetailViewed
```

Route nodes declare:

- path template
- parameters
- required reads
- allowed actions
- permission checks
- failure views
- trace events

Current implementation status: the bootstrap parser accepts `Route:`
declarations with path, reads, permission requirements, and trace events, then
lowers them into checked AIL-Core `Route`, `Value`, `Permission`, and `Trace`
nodes with deterministic render/reparse.

The v0.2 UI slice also accepts `Form:`, `Dashboard:`, and `Workflow:`
declarations. Forms lower to `Form`, `Field`, `Rule`, `Trace`, and
`Accessibility` nodes, with `calls`, `has_field`, `validates`,
`records_trace`, and `has_accessibility` edges. Dashboards lower reads,
permissions, filters, and trace events. Workflows lower ordered `Step` nodes
and `blocks_before` constraints; the checker rejects a blocked step that
appears before or at its prerequisite.

Components, responsive layout constraints, destructive-action confirmation,
and full backend/UI permission parity beyond dashboard-read permissions remain
future UI-profile implementation slices.

## Views And Components

View nodes render state from AIL-Core declarations. Component nodes are reusable
view fragments with typed inputs, events, and accessibility obligations.

Component schema:

```text
Component: Ticket status badge.

The component needs:

- status: State<New, Open, Assigned, Closed, Overdue>

The component emits:

- no events

The component guarantees:

- the status text is available to assistive technology
```

## Forms And Validation

Forms bind input fields to actions. Validation rules are AIL `Rule` nodes.

```text
Form: Create ticket.

The form calls action:

- CreateTicket

The form fields are:

- title: Text
- initial public update: Text

The form validates:

- title is not empty

If validation fails:

- FormValidationFailed

The form accessibility is:

- title error is announced
```

## Events

UI events are typed and traceable:

- click
- submit
- change
- focus
- navigation
- keyboard shortcut
- drag
- drop
- realtime update

Each event declares the action, state transition, or patch it may trigger.

## Accessibility

The UI profile requires:

- accessible names for controls
- semantic roles for interactive elements
- focus order
- keyboard equivalent for pointer-only actions
- visible error text linked to fields
- traceable confirmation for destructive or high-risk actions

Accessibility violations are checker diagnostics when they affect reachable UI
actions. The v0.2 checker currently enforces that a form with validation rules
and validation failure traces has an accessibility announcement.

Destructive actions reachable from forms require explicit confirmation:

```text
The form requires confirmation:

- reviewer confirms ticket deletion
```

Without that confirmation, a form that calls an action with destructive writes
such as `deletes Ticket` is rejected with `AIL-UI-CONFIRM-001`.

## Responsive Layout Constraints

Layout constraints are semantic when they affect visibility, permission, or
action availability. A responsive layout declaration may specify:

- minimum readable width
- component stacking order
- data hiding rules
- action availability rules
- overflow behavior

Hidden fields that contain secrets or permissions must remain represented in
AIL-Core and trace explanations.

## State

UI state is explicit:

- local form state
- local view filter state
- remote resource state
- optimistic pending state
- error state
- stale data state

Remote state changes require AIL actions. Optimistic updates require rollback
behavior and trace events.

## Permissions In UI

UI permission checks must match backend permission checks. A hidden button is
not an authorization boundary. The action still requires the permission in
AIL-Core.

## Error States

Every action reachable from UI declares:

- user-visible failure text
- retry behavior
- compensation if any
- trace event
- accessibility behavior for error announcement

## AIL-Flow Projection

The UI profile projects into AIL-Flow as:

- Route Map
- View Cards
- Component Cards
- Form Blocks
- Event Blocks
- Permission Highlights
- Failure State Blocks
- Accessibility Review Blocks

Visual UI edits become graph patches and must pass the same checker as
canonical structured English edits.

Current AIL-Flow projection includes top-level `routes`, `forms`,
`dashboards`, `workflows`, and `accessibility` blocks with edge references back
to AIL-Core.

## Accepted Fixtures

CRUD app:

```text
Route Ticket list shows Ticket rows.
Form Create ticket calls CreateTicket.
Action CloseTicket is available only to SupportAgent.
Failure PermissionDenied shows "Permission denied" and records trace.
```

Dashboard:

```text
Dashboard: Support manager dashboard.

The dashboard reads:

- Ticket.status

The dashboard requires permission:

- Support manager may view overdue tickets

The dashboard filters:

- status is Overdue

The dashboard records trace:

- DashboardViewed
```

Multi-step workflow:

```text
Workflow: Refund approval.

The workflow steps are:

- Request
- Manager approval
- Provider call

The workflow blocks:

- Provider call before Manager approval
```

Accessibility trace:

```text
If form validation fails:

- FormValidationFailed

The form accessibility is:

- title error is announced
```
