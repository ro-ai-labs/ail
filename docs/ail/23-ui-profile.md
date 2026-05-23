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
nodes with deterministic render/reparse. Components, forms, workflow blocks,
responsive constraints, and accessibility diagnostics remain future UI-profile
implementation slices.

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

- the form shows field error TitleRequired
- the action does not run
- the trace records FormValidationFailed
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
actions.

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
View Support manager dashboard reads overdue tickets.
The view requires SupportManager permission.
The view records DashboardViewed.
```

Multi-step workflow:

```text
Workflow Refund approval has steps Request, Manager approval, Provider call,
Ledger write, Customer notification.
Provider call cannot run before approval.
```

Accessibility trace:

```text
trace FormValidationFailed field=title announcement=TitleRequired
```
