# AIL-Spec Structured English

## Purpose

AIL-Spec is deterministic structured English. It is the human-reviewable source
projection used before and after normalization into AIL-Core.

AIL-Spec should be readable by English speakers without requiring them to learn
symbol-heavy programming syntax. It should also be regular enough for an AI
Agent to generate, patch, normalize, and explain.

AIL-Spec has two modes:

- `AIL-Spec Canonical`: parser-owned, deterministic structured English.
- `AIL-Spec Friendly`: user-facing explanation rendered from AIL-Core.

The compiler parses only Canonical. Friendly text may help review, but it must
cite canonical sections or graph node IDs and cannot be compiled directly.

## Required Qualities

An AIL-Spec document must be:

- explicit about actors, data, actions, rules, failures, secrets, permissions,
  effects, guarantees, and views
- deterministic enough to elaborate into AIL-Core
- stable enough to render from AIL-Core without losing semantics
- reviewable by non-engineers
- patchable in small sections
- diagnostic-friendly when something is missing or ambiguous

## Document Shape

An application document uses this shape:

```text
The application <name> manages <purpose>.

The application stores:
- <thing>

A <thing> has:
- <field name>: <type or explanation>

When <actor/event> <action>:
- the system requires ...
- the system reads ...
- the system changes ...
- the system calls ...
- if ... fails ...
- the system guarantees ...
```

Every paragraph that introduces behavior receives stable provenance so the
derived AIL-Core nodes can point back to the human-reviewed source.

## Stage-0 Source Text And Lexical Rules

Authority: Normative for `first-slice` AIL-Spec source parsing.

AIL-Spec Canonical source is UTF-8 text. A file that cannot be read as UTF-8 is
rejected before parsing. The stage-0 parser treats the source as an ordered
sequence of physical lines.

Stage-0 lexical rules:

- leading and trailing whitespace on each physical line is ignored
- blank lines are ignored
- a line whose trimmed text starts with `#` is a comment and is ignored
- bullet lines must start with the exact marker `- `
- structural headings and section sentences are matched after trimming
- non-structural continuation lines are valid only in parser-owned continuation
  slots; they extend the current slot instead of creating new behavior
- the canonical renderer emits `\n` line endings

The parser does not infer semantics from Markdown formatting, numbered lists,
tables, emphasis, indentation, or arbitrary prose. Anything that is not one of
the accepted stage-0 forms must either elaborate through a profile-specific
parser rule or produce a diagnostic.

## Stage-0 Accepted Canonical Grammar

Authority: Normative for `first-slice` package parsing and rendering.

The stage-0 parser accepts the following canonical headings and section
sentences. These are the forms used by `render_ail_spec` and accepted by
`parse_ail_spec_text`.

Canonical headings:

```text
The application <name> manages <purpose>.
A <thing> has:
An <thing> has:
Tool: <name>.
System component: <name>.
Compiler pass: <name>.
Action: <name>.
Failure <name> happens when <condition>:
```

Stage-0 action bullets use one verb phrase per semantic operation:

```text
- the system requires <rule>
- the system reads <thing.field>
- the system changes <thing.field> to <value>
- the system calls <action or function> with <arguments>
- the system repeats <ActionName> <count> times
- the system claims scheduler behavior for <scheduled work>
- the system uses temporal policy <policy name or window>
- the system records a trace event named <TraceName>
- the system guarantees <guarantee>
```

Normative rule `ail.spec.action.call-as-effect`: in `first-slice`, an action
call bullet is preserved as an auditable write/effect text in AIL-Core. It is
not yet a first-class function-call or return-value edge.

Tool, compiler-pass, and system-profile sections use their profile-specific
headings from `06-agent-tools.md`, `10-meta-profile.md`, and
`09-system-profile.md`.

## Target Reference Forms Under Evolution

Authority: Implementation note.

The broader AIL reference expects additional headings such as:

```text
Package: <package name>.
Application: <name>.
Function: <name>.
Route: <name>.
Form: <name>.
C library: <name>.
Import package: <package>@<version> as <alias>.
```

It also expects first-class external binding calls and return values. These
forms are reserved for later profile work and are not accepted by the stage-0
parser unless a profile-specific document and conformance fixture says so.

Friendly renderers may paraphrase these forms only outside the parser boundary.

## Requirement Preservation Rules

Normative rule `ail.spec.requirements.failure-preserved`: when an accepted
AIL-Requirements artifact contains a canonical failure sentence of the form
`Failure <name> happens when <condition>`, the derived AIL-Spec Canonical
document must contain a `Failure <name> happens when <condition>:` section
before it can elaborate to accepted AIL-Core.

