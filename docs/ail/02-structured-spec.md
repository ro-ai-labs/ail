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
- the system computes <thing.field> as <integer expression>
- the system computes boolean <thing.field> as <Boolean expression>
- the system returns <thing.field>
- the system calls <ActionName>
- the system repeats <ActionName> <count> times
- the system claims scheduler behavior for <scheduled work>
- the system uses temporal policy <policy name or window>
- the system records a trace event named <TraceName>
- the system guarantees <guarantee>
```

Calls to declared actions execute in source order alongside writes and repeats.
The optional word `sequentially` may follow a repeat's `times`. Repeats require
a positive integer count. Actions share state; these calls do not implement
general functions with arguments and return values. `returns` selects fields
for result output and does not exit early.

The executable integer extension also accepts these control bullets:

```text
- if <integer comparison>:
- otherwise:
- end if
- while <integer comparison>:
- break
- continue
- end while
```

Blocks contain executable writes, calls, repeats and other control statements.
Action contract declarations remain outside blocks. See
[integer expressions](34-integer-expressions.md) and [control flow](35-control-flow.md)
for complete grammar, limits and execution semantics.

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


## Executable integer functions

`The function executes:` introduces an ordered executable body. Inputs use
`The function needs:`, the single result uses `The function produces:`, and
optional locals use `The function stores:`. Types may be `Int`, `Float`, `Bool`, `Text`, `Bytes`,
`List<Int>`, `List<Float>`, `List<Text>` or `Json`; see [value functions](44-value-functions.md).
Use `local.name` inside functions; named value calls and early returns execute
through checked Core and STE. Legacy `When <function> runs:` bodies remain
descriptive and cannot mix with executable sections. See the
[integer function grammar](36-integer-functions.md) for complete forms and limits.

## Executable text and lists

Actions support [Unicode text operations](37-text-operations.md) and
[typed integer/text lists](38-typed-lists.md) and [floating lists](46-floating-lists.md). These statements are ordered with
computations, value calls, branches and loops. Their Core operands carry checked
global field types and semantic secret accesses; unsupported profiles reject
them. [Value functions](44-value-functions.md) support these operations on
declared locals with the `the function` prefix.

[Text transformations](49-text-transformations.md) accept these field operands:

```text
- the system splits text <source> by <separator> into <List<Text> destination>
- the system joins list <List<Text> source> with <separator> into <destination>
- the system replaces text <needle> in <source> with <replacement> into <destination>
- the system trims text <source> into <destination>
```

All fields other than the indicated lists have type `Text`. Split preserves empty
fields and requires a nonempty separator. Replace handles an empty needle at
Unicode scalar boundaries; trim removes a fixed set of boundary whitespace.

[List transformations](50-list-transformations.md) keep the same list element
type on every input and output:

```text
- the system concatenates list <source> and list <other> into <destination>
- the system slices list <source> from <Int expression> to <Int expression> into <destination>
- the system reverses list <source> into <destination>
- the system sorts list <source> in ascending order into <destination>
- the system sorts list <source> in descending order into <destination>
```

Slice bounds clamp to endpoints. Sorting is stable in both directions; Float
NaNs are placed last and both signs of zero retain their relative order.

Use `the system assigns <JSON literal> to <field>` to construct typed constants.
The destination type determines whether the literal must be an integer,
finite floating number, boolean, quoted text, integer/floating/text list or Json. See
[typed literals](39-typed-literals.md) for grammar, escaping and limits.


### Named file resources

Use `the system reads text from file resource input into data.text` and
`the system writes text data.text to file resource output` for executable UTF-8
file I/O. Fields must be global Text. Resources require explicit caller-provided
read/write bindings; source and package metadata cannot grant their own access.
See [file resources](40-file-resources.md) for syntax and runtime behavior.


### Typed JSON conversion

Use `the system parses JSON text data.raw into data.values` to parse a Text field
according to the destination type. Use `the system formats data.values as JSON
text into output.text` to serialize a typed value into Text. Supported types are
Int, Float, Bool, Text, List<Int>, List<Float>, List<Text> and Json. Both execute in source order through
Core, STE and the VM/native backends. See [JSON conversion](41-json-conversion.md).

Declare `Json` for objects, nested or mixed arrays, null and lossless number
tokens. `the system gets JSON value at path data.path from data.document into
output.value` uses a Text JSON Pointer and decodes according to the destination
type. `the system counts entries in JSON data.document into output.count`
counts immediate array items or object member pairs. See
[recursive JSON](42-recursive-json.md) for exact grammar, limits and errors.
`the system gets JSON kind from data.document into output.kind` produces Text;
`the system gets JSON keys from data.document into output.keys` produces List<Text>
for an object. See [JSON inspection](47-json-inspection.md) for generic traversal.

Set and remove create updated Json values from global fields:

```text
the system sets JSON value at path data.path in data.document to data.value into output.document
the system removes JSON value at path data.path from data.document into output.document
```

Set converts the declared value type, replaces an existing target, adds a
missing object member, or appends to an array with final `/-`. Parent containers
must exist. Remove requires an existing unique target and cannot remove the
root. Both preserve unmodified member order and number spellings. See
[JSON mutation](43-json-mutation.md).

[Boolean predicates](48-boolean-predicates.md) support `if boolean ...:` and
`while boolean ...:` controls, `not`/`and`/`or`, grouping and explicitly typed
Int/Float/Text/Bool comparisons. All operands remain statically checked even
when runtime short-circuiting skips their evaluation.

## Runtime recovery

Actions and executable value functions support `try:`, exactly one
`recover <categories> into <Text field>:` clause, and `end try`. Categories use
` or ` separators or the keyword `any`; the handler receives a stable runtime
error code. Earlier successful writes persist, and resource/execution/IO errors
remain fatal. See [runtime recovery](51-runtime-recovery.md) for the exact
category set, nesting, call propagation and checked scope rules.


## Standard streams

Application/C interop actions support ordered stream effects on declared global
Text fields:

```text
- the system reads text from standard input into data.body
- the system writes text data.body to standard output
- the system writes text data.warning to standard error
```

Pass `--stdio` at execution time for grants and clean stdout/stderr. Reads are
bounded through EOF; writes add no newline. Functions and compiler passes reject
these effects. See [standard streams](52-standard-streams.md) and the
[JSON-lines filter](../../examples/json-lines-filter).

## Process actions

Use the exact `the system runs process resource ...` form in [process resources](53-process-resources.md). Its arguments/input/timeout and three destinations are declared global typed fields; effects require a caller binding.

Use the three exact directory forms in [directory resources](54-directory-resources.md): list names, read Text, and write Text. Relative paths and contents are global Text fields; listing destinations are List<Text>. Application/C interop actions can place these effects in loops, branches and recovery scopes.

## Binary values and files

Declare `Bytes` for arbitrary octets. Literal assignments use canonical JSON
strings such as `"hex:00ff"`. The ten pure forms in [Bytes](56-bytes.md) work in
actions and typed functions. Binary file effects use `the system reads bytes
from file resource input into data.body` and `the system writes bytes data.body
to file resource output`; both require existing caller file grants.

## Explicit errors

Use `the system raises error BYTES` in actions or `the function raises error BYTES`
in executable functions. The category is a single literal catchable category; no
message, dynamic category or fatal error is accepted. Existing recovery handles
the failure. See [explicit errors](57-explicit-errors.md).

## Executable variant data

Existing `Type` declarations can define recursive custom values and concrete
generic instantiations. Use `constructs TYPE variant VARIANT with ARGUMENTS into
DESTINATION`, `tests TYPE variant VARIANT of SOURCE into DESTINATION`, and
`gets field FIELD of TYPE variant VARIANT from SOURCE into DESTINATION`, with the
normal `the system` or `the function` subject. Named constructor arguments are
separated by semicolons; no-payload variants use `with none`. See
[executable variants](59-executable-variants.md) for exact types and examples.
