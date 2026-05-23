# AIL Language Reference Style

## Purpose

This document defines how the AIL language reference is written and maintained.
It borrows useful structure from mature language references while keeping AIL's
own boundary clear: AIL is an English-first semantic language whose trusted
source of truth is checked AIL-Core.

The AIL reference is not a tutorial, product guide, prompt cookbook, or stage-0
implementation manual. Those documents may exist, but they do not define
language semantics unless this reference links to them as normative artifacts.

Existing implementation notes stay implementation notes until they are promoted
with authority labels, rule identifiers, and conformance links.

## External Reference Inputs

The style guide uses these references as comparison points:

- [Rust Reference](https://doc.rust-lang.org/reference/): primary language
  reference, standalone chapters, rule labels, and test-linked rules.
- [Go Language Specification](https://go.dev/ref/spec): compact notation,
  language-version heading, and explicit source, lexical, type, statement,
  package, and program structure.
- [Python Language Reference](https://docs.python.org/3/reference/index.html):
  practical separation of syntax formalism from prose semantics, implementation
  notes, and a clear boundary from tutorial and standard-library documentation.

AIL adopts the discipline, not the surface syntax, of those references.

## Imported Reference Practices

AIL uses these language-reference practices as requirements for its own
reference suite:

| Practice | AIL requirement |
| --- | --- |
| Scope boundary | Each reference document says what it defines, what it does not define, and which artifacts are authoritative. Tutorials, prompt cookbooks, and implementation guides are not language authority unless a normative rule links to them. |
| Version heading | Each released reference names the language, Core schema, prompt pack, bytecode, standard library, and conformance versions it describes. Feature additions carry version notes. |
| Notation section | Each syntax-bearing document defines its grammar, schema, graph, or table notation before using it for normative rules. |
| Source and lexical rules | Each accepted textual surface defines source encoding, line handling, comments, whitespace, tokens, canonical rendering, and parser-owned slots before listing grammar productions. |
| Rule anchors | Normative rules receive stable identifiers when diagnostics, tests, prompts, or evolution notes need to cite them. |
| Implementation notes | Host tool behavior, bootstrap shortcuts, and temporary limits are explicitly labeled and cannot silently define language semantics. |
| Conformance links | Normative rules identify the parser, checker, verifier, fixture, or trace that proves the rule is enforceable. |

## Authority Levels

Every AIL reference section added or promoted after this guide uses one of
these authority levels:

- Normative: defines accepted AIL behavior and must be checkable by parser,
  schema validator, checker, runtime verifier, or conformance fixture.
- Explanatory: clarifies a normative rule without adding behavior.
- Example: demonstrates valid or invalid artifacts.
- Implementation note: describes current stage-0 behavior that must not become
  language authority unless promoted through the evolution protocol.
- Rationale: explains why a decision was made and can change without changing
  language behavior.

Unlabeled prose in the numbered reference is normative only when it names a
language rule, artifact boundary, checker obligation, execution obligation, or
conformance requirement.

## Reference Section Header

Each normative reference document should begin with a short status block before
the first rule-bearing section:

```text
Authority: <Normative | Explanatory | Implementation note | Rationale>.
Language version: <version or draft label>.
Applies to: <AIL-Spec | AIL-Core | AIL-Flow | prompt pack | bytecode | profile>.
Accepted by: <parser | checker | verifier | renderer | conformance fixture>.
Out of scope: <tutorials, implementation detail, future target forms, or profiles>.
```

Mixed-authority documents may use this header per section, but every normative
section still needs an authority label and conformance link.

## Rule Identifiers

Normative rules should receive stable identifiers when they need cross-links
from diagnostics, tests, prompt packs, examples, or evolution proposals.

Rule identifier format:

```text
ail.<document>.<topic>.<rule>
```

Examples:

```text
ail.spec.action.requires-trace
ail.core.secret-read.requires-protection
ail.prompt.envelope.requires-artifact-or-questions
ail.runtime.trace.action-entry
```

Rule identifiers are stable within a language version. Renaming or splitting a
rule requires an evolution note, migration mapping, and conformance update.

## Grammar And Schema Notation

AIL uses different notation for different layers:

- AIL-Spec Canonical textual forms use structured English templates plus
  grammar fragments.
- AIL-Core uses versioned schema fragments, graph invariants, and an explicitly
  named serialization format.
- AIL-Bytecode uses opcode tables, stack/register effects, and verifier rules.
- Prompt envelopes use JSON object schemas plus prompt-protocol diagnostics.

Grammar fragments use this notation:

```text
RuleName = Term { Term } .
Term = "literal" | RuleName | [ Optional ] | { Repeated } .
```

Notation rules:

- CamelCase names are syntactic productions.
- lowercase names are lexical tokens or parser-owned slots.
- quoted text is literal canonical text.
- `{ X }` means zero or more repetitions.
- `[ X ]` means optional.
- `X | Y` means alternatives.
- prose inside angle brackets describes an English slot that must elaborate
  into a typed AIL-Core value, node, or edge.

Structured English slots are never accepted as vague free text at the compiler
boundary. Each slot must normalize to a checked AIL-Core representation or
produce a diagnostic. When a future target grammar is broader than the current
stage-0 parser, the reference must label that grammar as target behavior and
name the accepted stage-0 forms separately.

Textual surface documents must separate source representation, lexical
handling, and grammar:

- source representation defines encoding and file-level restrictions
- lexical handling defines comments, whitespace, line joining, token classes,
  and canonical line endings
- grammar defines accepted productions after source and lexical handling
- semantic elaboration defines how accepted productions become AIL-Core

This separation keeps English-first syntax teachable to AI agents without
making the compiler accept vague prose.

## Versioning

Each released AIL reference names:

- language version
- AIL-Core schema version
- prompt pack version
- bytecode version
- standard library compatibility version
- conformance suite version

The current draft version surface is recorded in `README.md` under Reference
Status / Versions. Versioned examples and corpora must state which versions
they target. A future reference may describe historical behavior only in
clearly labeled version notes or migration sections.

## Implementation Notes

Implementation notes are allowed when the stage-0 toolchain has a temporary
limitation, optimization, or host-language behavior that affects users.

An implementation note must:

- name the component it describes
- say whether the behavior is temporary or required
- link to the normative rule it implements or falls short of
- avoid defining new language behavior

If implementation behavior and normative AIL semantics conflict, the
normative rule wins and the implementation is non-conformant.

## Conformance Links

Each normative rule should identify at least one of:

- parser acceptance test
- schema validation test
- checker diagnostic
- runtime trace fixture
- round-trip equivalence fixture
- prompt portability fixture
- backend conformance fixture

Released diagnostics must cite the primary rule they enforce. During stage-0,
the diagnostics catalog is the rule-to-diagnostic map; adding a structured
`rule_id` diagnostic field is part of the released-reference conformance work.
Tests should prefer the same vocabulary as the rule identifier so failures are
easy for humans and AI agents to map back to the reference.

## Reference Maintenance Checklist

Before adding or changing a language rule, confirm:

- the authority level is clear
- the affected layer is named
- AIL-Spec, AIL-Core, and projection behavior remain aligned
- the AI agent is not made trusted by implication
- the checker or verifier has an enforceable obligation
- a diagnostic or conformance fixture exists or is explicitly planned
- version and migration impact are recorded