If the section is missing, the checker emits `AILR012`. The agent may repair
the draft only by adding an explicit Failure section with handling and trace
bullets, or by asking a blocking question when the requirement is ambiguous.

Normative rule `ail.spec.requirements.permission-preserved`: when an
AIL-Requirements artifact states that an action requires permission, role,
approval, access, or forbidden-state enforcement, the derived AIL-Spec
Canonical action must contain an explicit requirement bullet that preserves
that enforcement. If the action drops the requirement, the checker emits
`AILR011`.

## Control Flow Forms

Branch:

```text
If <condition>:
- the system <operation>
Otherwise:
- the system <operation>
```

Loop:

```text
While <condition> remains true:
- the system <operation>
- the system records a trace event named <TraceName>
```

Finite iteration:

```text
For each <item> in <collection>:
- the system <operation using item>
```

Match:

```text
When <value> is Some:
- the system <operation>
When <value> is None:
- the system <operation>
```

Recursion:

```text
The function calls <function> with <arguments>.
```

Profiles that require termination must include a bounded iteration policy or a
structural decrease explanation.

## Function And Action Calls

Function signatures:

```text
Function: <name>.

The function needs:
- <input>: <type>

The function produces:
- <output>: <type>
```

Action calls:

```text
The action calls <action name> with:
- <parameter>: <value>
```

Functions are pure unless their signature declares effects. Actions may have
effects when permissions, failures, guarantees, and traces are explicit.

## External Calls And C Imports

External calls:

```text
The system calls external binding <binding name>.
```

C imports:

```text
C library: <library>.

The library imports function <symbol>.

<symbol> needs:
- <parameter>: <type and ownership>

<symbol> maps errno or status codes:
- <success-code> maps to success
- <code> maps to Failure.<name>
```

Every external call requires an effect, permission or capability, explicit
status mapping, secret-redaction rule when relevant, and trace event.

## Library Imports And Standard Library Usage

Package import:

```text
Import package: ail.std.collections@0.1 as Collections.
```

Standard library type usage:

```text
A Ticket has:
- assignee: Option<User>
```

Imported declarations keep their package identity in AIL-Core. The checker
rejects ambiguous aliases and unsupported package versions.

## UI Forms

Routes, views, components, and forms are canonical AIL-Spec sections:

```text
Route: Ticket detail.

The route path is:
- /tickets/:ticket_id

The route reads:
- Ticket.status
```

UI actions still lower to ordinary actions, rules, permissions, failures,
guarantees, and traces.

## Application Sections

An application section defines the purpose, users, stored things, external
systems, views, and top-level guarantees.

Required slots:

- purpose
- actors
- stored things
- external systems
- visible views
- global secrets
- global guarantees

## Action Sections

An action section defines one executable behavior.

Required slots:

- trigger or actor
- inputs
- preconditions
- reads
- writes
- external calls
- failures
- approvals
- guarantees
- trace expectations

The checker rejects an action that changes data without a declared permission
or reads a secret without a declared capability.

## Tool Sections

A tool section defines a capability that an AI Agent or runtime component can
request. It must name purpose, allowed use, inputs, outputs, permissions,
effects, secrets, approvals, failures, guarantees, and audit trace events.
Permission requirements are explicit grants; they are not inferred from
general precondition prose.
Approval requirements are explicit review gates; they are not inferred from
general "must not" prose.

## Failure Sections

Failures are named outcomes, not hidden exceptions. Each failure must define:

- when it can occur
- which data remains unchanged
- which compensation runs
- what the user or caller sees
- what trace event is recorded

## Secret And Permission Sections

Secrets must be named as `Secret<T>` values or structured fields that contain
secrets. Permissions must describe who or what may read, write, call, approve,
or disclose each resource.

The spec must say what data may be revealed to each audience.

## Human Confirmation Rules

The AI Agent may infer draft details, but accepted AIL-Spec must distinguish:

- human-stated facts
- agent-inferred facts
- defaults from a package or profile
- unresolved questions

Human confirmation is required before compiling an inferred rule that affects
permissions, effects, secrets, money, safety, or external calls.

## Invalid Or Ambiguous Specs

The checker rejects AIL-Spec or derived AIL-Core when required semantic slots
are missing. Examples include:

- an action says it "updates the account" without naming fields
- a tool calls an external provider without declaring effects
- a response exposes a value whose type contains `Secret`
- a failure is named but no handling or trace is defined
- two actions can write the same value concurrently without a join rule
