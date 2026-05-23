# AIL Bootstrap And Self-Hosting

## Purpose

AIL bootstrap and self-hosting rules describe how the language moves from the
current prototype to a self-sovereign toolchain.

## Bootstrap Allowance

Rust, C, C++, Python, JavaScript, TypeScript, Go, LLVM, Wasm, and other systems
may be used as scaffolding. The rule is that legacy languages may bootstrap AIL but must not own AIL.

## Self-Sovereign Toolchain Principle

The mature AIL toolchain must define its required compiler, runtime, standard
library, package system, debugger, agent protocol, build system, and conformance
suite in AIL itself.

## Stage 0: Bootstrap Prototype

Use the current prototype to explore parsing, checking, semantic graph
construction, views, traces, execution, and round-trip tests.

## Stage 1: AIL Foundation Specs

Define the AIL foundation, structured spec, semantic IR, no-code views, agent
protocol, tool model, types, effects, failures, traces, systems profile, meta
profile, equivalence model, training corpus, and evolution protocol.

## Stage 2: AIL-Defined Compiler Rules

Represent parser rules, checker rules, diagnostics, renderers, examples,
round-trip rules, lowering obligations, optimizer rules, and package metadata
in AIL-Meta.

## Stage 3: Generated AIL Compiler

Use the bootstrap compiler to compile AIL-defined compiler rules into a new
compiler artifact. The generated compiler must pass the conformance suite
before it can be trusted for later stages.
Native target generation is part of this artifact boundary: for Linux, the
reviewable output is deterministic ELF executable bytes plus fingerprints and
AIL-authored agent traces, not generated Rust, C, or other host-language backend
source. The bootstrap artifact set should bundle the AIL-authored toolchain
agent and AIL-Meta compiler passes as source package snapshots, checked
AIL-Core IR, AIL-Meta compiler-pass output IR and trace, checked AIL-Bytecode,
native machine-code artifacts, a native-bytecode report proving the Linux
outputs are ELF64 x86_64 executable bytes, package conformance reports, and a
fixed-point report proving the compiler-pass output is stable on a second pass,
a host-boundary report proving no Rust, C, Python, JavaScript, or other
host-language backend source was generated, and a dependency report proving the
emitted Linux ELF artifacts use no dynamic linker, shared libraries,
host-language runtime, or linker invocation, plus a handoff report proving
representative generated native AIL toolchain actions and AIL-Meta compiler
passes execute through the Linux argv ABI, with an AIL-authored verifier
accepting the manifest.

## Stage 4: Self-Hosted Fixed Point

Compiler N compiles the AIL toolchain spec into Compiler N+1. Compiler N+1
compiles the same spec into Compiler N+2. The outputs are equivalent under the
defined fixed-point check.

## Stage 5: Legacy Independence

AIL can rebuild its required compiler, runtime, standard library, agent tooling,
package system, and build system from AIL sources. Legacy artifacts may remain
as optional backends, historical bootstrap artifacts, or interoperability
targets.

## Fixed-Point Checks

Fixed-Point Checks compare generated compiler outputs, diagnostics,
conformance behavior, package hashes, runtime traces, and self-explanations.
The check fails if the next compiler changes accepted language semantics without
an approved evolution proposal.
