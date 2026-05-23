# AIL Specification

AIL means Agentic Intent Language.

AIL is a semantic programming language and toolchain for humans and AI agents.
Humans begin in English, AI agents help clarify and structure intent, the
toolchain normalizes accepted programs into canonical AIL-Core IR, and checked
artifacts can render back into structured English, no-code views, diagnostics,
traces, bytecode, native executables, and low-level explanations.

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

## Specification Contract

```text
human English
  -> AI-assisted interview
  -> AIL-Requirements
  -> AIL-Spec structured English
  -> AIL-Core canonical semantic graph
  -> checked program artifact
  -> AIL bytecode, native Linux ELF, and projections
```

The compiler accepts checked deterministic artifacts, not free-form
conversation. The AI agent may draft, repair, and explain those artifacts, but
the trusted checker is the authority for acceptance.

## Examples

- `examples/support-ticket.ail-spec.md`
- `examples/support-ticket.ail-core.md`
- `../../examples/support_ticket.ail/spec.ail-spec.md`
- `../../examples/ail_toolchain_agent.ail/spec.ail-spec.md`
- `examples/refund-tool.ail-spec.md`
- `examples/refund-tool.ail-core.md`
- `examples/compiler-pass.ail-spec.md`
- `examples/compiler-pass.ail-core.md`
- `../../examples/network_driver.ail/spec.ail-spec.md`

## Implementation Start

Use `15-toolchain-implementation-guide.md` as the implementation reference and
`16-implementation-readiness-checklist.md` as the readiness gate. The first
vertical slice is the support-ticket package, followed by agent-tool, systems,
compiler-pass, conformance, and native Linux ELF workflows.
