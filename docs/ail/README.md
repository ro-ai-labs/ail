# AIL Specification

AIL means Agentic Intent Language.

AIL is a semantic programming language and toolchain for humans and AI agents.
Humans begin in English, AI agents help clarify and structure intent, the
toolchain normalizes accepted programs into a canonical semantic IR, and every
accepted program can render back into structured English, no-code views, traces,
and low-level explanations.

## Read Order

1. `00-foundation.md`
2. `01-language-architecture.md`
3. `02-structured-spec.md`
4. `03-semantic-ir.md`
5. `04-no-code-views.md`
6. `05-agent-protocol.md`
7. `06-agent-tools.md`
8. `07-types-values-effects.md`
9. `08-failures-guarantees-traces.md`
10. `09-system-profile.md`
11. `10-meta-profile.md`
12. `11-round-trip-equivalence.md`
13. `12-training-corpus.md`
14. `13-bootstrap-self-hosting.md`
15. `14-evolution-protocol.md`
16. `15-toolchain-implementation-guide.md`
17. `16-implementation-readiness-checklist.md`

## Status

These documents define the first AIL specification suite. They are precise
enough to guide implementation, but still versioned and expected to evolve as
examples, round-trip tests, no-code projections, and compiler prototypes expose
gaps.

## Specification Contract

The active AIL contract is:

```text
human English
  -> AI-assisted interview
  -> AIL-Spec structured English
  -> AIL-Core canonical semantic graph
  -> checked program artifact
  -> AIL-Bytecode executable artifact and projections
```

The compiler accepts checked deterministic artifacts, not free-form
conversation. The AI Agent may draft and explain those artifacts, but the
trusted checker is the authority for acceptance. Bootstrap compiler code may be
hosted in Rust, but accepted AIL programs lower to AIL-owned bytecode rather
than generated host-language source.

## Examples

- `examples/support-ticket.ail-spec.md`
- `examples/support-ticket.ail-core.md`
- `../../examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- `examples/refund-tool.ail-spec.md`
- `examples/refund-tool.ail-core.md`
- `examples/compiler-pass.ail-spec.md`
- `examples/compiler-pass.ail-core.md`
- `../../examples/network_driver.ail/spec.ail-spec.md`

## Implementation Start

Development should begin with `15-toolchain-implementation-guide.md`, using the
Support Ticket example as the first vertical slice. Use
`16-implementation-readiness-checklist.md` as the gate before claiming the
specification suite is ready for compiler and runtime implementation.

## Prototype History

This repository previously explored the language under the EIGL name. New
language design uses AIL names. Existing EIGL prototype code and examples remain
historical implementation scaffolding until migration is planned explicitly.
