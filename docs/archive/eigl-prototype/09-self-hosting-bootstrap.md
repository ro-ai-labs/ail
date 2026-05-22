# Self-Hosting and Bootstrap Plan

The long-term goal is for EIGL to compile itself.

```text
EIGL is specified in EIGL.
The EIGL compiler is generated from that specification.
The generated compiler can compile the same specification again.
```

## Why self-hosting matters

Self-hosting demonstrates that:

```text
the language can express compiler logic
the intermediate representation is precise enough
the checker can validate complex systems
the generated compiler is not merely a toy
the language can maintain itself
```

## Start with RIF, not RSL

The first self-hosted compiler should not be written in the compact requirement surface.

It should be written in RIF.

Reason:

```text
RSL is pleasant but ambiguous by design.
RIF is explicit, normalized, and reviewable.
EIG-Core is canonical and checkable.
```

Self-hosting should begin at the precise layer.

## Bootstrap stages

### Stage -1: Manual trusted kernel

Written in an existing systems language.

Possible choices:

```text
Rust
Zig
C++
OCaml
Haskell
```

Responsibilities:

```text
parse EIG-Core or restricted RIF
validate graph well-formedness
check basic types
check permissions
check effects
check failures
emit bytecode or Wasm
run tests
```

This implementation should be small, auditable, and boring.

### Stage 0: Seed compiler

Written in an existing language.

Capabilities:

```text
compile restricted RIF to EIG-Core
run basic checker
lower to bytecode or Wasm
execute simple process graphs
produce diagnostics
produce views
```

Stage 0 does not need full RSL.

### Stage 1: Compiler spec written in RIF/EIG-Meta

Define compiler passes as RIF/EIG-Meta intents:

```text
BuildCoreGraphFromRIF
CheckReferences
InferPermissions
CheckEffects
CheckFailures
RenderExplanation
RenderFlowView
LowerProcessToIR
```

Stage 0 compiles this compiler spec into Compiler-1.

### Stage 2: Compiler-1 compiles the compiler spec

Compiler-1 compiles the same RIF/EIG-Meta compiler spec and produces Compiler-2.

### Stage 3: Compiler-2 compiles the compiler spec

Compiler-2 produces Compiler-3.

### Fixed-point check

Check that Compiler-2 and Compiler-3 are equivalent.

Possible equivalence levels:

```text
same accepted EIG-Core
same diagnostics
same generated explanations
same graph outputs
same executable behavior
bit-identical output under deterministic build mode
```

Bit-identical output is ideal but not required at first.

## Self-hosting definition

Minimal self-hosting:

```text
An EIGL compiler generated from an EIGL compiler specification can compile the same specification again and produce an equivalent compiler.
```

Strong self-hosting:

```text
Compiler-N and Compiler-N+1 produce equivalent EIG-Core and equivalent executable output for the compiler specification and standard test suite.
```

Deterministic self-hosting:

```text
Compiler-N and Compiler-N+1 are bit-identical under deterministic build settings.
```

## Trusted base

Initial trusted base:

```text
seed compiler
trusted checker
runtime primitives
backend code generator
standard library primitives
```

Over time, move into EIGL:

```text
phrase matcher
resolver
RIF elaborator
diagnostics
view renderers
many type rules
many permission rules
many effect rules
rewrite rules
optimization rules
lowering rules
test generation
```

Remain trusted:

```text
EIG-Core checker
proof checker
primitive runtime
machine-code/Wasm emitter or verified backend bridge
trusted capsules
```

## Compiler pass example in RIF

```text
intent BuildCoreGraphFromRIF

subject:
  rif_document: RIFDocument

inputs:
  language_spec: LanguageDefinitionPackage

outputs:
  graph: EIGCoreGraph

steps:
  1. Create graph module nodes from RIF modules.
  2. Create type nodes from RIF things.
  3. Create operation nodes from RIF actions.
  4. Create contract nodes from RIF rules.
  5. Create effect edges from RIF effect declarations.
  6. Create permission edges from RIF permission declarations.
  7. Create failure edges from RIF failure behavior.
  8. Validate that every reference resolves to one graph node.

failure behavior:
  if a reference cannot be resolved:
    emit UnresolvedReference diagnostic
    stop

  if a reference resolves to multiple graph nodes:
    emit AmbiguousReference diagnostic
    stop

guarantees:
  every graph node has a stable identity
  every edge source exists
  every edge target exists
  every operation has declared inputs and outputs
```

## Permission inference compiler pass

```text
compiler pass Infer permissions:

  input:
    intent: Intent

  output:
    intent with permission requirements

  rule:
    If a step reads a field, the step needs Read permission for that field.

  rule:
    If a step sets a field, the step needs Change permission for that field.

  rule:
    If a step passes a thing to an action that consumes it, the thing is no longer available after that step.

  rule:
    If two unordered steps both need Change permission for the same thing, the intent is unsafe.

  diagnostic:
    "Steps {first_step} and {second_step} both change {thing}. They must be ordered, combined, or protected by synchronization."

  guarantees:
    every read has a Read permission
    every change has a Change permission
    no unordered steps require conflicting Change permissions
```

## Compiler determinism policy

Compiler passes should be deterministic by default.

```text
policy CompilerDeterminism:

  compiler passes may not read:
    wall clock time
    random numbers
    environment variables
    filesystem outside declared inputs
    network resources

  unless:
    the pass declares the effect
    and the build mode permits it
```

This supports reproducible builds and fixed-point tests.

## Meta-rule: compiler must not invent meaning

```text
rule A compiler pass must not invent meaning.

A compiler pass may:
  infer a fact from a declared rule,
  copy a fact from an input,
  derive a fact from a checked constraint,
  or ask for clarification.

A compiler pass must not:
  choose between two possible meanings without a disambiguation rule.
```

Formal form:

```text
DerivedFact(f) requires
  SourceFact(f)
  or InferenceRule(rule, premises, f)
  or CheckedConstraint(c, f)
  or UserDecision(f)

Ambiguous(candidates) and no DisambiguationRule
  => Error(AmbiguousMeaning)
```

## Practical self-hosting milestone

The first major milestone should be:

```text
The RIF compiler written in RIF compiles itself.
```

Not:

```text
The full English-like RSL compiler compiles itself.
```

RSL self-hosting can come later.

## Long-term goal

```text
Requirements compile programs.
Requirements also compile the compiler.
```
