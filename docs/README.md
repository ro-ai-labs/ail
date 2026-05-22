# AIL Documentation

AIL means Agentic Intent Language.

AIL is the new language direction for this repository. It replaces the earlier
EIGL naming and reframes the work around a semantic programming language and
toolchain for humans and AI agents.

The active specification starts here:

1. [AIL Specification](ail/README.md)
2. [AIL Language Foundation Design](superpowers/specs/2026-05-22-ail-language-foundation-design.md)
3. [AIL Language Specification Work Plan](superpowers/plans/2026-05-22-ail-language-specification-work.md)

## Current Direction

AIL is intended to let humans begin in English, let an AI Agent help clarify and
structure intent, normalize accepted programs into a canonical semantic IR, and
render that IR back into structured English, no-code views, traces, and
low-level explanations.

The long-term goal is a self-sovereign AIL toolchain: legacy languages may
bootstrap early compilers, but the required compiler, runtime, standard library,
agent protocol, debugger, package system, and build system should eventually be
defined in AIL itself.

## Active Specification Suite

The `ail/` directory defines the current AIL language framework: foundation,
architecture, structured English, semantic IR, no-code views, agent protocol,
agent tools, type/effect model, failure and trace model, systems profile,
meta/compiler profile, round-trip equivalence, training corpus, bootstrap
self-hosting path, language evolution protocol, first toolchain implementation
guide, and implementation-readiness checklist.

## Historical Material

Earlier documents used the EIGL name and describe a prototype direction that no
longer represents the active language definition. They are archived only for
provenance:

- [Archived EIGL Prototype Docs](archive/eigl-prototype/)

Do not use archived EIGL documents as current AIL specification authority.
