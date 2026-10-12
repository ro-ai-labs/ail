# Executable STE intermediate for AIL

The continuing user objective is Specs -> ASD-STE100 -> Binary, followed by a
language capable of replacing Python and usable by AI agents. Neither an English
report nor a passing artifact audit meets that objective.

## Pipeline

Use `AIL-Spec -> checked AIL-Core -> AIL-STE -> verified AIL bytecode -> ELF/VM`.
Core stays before STE to preserve the existing type, permission, and package
checks. The common Core compiler must emit and parse STE; emitting an optional
sidecar is insufficient. Existing instruction records can be used internally
while constructing STE, but the parser output is the bytecode used downstream.

AIL-STE version 1 is an executable programming profile based on ASD-STE100
Issue 9 procedural writing. Each instruction has one fixed imperative template.
There is no synonym inference, LLM dependency, or fallback to opaque opcodes.
Templates have at most 20 words, excluding separately declared technical data.
Some existing operations only record requirements or function descriptions;
their templates must say "Record" and must not imply execution or enforcement.

ASD-STE100 is a technical writing standard, not programming semantics. This
implementation must state its limits: the grammar enforces its chosen templates,
not all 53 writing rules or the full dictionary. No certification claim is made.
Primary reference: https://www.asd-ste100.org/assets/files/ASD-STE100_ISSUE9.pdf

## Format and interfaces

The header is `AIL-STE 1`. A metadata declaration retains package identity,
version, profile, capabilities, targets, bindings, and failures. Metadata cannot
contain executable actions. Technical data are JSON-quoted strings, declared
under stable term identifiers. Instructions reference these identifiers, so
arbitrary text cannot inject an instruction or expand a procedural sentence.

Each `Procedure` has ordered instructions and an explicit end. Templates bind
named semantic operands. Optional instruction attributes remain explicit data.
Every nonblank line must parse. Reject duplicate declarations, unresolved terms,
unknown instructions, missing operands, invalid references, and malformed data
with `AIL-STE-001` and source line context. Preserve all compiler-produced
metadata and semantics during round trip.

Expose library rendering/parsing and `compile_ail_core_ste`. Add `ail-ste` for
packages and saved Core. Accept `.ail-ste` in `ail-check`, `ail-lower`, `ail-run`,
`ail-vm`, and `ail-compile`. Keep bytecode compilation and target checks shared.
Write `artifact.ail-ste` alongside bytecode in compilation artifact bundles.
Provide agent instructions and a runnable example.

## Verification and remaining objective

Test independently authored STE, a changed arithmetic operand, malformed input,
all checked example packages, and VM/native agreement. Run formatting, checking,
the full Rust suite, Clippy, and the frozen computation benchmark. Existing
benchmark oracles and inputs must not be weakened.

This is a first architectural step. Variable arithmetic, typed runtime values,
real functions and loops, collections, file/network/process I/O, modules, usable
agent diagnostics, and independently executed Python comparison workloads remain
required before claiming Python replacement. The current locked score is 1/16.
