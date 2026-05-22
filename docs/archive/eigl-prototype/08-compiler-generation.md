# Compiler Generation

EIGL should eventually generate its compiler from language specifications written in EIGL itself.

The compiler is not primarily hand-written code. It is a set of requirements, rules, transformations, constraints, diagnostics, explanations, visualizations, and lowering contracts.

## Thesis

```text
The compiler is an EIGL program.
```

Or more precisely:

```text
The language definition is written in EIG-Meta.
The compiler is generated from the language definition.
The generated compiler validates, explains, visualizes, and compiles EIGL programs.
```

## Language Definition Package

The compiler should be generated from a **Language Definition Package** or **LDP**.

An LDP contains:

```text
vocabulary
phrase patterns
domain dictionaries
syntax rules
elaboration rules
type rules
permission rules
effect rules
failure rules
contract rules
rewrite rules
lowering rules
diagnostic templates
explanation templates
visual view definitions
test-generation rules
backend contracts
```

## EIG-Meta

EIG-Meta is the language layer for writing LDPs.

It defines compiler concepts such as:

```text
SourceText
Phrase
PhrasePattern
ParseTree
RequirementDraft
ResolvedRequirement
Intent
Type
Operation
Permission
Effect
Failure
Contract
Diagnostic
Graph
Node
Edge
RewriteRule
CompilerPass
LoweringTarget
BackendContract
```

## Compiler passes as intents

Each compiler pass can be specified as an EIGL intent.

Example:

```text
compiler pass Resolve domain vocabulary:

  input:
    requirement draft
    domain dictionary

  output:
    resolved requirement

  does:
    find every noun phrase
    match each noun phrase to a known thing
    find every verb phrase
    match each verb phrase to a known action or phrase rule
    record every match with provenance

  if a phrase has no meaning:
    report Unknown phrase

  if a phrase has more than one meaning:
    report Ambiguous phrase

  guarantees:
    every resolved phrase has exactly one meaning
    every inferred meaning has a provenance record
```

## Compiler pipeline

Generated compiler pipeline:

```text
Requirement text
  ↓
Parse candidate phrases
  ↓
Resolve domain vocabulary
  ↓
Elaborate into RIF
  ↓
Normalize RIF
  ↓
Build EIG-Core graph
  ↓
Check types
  ↓
Check permissions
  ↓
Check effects
  ↓
Check failures
  ↓
Check contracts
  ↓
Generate explanations and views
  ↓
Lower to EIG-IR
  ↓
Optimize
  ↓
Emit executable artifact
```

## Generated components

The LDP should generate:

```text
phrase matcher
resolver
RIF elaborator
EIG-Core builder
type checker
permission checker
effect checker
failure checker
contract checker
explanation renderer
visual renderer
optimizer
lowering pipeline
diagnostics
tests
```

Not every generated component is trusted. The trusted checker validates outputs.

## Small trusted kernel

The entire compiler should not be trusted equally.

Suggested trust model:

```text
Generated elaborator: untrusted or semi-trusted
Generated optimizer: untrusted or semi-trusted
Generated explanation renderer: untrusted
Generated visualizer: untrusted
Generated diagnostics: untrusted

Trusted kernel:
  checks EIG-Core validity
  checks types
  checks permissions
  checks effects
  checks contracts/proof objects
  checks lowering obligations
```

The front end may be clever or LLM-assisted, but the accepted graph must pass the trusted checker.

## Vocabulary definitions

Example EIG-Meta vocabulary:

```text
word "order":
  means thing Order
  plural "orders"

phrase "draft order":
  means Order where status is Draft

phrase "confirmed order":
  means Order where status is Confirmed
```

## Action phrase definitions

```text
phrase "reserve inventory":
  needs order: Order
  means call Inventory.reserve(order.items)
  produces reservation: InventoryReservation
  changes Inventory
  compensation is Inventory.release(reservation)
```

## Phrase pattern definitions

```text
phrase pattern State transition requirement:

  form:
    "{subject} can become {target_state} when {condition}"

  produces:
    intent StateTransitionIntent

  fields:
    subject: Thing
    target_state: State of subject
    condition: Condition
```

## Compact syntax definition

```text
phrase pattern Compact state transition:

  form:
    "{action_name}: {from_state} -> {to_state} by {step_list}"

  examples:
    "Confirm order: Draft -> Confirmed by reserve inventory, capture payment, create shipment"

  parse:
    action_name is ActionName
    from_state is State
    to_state is State
    step_list is List<Phrase>

  infer:
    subject is the thing that owns from_state
    action changes subject state from from_state to to_state
    each phrase in step_list becomes a step

  require:
    from_state and to_state belong to the same state set
    every step phrase resolves to exactly one action
    the final state transition is valid for the subject

  produce:
    StateTransitionIntent
```

