# AIL-Spec Grammar Sheet

This sheet is shipped to the agent under test as part of its system prompt, next
to one complete checker-clean exemplar. `32-capability-ladder-and-improvement-loop.md`
§5.1 measured that a grammar sheet plus an exemplar is what moves spec drafting
from hard-reject to `ail diagnostics: none`; without both, the model does not
produce a parseable spec at all.

Every rule below is the surface form the Rust parser in `src/ail.rs` actually
accepts. It is a description of the implementation, not an aspiration.

---

## Document shape

A spec is plain Markdown. Order matters:

1. a title heading
2. the application declaration
3. the users list (optional)
4. one `A <Thing> has:` block per kind of thing
5. the views list (optional)
6. one block per action
7. one block per failure

## The application declaration

Exactly one line, exactly this shape:

```text
The application <Name> manages <purpose>.
```

`<Name>` is a short capitalised name. `<purpose>` is a phrase. The words `The
application ` and ` manages ` are literal; the parser splits on them. Anything
else is not an application declaration and the whole spec is rejected.

## Users

```text
The application has these users:

- Customer
- Support agent
- Support manager
```

The header line is literal, character for character.

## Things and fields

```text
A Ticket has:

- id: Text
- title: Text
- status: State<New, Open, Assigned, Closed>
- customer: User
- assignee: Option<User>
- created_at: Time
- public_updates: List<Text>
- internal notes: Secret<List<Text>>
```

The header is `A <Name> has:` or `An <Name> has:` and nothing else. Each field is
`- <name>: <Type>`.

Types: `Text`, `Int`, `Time`, `Bool`, another declared thing, and the wrappers
`Option<T>`, `List<T>`, `Secret<T>`, `State<A, B, C>`. Wrappers nest:
`Secret<List<Text>>`.

`Secret<T>` is how a field is marked as holding something that must not leak. Use
it for anything the person described as private, sensitive, or restricted.

## Views

```text
The application shows:

- an open ticket queue for support agents
- a customer-visible ticket history that never includes internal notes
```

The header line is literal.

## Actions

```text
Action: Close ticket.

When a support agent closes a ticket:

- the system requires the ticket to exist
- the system changes the ticket status to Closed
- the system records a trace event named TicketClosed
```

The `Action:` line names it. The `When ...:` line names who does it. Then one
bullet per step.

**Bullets are matched by literal prefix.** A bullet that starts with none of
these prefixes carries no meaning. Use only these:

| Prefix | Use |
|---|---|
| `- the system requires ` | a precondition |
| `- the system reads ` | reads state |
| `- the system changes ` | writes state |
| `- the system creates ` | creates a thing |
| `- the system deletes ` | deletes a thing |
| `- the system removes ` | removes from a collection |
| `- the system records ` | records an update or value |
| `- the system records a trace event named <Name>` | audit record |
| `- the system increments <field> by <literal number>` | add |
| `- the system decrements <field> by <literal number>` | subtract |
| `- the system calls ` | an external call |
| `- the system repeats ` | repetition |
| `- the system guarantees ` | a property that must hold |
| `- the system does not reveal ` | a non-disclosure obligation |
| `- if <condition> ` | a conditional step |

Trace event names are single CamelCase words: `TicketClosed`, `BookingCancelled`.

The number in `increments`/`decrements` must be a literal digit. A variable there
is not supported.

## Failures

```text
Failure NotFound happens when a ticket id does not match a stored ticket:

- the system changes no ticket data
- the caller sees "Ticket not found"
- the trace records TicketNotFound
```

The shape is `Failure <Name> happens when <condition>:`. Bullets inside a failure
use `- the system ...`, `- the caller sees "<message>"`, and
`- the trace records <TraceName>`.

Write a failure for every way the thing the person described can go wrong,
including permission being refused.

## What the checker will not tell you

The checker validates one artifact in isolation. It does not know what the
person asked for. A spec can be `ail diagnostics: none` and still be the wrong
application. Getting it right is your job, not the checker's, and the way you
get it right is by asking rather than guessing.
