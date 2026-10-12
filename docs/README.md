# AIL Documentation

AIL means Agentic Intent Language.

This directory is the active documentation root for the language, compiler, VM,
native Linux ELF backend, LLM-assisted agent workflow, and conformance suite.

## Read Order

1. [AIL Specification](ail/README.md)
2. [AIL Desired Outcome Traceability](ail/27-desired-outcome-traceability.md)
3. [AIL Implementation Readiness Checklist](ail/16-implementation-readiness-checklist.md)
4. [AIL First Version Completion Gate](ail/29-first-version-completion-gate.md)
5. [AIL Next Version Completion Gate](ail/30-next-version-completion-gate.md)

## Current Direction

AIL lets humans begin in English while an AI agent asks clarifying questions,
captures sufficient requirements, drafts deterministic AIL-Spec, normalizes the
accepted program into AIL-Core IR, and invokes the compiler pipeline. Checked
Core now passes through [executable AIL-STE](ail/33-ste-intermediate.md), a
programming profile based on ASD-STE100, before bytecode and binary emission.

Checked AIL artifacts can render back into structured English, no-code views,
diagnostics, traces, manifests, fingerprints, AIL bytecode, and low-level
explanations. For Linux, the executable machine-level bytecode target is a
native ELF64 x86_64 executable emitted by the compiler.

## Active Specification Suite

The [ail](ail) directory defines the current language framework: foundation,
architecture, structured English, semantic IR, no-code views, agent protocol,
agent tools, type/effect model, failure and trace model, systems profile,
meta/compiler profile, round-trip equivalence, training corpus, bootstrap
self-hosting path, language evolution protocol, toolchain implementation guide,
implementation-readiness checklist, formal execution semantics, AIL-Core schema,
agent prompt pack, standard library and package model, C interop and ABI
profile, backend portability, UI profile, diagnostic catalog, example
inventory, semantic safety model, desired-outcome traceability matrix, and the
first-version and next-version completion gates.

- [`34-integer-expressions.md`](ail/34-integer-expressions.md): typed integer computation, ordered calls, result selection, and current capability limits.
- [`35-control-flow.md`](ail/35-control-flow.md): typed comparisons, source branches and loops, Core validation and per-action execution limits.

- [`36-integer-functions.md`](ail/36-integer-functions.md): named arguments, isolated frames, recursion and early returns through STE and native code.
- [`37-text-operations.md`](ail/37-text-operations.md): executable Unicode text processing through Core, STE, VM and native ELF.
- [`38-typed-lists.md`](ail/38-typed-lists.md): integer and text lists with immutable updates, JSON transport and shared runtime limits.
- [`39-typed-literals.md`](ail/39-typed-literals.md): checked source constants for integers, booleans, Unicode text and flat lists.
- [`40-file-resources.md`](ail/40-file-resources.md): UTF-8 regular file reads and writes with explicit caller bindings in the VM and native ELF.
- [`41-json-conversion.md`](ail/41-json-conversion.md): typed JSON parse/format linking text files to scalar and list computations.
- [`42-recursive-json.md`](ail/42-recursive-json.md): nested values, lossless number transport, typed JSON Pointer lookup and collection counts.
- [`43-json-mutation.md`](ail/43-json-mutation.md): immutable JSON set/remove, typed replacements, array append and complete report construction.

- [`44-value-functions.md`](ail/44-value-functions.md): typed reusable functions for scalars, Unicode text, lists and recursive JSON.
- [`45-float-computation.md`](ail/45-float-computation.md): binary64 arithmetic, explicit conversions, numeric functions and JSON measurements.
- [`46-floating-lists.md`](ail/46-floating-lists.md): floating collections, reusable numerical functions, finite JSON boundaries and a statistics file report.
- [`47-json-inspection.md`](ail/47-json-inspection.md): discover JSON kinds and ordered keys, then recursively traverse unknown records.
- [`48-boolean-predicates.md`](ail/48-boolean-predicates.md): lazy Boolean logic, typed scalar comparisons and reusable record-filter predicates.
- [`49-text-transformations.md`](ail/49-text-transformations.md): split, list join, replace and Unicode trim with a reusable TSV file report.
- [`50-list-transformations.md`](ail/50-list-transformations.md): slicing, concatenation, reversal and stable sorting with a file statistics report.
- [`51-runtime-recovery.md`](ail/51-runtime-recovery.md): explicit data-error handlers, call unwinding and a recovering JSON-lines batch report.
- [`52-standard-streams.md`](ail/52-standard-streams.md): bounded stdin and exact stdout/stderr for standalone command-line filters.
- [`53-process-resources.md`](ail/53-process-resources.md): caller-bound tools, separate captures, exit status, timeouts and an AIL tool coordinator.

- [`54-directory-resources.md`](ail/54-directory-resources.md): caller-bound roots, sorted entry discovery, relative text files and a batch report manifest.

- [`56-bytes.md`](ail/56-bytes.md): arbitrary octets, explicit text conversion, reusable byte functions and binary fixed-file access.

- [`57-explicit-errors.md`](ail/57-explicit-errors.md): deliberate domain failures through existing structured recovery.
- [`58-run-length-codec.md`](ail/58-run-length-codec.md): a reusable pure AIL binary codec and native file tool.
- [`59-executable-variants.md`](ail/59-executable-variants.md): recursive custom values, concrete generic types and a pure AIL binary tree.

- [Trustworthy edits](ail/60-trustworthy-edits.md): preserve-by-default authoring, explicit expected targets, structural proof limits and independent edit verification; this authoring-correctness milestone adds no ladder rung.

- [CSV aggregation](ail/61-csv-aggregation.md): pure CSV parsing and checked category totals, JSON-lines requests, caller-granted file reports and independent VM/native acceptance.

- [Word frequency](ail/62-word-frequency.md): exact words, deterministic top-five ranking, bounded raw stdin and independent VM/native acceptance.

- [JSON pretty printing](ail/63-json-pretty.md): pure lossless layout, request errors, Bytes transport and bounded raw stdin.

- [LRU cache](ail/64-lru-cache.md): immutable pure cache state, exact events and bounded raw stdin.