## Checking rule example: exclusive change

Human-readable rule:

```text
rule A thing that is changed by an action must be available exclusively to that action while the change happens.
```

RIF/EIG-Meta:

```text
checking rule ExclusiveChangePermission

applies to:
  action: Action
  thing: Thing

when:
  action changes thing

requires:
  action has Change permission for thing
  no other active action has Read permission for thing during the change
  no other active action has Change permission for thing during the change

diagnostic if violated:
  "This is not safe because {action} changes {thing}, while {other_action} also reads or changes it."
```

Formal constraint:

```text
Changes(action, thing) => RequiresPermission(action, Change(thing))

Holds(Change(action_a, thing), time)
  => not Holds(Read(action_b, thing), time)
  and not Holds(Change(action_b, thing), time)
  unless action_a == action_b
```

## Checking rule example: failures

```text
checking rule Declared failures must be handled:

  when:
    an action contains a step
    and the step may fail with a failure

  require:
    the action handles the failure
    or the action returns the failure
    or the action proves the failure cannot happen

  diagnostic if violated:
    "{step} may fail with {failure}, but {action} does not say what to do."
```

Formal constraint:

```text
StepInAction(step, action)
and MayFail(step, failure)
requires
  Handles(action, step, failure)
  or Returns(action, failure)
  or ImpossibleWithProof(action, step, failure)
```

## Diagnostics as part of the spec

Diagnostics should be generated with rules.

```text
diagnostic ConflictingChangePermission:

  when:
    two unordered steps both change the same thing

  message:
    "This is not safe because {first_step} and {second_step} both change {thing} at the same time."

  explain:
    "Only one step may change a thing at a time."

  suggest:
    "Run {first_step} before {second_step}."
    "Run {second_step} before {first_step}."
    "Combine both changes into one step."
    "Add a synchronization rule."
```

## Rewrite rules

Optimizations should be declarative and checked for legality.

```text
rewrite rule Remove redundant state set:

  when:
    a step sets field to value
    and the next step sets the same field to the same value
    and no step between them reads the field

  replace:
    remove the second set

  legal only if:
    setting the field has no external effect
    setting the field does not emit an event
    setting the field does not update audit history

  guarantee:
    observable behavior is unchanged
```

## Parallelization rule

```text
rewrite rule Parallelize independent pure steps:

  when:
    two steps are unordered
    both steps are pure
    neither step reads a value produced by the other

  replace:
    run the steps in parallel

  legal only if:
    both steps have no effects
    both steps do not read time or randomness

  guarantee:
    the result is deterministic
```

## Lowering rules

```text
lowering rule Process step to IR block:

  when:
    a process contains a step

  emit:
    an IR block for the step
    data edges for step inputs
    success edge to the next step
    failure edge to the step failure handler

  require:
    every input has a producer
    every failure edge has a target
    every effect has a capability

  guarantee:
    the IR block has the same declared effects as the source step
```

## Backend contracts

```text
backend contract NativeCode:

  accepts:
    checked EIG-IR

  guarantees:
    preserves control flow
    preserves data dependencies
    preserves permission constraints
    preserves effect order where required
    does not introduce unauthorized effects
    does not expose Secret values
    does not call trusted capsules unless explicitly present in EIG-IR

  may optimize:
    pure computations
    independent reads
    stack allocation
    region allocation
    static dispatch
    vectorization
    inlining

  must report:
    unsupported target feature
    unsupported trusted capsule
    layout conflict
    unresolved external capability
```

## Compiler generation pipeline

```text
Language Definition Package
        ↓
Validate language spec
        ↓
Generate compiler passes
        ↓
Generate diagnostics
        ↓
Generate explanation renderer
        ↓
Generate visual renderers
        ↓
Generate tests/fuzzers
        ↓
Generate bootstrap compiler
```

## LLM role in compiler generation

LLMs may help draft compiler rules, but cannot be trusted as final authority.

Good roles:

```text
suggest phrase patterns
translate informal design rules into EIG-Meta drafts
generate examples
explain compiler rules
find missing diagnostics
propose rewrite rules
generate tests from the spec
```

Bad roles:

```text
silently changing compiler semantics
generating unchecked machine code
resolving ambiguity without provenance
inventing lowering behavior without contracts
bypassing the trusted checker
```

## Compiler-generation design rule

The generated compiler can be clever. The trusted checker must be boring.
