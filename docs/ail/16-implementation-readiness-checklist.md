# AIL Implementation Readiness Checklist

## Purpose

This checklist is the acceptance gate for starting toolchain development. It
does not claim the language is frozen. It verifies that the current
specification suite is complete enough for an implementation team to begin the
first vertical slice.

## Required Documentation

The documentation set is implementation-ready when these artifacts exist and
are internally consistent:

- `00-foundation.md`: language purpose, invariants, layers, profiles, and
  self-sovereign direction
- `01-language-architecture.md`: pipeline, source of truth, projections, trust
  boundary, compiler boundary, and runtime boundary
- `02-structured-spec.md`: deterministic English shape and required semantic
  slots
- `03-semantic-ir.md`: graph model, stable identity, node kinds, edge kinds,
  provenance, normalization, and serialization expectations
- `04-no-code-views.md`: view types and graph-patch editing model
- `05-agent-protocol.md`: agent responsibilities, interview loop, patch
  discipline, prompt compatibility, and trust boundary
- `06-agent-tools.md`: tool contract, permissions, capabilities, approvals,
  secrets, effects, audit, and enforcement
- `07-types-values-effects.md`: core types, structured values, permissions,
  capabilities, effects, ownership, borrowing, and explanation rules
- `08-failures-guarantees-traces.md`: declared failures, compensation,
  guarantees, trace events, and debugging
- `09-system-profile.md`: low-level scope, ownership, borrowing, regions,
  layout, allocation placement, interrupt context constraints, interrupt
  priority declarations, interrupt mask declarations, scheduler task
  declarations, scheduler task priority declarations, scheduler task timing
  declarations, lock guard declarations, scheduling, device capabilities, and
  lowering obligations
- `10-meta-profile.md`: language definition packages, compiler passes, checker
  rules, diagnostics, renderers, prompts, lowering, and self-hosting role
- `11-round-trip-equivalence.md`: strong, behavioral, and explanation
  equivalence
- `12-training-corpus.md`: examples, invalid examples, diagnostics, traces,
  human review data, and conformance suite
- `13-bootstrap-self-hosting.md`: bootstrap allowance, staged self-hosting, and
  fixed-point checks
- `14-evolution-protocol.md`: stable invariants, versioned decisions,
  experiments, proposal requirements, and acceptance gates
- `15-toolchain-implementation-guide.md`: first vertical slice, component
  boundaries, artifact format, development sequence, and slice completion gate

## Examples Required For Development Start

The first implementation must have at least these paired examples:

- Support Ticket AIL-Spec and AIL-Core for an application program
- Refund Tool AIL-Spec and AIL-Core for an agent tool
- Compiler Pass AIL-Spec and AIL-Core for AIL-Meta
- Network Driver AIL-Spec package and conformance fixtures for AIL-System
  resources, capabilities, ownership, borrowing, mutable borrowing,
  move semantics, ABI layout, allocation placement, interrupt context,
  interrupt priority, interrupt masks, scheduler tasks, scheduler task
  priorities, scheduler task timings, lock guards, borrow-checking,
  lifetime-checking, and regions

The Support Ticket pair is the first executable conformance target.

## Validation Commands

Run these checks before claiming the documentation is ready for implementation:

```bash
find docs -maxdepth 1 -type f | sort
find docs/ail -maxdepth 2 -type f | sort
rg -n "TB[D]|TO[D]O|FIXM[E]|implement late[r]|fill in detail[s]" docs/ail docs/superpowers/specs README.md docs/README.md
rg -n "English starts|canonical semantic|AI Agent|Round-Trip|Self-Sovereign|Readability Gate|LLM" docs/ail
rg -n "Package Loader|AIL-Spec Parser|Elaborator|AIL-Core Store|Checker|Renderer|Trace Runtime|Diagnostics" docs/ail/15-toolchain-implementation-guide.md
git diff --check -- README.md docs/README.md docs/ail docs/superpowers/specs docs/superpowers/plans
```

The placeholder scan should return no matches. `rg` exits with code 1 when it
finds no matches; that is the expected result for the placeholder scan.

## Manual Review Gate

Before starting implementation, review the active spec suite and confirm:

- English is the first authoring surface, but not the compiled artifact.
- AIL-Core is the accepted source of truth.
- The AI Agent is official but untrusted.
- Every executable behavior can trace to human-reviewed structured English.
- No-code views edit through graph patches.
- Secret, permission, effect, failure, guarantee, and trace semantics are
  visible in both spec and IR examples.
- The Support Ticket example can drive the first parser, checker, renderer,
  equivalence, and runtime tests.
- Long-term systems, meta, training, and self-hosting requirements remain
  represented even though they are not in the first vertical slice.

## Development Start Decision

Development may start when:

- all required documentation artifacts exist
- the active examples exist
- validation commands pass with the expected results
- manual review finds no contradiction that blocks the first vertical slice
- any remaining broad language choices are recorded as later evolution
  decisions rather than hidden assumptions

If one of these conditions fails, update the relevant spec before implementing
compiler code.
