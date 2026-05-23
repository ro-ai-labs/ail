# AIL Toolchain Implementation Guide

## Purpose

This guide defines the first implementation target for the AIL toolchain. It
turns the language specification into a concrete build sequence without
requiring the first compiler to solve every future AIL profile.

The first implementation proves the central loop:

```text
AIL-Spec structured English
  -> AIL-Core semantic graph
  -> checker
  -> canonical render
  -> traceable execution for a small application slice
```

## Implementation Principles

The first toolchain must preserve these rules:

- English is the authoring surface, but checked AIL-Core is the compiler input.
- The AI Agent can draft artifacts, but the checker decides acceptance.
- AIL-Spec, AIL-Core, no-code views, diagnostics, and traces are projections of
  the same accepted semantics.
- Round-trip checks use normalized AIL-Core equivalence.
- Every accepted behavior has provenance back to human-reviewed structured
  English.
- The first slice may be small, but it must be shaped like the final language.

## First Vertical Slice

The first implementation slice should support the Support Ticket example:

- package metadata
- one application
- application users
- things and fields
- scalar types, `State`, `Option<T>`, `List<T>`, and `Secret<T>`
- actions with requirements, reads, writes, failures, guarantees, and trace
  events
- one view projection
- one runtime execution path
- one failed execution path
- canonical render/reparse equivalence

This is intentionally narrow. It exercises the architecture without forcing
systems programming, self-hosting, package distribution, or full no-code editing
into the first milestone.

## Implementation Phase Map

The guide is split into phases so the first vertical slice remains narrow while
the language definition covers the complete desired outcome:

1. Semantic MVP: package loading, canonical AIL-Spec parsing, AIL-Core schema,
   checker, renderer, diagnostics, round-trip tests, and one trace runtime.
2. Turing Core: control flow, functions, action calls, recursion or loops,
   collections, state mutation, and operational semantics.
3. Agent Prompt Pack: interview, requirements capture, spec drafting, repair,
   IR rendering, trace explanation, patch prompts, and portability tests.
4. Visual Review And Patch: AIL-Flow block/card views, graph patch validation,
   patch round trips, and low-code edit fixtures.
5. Standard Library And Packages: core packages, imports, versioning,
   capability grants, and package projections.
6. Bytecode And VM: checked AIL-Core to AIL-Bytecode, bytecode verifier, VM
   runtime, and bytecode artifact boundary.
7. Native Target 1: Linux x86_64 ELF with backend conformance manifest.
8. C Interop: ABI-safe imports, pointer ownership, failure mapping, callbacks,
   and traceable foreign calls.
9. UI Profile: routes, forms, components, events, accessibility, UI state, and
   UI traces.
10. AIL-Meta Self-Hosting: parser, checker, renderer, diagnostic, lowering,
    and conformance rules as AIL-Meta packages.
11. Self-Hosted Fixed Point: bootstrap compiler generates the next compiler
    from AIL-defined toolchain sources and proves fixed-point equivalence.

The next implemented profile expansion is the Refund Tool `AgentTool` example.
It proves that the same package loader, parser, AIL-Core store, checker,
renderer, AIL-Flow projection, and LLM draft loop can represent an agent tool
contract with typed inputs and outputs, requirements, reads, writes, external
calls, secret protection, failures, guarantees, and provenance.

The AIL Toolchain Agent Application example is the first AIL-authored toolchain
agent. It models the agent that interviews an application developer, captures
requirements, turns checked specs into AIL-Core IR, compiles verified
AIL-Bytecode, and compares prompt portability from a named base model to a
target LLM. Because it is
an Application-profile package, it lowers through the same AIL-Bytecode
compiler as user applications instead of living as host-language code.

The following implemented profile expansion is the AIL-Meta Compiler Pass
example. It proves that compiler passes can be authored as structured English,
lowered into AIL-Core as compiler-pass actions with typed values, steps,
effects, failures, guarantees, traces, and provenance, rendered back to
AIL-Spec, taught to the LLM draft loop through a profile-specific prompt, and
executed as bytecode against checked AIL-Core for the first concrete
IR-transforming pass.

The next implemented profile expansion is the Network Driver `System` example.
It proves that AIL-System can describe a low-level component with typed
resources, explicit capabilities, effects, guarantees, traces, AIL-Flow
projection, allocation placement, interrupt context constraints, conformance
fixtures, interrupt priority declarations, and a profile-specific LLM draft
prompt. Interrupt-mask, scheduler task, task-priority, task-timing, and lock-guard
declarations are the next expansion of the same System-profile projection path.

## Toolchain Components

### Package Loader

The package loader reads an AIL package directory, validates required files,
assigns package identity, and passes source artifacts to the parser.

Minimum package layout:

```text
package.ail/
  ail-package.md
  spec.ail-spec.md
  examples/
    accepted/
    rejected/
```

`ail-package.md` declares package name, version, profile, entry spec, required
features, imports, and conformance level. Imports use
`imports: <path> as <Alias>` entries. The alias is a namespace boundary for the
imported fragment before checking and rendering.

Conformance checks must validate the entry spec, every accepted fixture, and
every rejected fixture. Accepted fixtures pass only when they produce no checker
diagnostics; rejected fixtures pass only when they produce at least one stable
diagnostic. With `--artifact-dir`, `ail-conformance` writes
`conformance-report.txt`, `conformance-report.fingerprint.txt`,
`manifest.ail-conformance.txt`, and `manifest.fingerprint.txt`, so the
conformance gate produces deterministic audit artifacts instead of only
host-side stdout. With `--agent <agent-package-or-bytecode> --artifact-dir`,
the gate must compile or load an AIL-authored Application agent and run its
`VerifyConformanceManifest` bytecode action against the conformance report,
report fingerprint, manifest, manifest fingerprint, and native-bytecode and
dependency report fingerprints when native verifier agents are emitted before
writing `agent.ailbc.json`, `agent.fingerprint.txt`, and `agent-trace.txt`. With
`--target linux-x86_64-elf`, the conformance gate must also emit
`agent-<ActionName>.elf` for each AIL-authored agent action and record every
native verifier executable as an `agent-target` entry in
`manifest.ail-conformance.txt`. That native conformance-agent run also writes
`native-bytecode-report.txt`, `native-bytecode-report.fingerprint.txt`,
`dependency-report.txt`, and `dependency-report.fingerprint.txt`; the reports
prove the verifier-agent ELFs are ELF64 little-endian x86_64 executable bytes
and record `host-language-runtime none`, `dynamic-linker none`,
`shared-libraries none`, `library-dependencies none`, and `linker-invocation
none`.

### AIL-Spec Parser

The parser reads structured English sections and produces a draft semantic
document. It should accept regular headings and bullet forms before attempting
broader English variation.

Minimum parser responsibilities:

- application purpose
- application users
- tool declarations
- tool inputs and outputs
- compiler pass declarations
- compiler pass inputs, outputs, steps, reads, writes, guarantees, failures,
  and trace bullets
- system component declarations
- system component resources, ownership, borrowing, mutable borrowing, region
  placement, ABI layout, allocation placement, execution context,
  interrupt priority, interrupt mask, scheduler task, scheduler task priority,
  scheduler task timing, lock guard, capabilities, effects, guarantees, and
  trace bullets
- thing declarations
- field declarations
- action declarations
- tool requirements, reads, writes, calls, secret protections, guarantees, and
  trace bullets
- requirement bullets
- read/write/effect bullets
- stage-0 action call bullets as auditable write/effect text
- failure bullets
- guarantee bullets
- trace bullets
- provenance IDs for each parsed paragraph or bullet

The parser does not decide whether the program is valid.

### Elaborator

The elaborator converts the parsed AIL-Spec document into candidate AIL-Core.
It resolves names, creates stable IDs, expands defaults, links actions to
fields, and turns bullets into graph nodes and edges.

The elaborator must preserve unresolved questions instead of guessing.
Behavior bullets that become rules, effects, guarantees, traces, or failure
handling nodes must keep provenance back to the source bullet.

### AIL-Core Store

The core store holds normalized graph nodes, edges, attributes, provenance, and
package metadata.

Minimum APIs:

- add node
- add edge
- attach attribute
- attach provenance
- normalize graph
- compare normalized graph equivalence
- render deterministic text serialization

### Checker

The checker validates candidate AIL-Core before acceptance.

Minimum checks:

- every referenced thing, field, action, rule, failure, guarantee, and trace
  exists
- field, tool input, tool output, and compiler-pass value types are known
  scalar, state, generic wrapper, profile built-in, or declared object types
- action reads and writes target declared fields
- requirements reference declared values and fields
- secret fields and agent-tool outputs do not flow into public outputs, and
  secret reads/writes require explicit protection rules
- declared failures have handling and trace coverage
- agent tools that mention permission have an explicit permission rule
- agent tools that mention approval have an explicit approval rule
- agent tools with executable behavior have an explicit audit trace event
- system components that perform effects have an explicit capability
- system component effects that name resources resolve to declared component
  resources
- system component effects that target device resources are authorized by a
  matching capability for the same resource
- system component mutable effects target resources owned or mutably borrowed
  by the component
- system component read effects that target non-device resources target
  resources owned, borrowed, or mutably borrowed by the component
- system component effects that target non-device resources target resources
  placed in a region
- system component layout declarations target declared component resources
- system component allocation declarations target declared component resources
- system component lock guard declarations target declared protected resources
  and declared lock resources
- system components that run in interrupt context do not perform blocking
  effects
- system component interrupt priority declarations target declared execution
  contexts
- system component interrupt mask declarations target declared execution
  contexts
- system component scheduler task declarations target declared execution
  contexts
- system component scheduler task priority declarations target declared
  scheduler tasks
- system component scheduler task timing declarations target declared
  scheduler tasks
- system component mutable effects do not target resources currently declared
  as borrowed by that component
- system component move effects target resources owned by that component
- system component resources are not declared as both borrowed and mutably
  borrowed by that component
- system component effects do not target resources after a previous release or
  free effect has ended that resource lifetime
- system component effects do not target resources after a previous move effect
  has transferred that resource out of the component
- tool-owned rules, effects, calls, guarantees, traces, inputs, outputs, and
  secrets are attached to their declaring tool
- compiler-pass values, steps, reads, writes, failures, guarantees, and traces
  are attached to their declaring compiler pass
- system-component resources, ownership, borrowing, mutable borrowing,
  capabilities, effects, guarantees, and traces are attached to their
  declaring system component, and effects that name resources are linked to
  those resources
- system-component capabilities that name resources are linked to those
  resources
- system-component ownership declarations are linked to declared resources
- system-component borrowing declarations are linked to declared resources
- system-component mutable borrowing declarations are linked to declared
  resources
- system-component region placement declarations are linked to declared
  resources and region nodes
- system-component ABI layout declarations are linked to declared resources and
  layout nodes
- system-component allocation placement declarations are linked to declared
  resources and allocation nodes
- system-component lock guard declarations are linked to declared protected
  resources, declared lock resources, and lock-guard nodes
- system-component execution context declarations are linked to execution
  context nodes
- system-component interrupt priority declarations are linked to declared
  execution contexts and interrupt-priority nodes
- system-component interrupt mask declarations are linked to declared
  execution contexts and interrupt-mask nodes
- system-component scheduler task declarations are linked to declared
  execution contexts and scheduler-task nodes
- system-component scheduler task priority declarations are linked to declared
  scheduler-task nodes and scheduler-task-priority nodes
- system-component scheduler task timing declarations are linked to declared
  scheduler-task nodes and scheduler-task-timing nodes
- system-component borrow-checker diagnostics reject mutable effects against
  borrowed resources
- system-component lifetime diagnostics reject use after release or move
- guarantees attach to actions or tools
- trace events cover action start, rule checks, writes, failures, and
  guarantees
- provenance exists for accepted semantic nodes, including generated helper
  nodes

### Renderer

The renderer produces deterministic AIL-Spec and readable AIL-Core text from
checked AIL-Core. The first renderer can be conservative and regular; it does
not need to imitate the original prose.

The renderer is correct only if rendering and reparsing return an equivalent
normalized graph.

### Trace Runtime

The first runtime executes a checked action against simple state and returns:

- final state
- named failure or success
- semantic trace events
- guarantee results
- human-readable explanation

The runtime must enforce checked permissions and secret redaction for the slice
it supports.

### Native And VM Compiler

Checked AIL-Core now has two compiler targets. The current `ail-lower` target
is a deterministic AIL VM instruction artifact used for inspection, agent
acceptance checkpoints, compiler-pass execution, and bootstrap runtime tests.
The native target used by `ail-compile` and `ail-build --target
linux-x86_64-elf` is the machine-code path: it emits Linux x86_64 ELF
executable bytes directly from the bootstrap compiler, without generating Rust,
invoking a linker, using libc, or relying on LLVM.

The native executable path is the bytecode target in the machine-level sense.
The VM instruction artifact remains an intermediate representation until the
native compiler covers the full action ABI, runtime state, traces,
requirements, and field writes.
Every native-bytecode report begins with `bytecode-level machine`,
`bytecode-container linux-elf-executable`, and
`bytecode-format elf64-little-x86_64-executable` for the Linux target. That
report-level contract is mirrored in native manifests as
`machine-bytecode-contract linux-x86_64-elf ...`, keeping "bytecode" anchored
to machine-level ELF executables at both the report and manifest boundaries,
while AIL-Bytecode JSON remains the auditable compiler intermediate used for
checking, VM execution, and bootstrap handoff. The AIL Toolchain Agent declares
`BuildRequest.machine bytecode contract`; native-relevant manifest verifier
actions require, validate, and read that field before accepting the artifact
set. The bootstrap host supplies `none` for non-native verifier paths and the
concrete Linux ELF contract for native paths.

The first VM instruction compiler supports Application-profile actions,
AgentTool-profile tool declarations, Compiler-profile compiler passes, and
System-profile components. For applications it emits:

- package metadata and profile identity
- one bytecode action per checked AIL action
- requirement opcodes for existence checks, positive field requirements, and
  negative field requirements
- read, write, field-set, effect, guarantee, trace, and return opcodes
- declared failure trace tables

For compiler packages it emits one bytecode action per checked compiler pass,
including pass metadata, input and output declarations, read, step, write,
explicit IR-transform, guarantee, trace, and return opcodes. This keeps
AIL-Meta compiler work on the same bytecode path as applications instead of
generating Rust or another host backend. The bootstrap runner executes
`InferReadPermissions` only through the AIL-authored
`CORE_INFER_READ_PERMISSIONS` bytecode primitive, adding candidate read
`Permission` nodes with compiler-pass provenance when that opcode is present.

For agent-tool packages it emits one bytecode action per checked tool,
including tool metadata, requirements, typed inputs and outputs with secret
markers, reads, calls, writes, permissions, approvals, secret protections,
guarantees, traces, and return opcodes. This lets developer-facing agents call
auditable AIL-authored tools as bytecode artifacts instead of host-language
plugins.

For system packages it emits one bytecode action per checked component,
including component metadata, resources, ownership and borrow relations,
regions, layouts, allocations, lock guards, execution contexts, interrupt
configuration, scheduler tasks, capabilities, effects, guarantees, traces, and
return opcodes. This keeps low-level toolchain/runtime components in the same
verified bytecode artifact family as applications, agent tools, and compiler
passes.

`ail-lower` renders the deterministic VM instruction artifact after the same
package loading, parsing, elaboration, and checker gate as `ail-core` and
`ail-flow`; the VM instruction compiler receives the checked AIL-Core IR, not
the parsed AIL-Spec document. `ail-run` uses the same checked AIL-Core-to-VM
instruction path and then executes through the AIL VM for supported Application
packages.
The public AIL-Core compiler entry points also enforce this boundary. Calling
the bytecode or native compiler API with unchecked AIL-Core returns checker
diagnostics instead of producing an artifact, even when the caller is not using
the CLI path.
`ail-compile --target linux-x86_64-elf --out <path>` uses the same checked
AIL-Core gate, validates the selected action, and writes a native executable
ELF file with direct Linux x86_64 syscall code as the first native compiler
slice. The first native runtime ABI receives state as `key=value` argv entries.
`ail-build --target linux-x86_64-elf --action <ActionName> --out <path>` uses
the same native emitter after its requirements/spec/core pipeline has produced
checked AIL-Core, so saved-spec and saved-core builds can produce the native
target without printing the VM artifact on stdout.
Native code generated from supported `REQUIRE_EXISTS` instructions checks for a
matching `key=` argument, native code generated from supported
`REQUIRE_FIELD_IN` instructions checks that at least one allowed `key=value`
argument is present, including LLM-style field requirements phrased as
`<field> is <value>` as well as `<field> to be <value>`. Field-reference
resolution can follow typed object fields, so `assignee role` on a `Ticket`
with `assignee: Option<User>` lowers to `ticket.assignee.role`. Native code
generated from supported negative field requirements treats both
`<field> not to be <value>` and LLM-style `<field> is not <value>` as exact
forbidden-value checks. A single LLM-style compound requirement can contribute
multiple checks, so `the ticket exists and its status is not Closed` lowers both
a `ticket.id` existence check and a `ticket.status != Closed` check. Compound
input requirements such as `the customer id and title` lower to
`REQUIRE_EXISTS` checks for `customer.id` and `ticket.title` after application
users are preserved in checked AIL-Core. Permission requirements phrased as
`the actor has role SupportAgent` lower to an exact `actor.role=SupportAgent`
field check, `the caller has Admin or SupportAgent role` lowers to
`caller.role` with both allowed values, and requirements such as
`the requesting user has permission to modify ticket status` lower to
`requesting user.permission=modify ticket status`, matching common LLM repair
output. Time comparison requirements such as
`the current time to be later than due_at` lower to `REQUIRE_FIELD_AFTER` from
deterministic runtime input `current.time` to `ticket.due_at`; the interpreter,
VM, and native executable all require the left value to sort later than the
right UTC timestamp token. Creation writes such as
`a Ticket with status New` lower to `SET_FIELD ticket.status=New`, so the
interpreter, VM, and native executable all materialize the initialized field.
Copy writes such as `the customer as the ticket customer` lower to
`COPY_FIELD` from `customer.id` to `ticket.customer.id`; the native backend
finds the source argv value and writes the copied destination state line without
generating host-language code. Object-field writes such as `the ticket
assignee` lower to `WRITE_FIELD ticket.assignee`; the native backend emits a
supplied nested identity value such as `ticket.assignee.id=A-1` on stdout so
assigned object identity remains visible in the native state-change stream.
Native code generated from supported
`REQUIRE_FIELD_NOT_EQUALS` instructions checks that the forbidden `key=value`
argument is absent. The executable exits `0` when those compiled requirements
pass and exits `1` when they fail. Native code generated from supported
`SET_FIELD` instructions writes the changed state as `key=value` lines to
stdout after requirements pass and before process exit. Native code generated
from supported Application semantic opcodes writes success-path trace entries
to stderr in VM trace order, including action start, rule-pass, write, effect,
guarantee, and explicit trace-event entries. Supported requirement failure
branches emit the VM-style action prefix, failure name, and any declared
failure trace events to stderr before exiting `1`. Native code generated from
supported AgentTool opcodes writes the same audit trace entries as the VM,
including tool start, requirements, typed inputs and outputs, reads, external
calls, writes, permissions, approvals, secret-protection declarations,
guarantees, and explicit trace events, while keeping secret runtime values out
of stdout and stderr. Native code generated from supported CompilerPass opcodes
writes the same compiler-pass trace entries as the VM, including pass start,
declared inputs and outputs, reads, steps, writes, guarantees, explicit trace
events, and the auditable `CORE_INFER_READ_PERMISSIONS` transform marker.
Native code generated from supported System opcodes writes the same low-level
component trace entries as the VM, including resources, ownership, borrowing,
region placement, layout, allocation, lock guards, execution context,
interrupts, scheduler tasks, capabilities, effects, guarantees, and explicit
trace events. Stdout remains reserved for parseable state changes. The first
native backend rejects future unknown VM opcodes and unlowered
`OBSERVE_RULE` requirements instead of silently emitting a partial executable.
With `--artifact-dir`, `ail-lower` writes `source.ail-package.md`,
`source.ail-spec.md`, and `source.fingerprint.txt` when a package source is
available; it also writes `checked.ail-core.txt`,
`checked.ail-core.fingerprint.txt`, `artifact.ailbc.json`,
`artifact.fingerprint.txt`, `manifest.ail-lower.txt`, and
`manifest.fingerprint.txt`, keeping direct IR-to-VM-instruction lowering
auditable while stdout remains the parseable VM instruction artifact. With
`--agent <agent-package-or-bytecode> --artifact-dir`, `ail-lower` must compile
or load an AIL-authored Application agent and run its `VerifyLowerManifest`
bytecode action against the checked core, checked-core fingerprint,
source-package fingerprint when present, bytecode artifact, bytecode
fingerprint, lower manifest, manifest fingerprint, and native-bytecode and
dependency report fingerprints when native verifier agents are emitted before
writing `agent.ailbc.json`, `agent.fingerprint.txt`, and `agent-trace.txt`. With
`--target linux-x86_64-elf`, it also emits `agent-<ActionName>.elf`
machine-code ELF executables for the lower verifier and records them as
`agent-target` entries in `manifest.ail-lower.txt`. That native lower-agent run
also writes `native-bytecode-report.txt`,
`native-bytecode-report.fingerprint.txt`, `dependency-report.txt`, and
`dependency-report.fingerprint.txt`; the reports prove the verifier-agent ELFs
are ELF64 little-endian x86_64 executable bytes and record
`host-language-runtime none`, `dynamic-linker none`, `shared-libraries none`,
`library-dependencies none`, and `linker-invocation none`.
`ail-check`, `ail-core`, `ail-flow`, `ail-lower`, `ail-compile`, `ail-run`,
and `ail-build` can use `--spec-file <path>` to read a saved generated
AIL-Spec artifact instead of the package entry spec, preserving the package
metadata while making accepted AIL-Spec files reusable inputs to IR rendering,
bytecode lowering, native target emission, and auditable build artifacts.
`ail-spec --core-file <path>` reads a saved checked AIL-Core artifact, runs the
core checker, reconstructs the semantic document, and prints deterministic
AIL-Spec. `ail-lower --core-file <path>`, `ail-compile --core-file <path>`,
and `ail-build --core-file <path>` read the same saved checked AIL-Core
artifact, reconstruct the graph from the serialized nodes, edges, and edge
attributes, run the core checker, and compile that IR directly to the VM
instruction artifact or native target without loading the source package spec.
This keeps AIL-Core as a real compiler and review boundary rather than only a
display format, and it preserves lowering payloads such as read/write
provenance text that affect emitted bytecode instructions.
`ail-vm` reads a saved AIL-Bytecode artifact and executes it directly without
reparsing the source AIL package, making bytecode a real artifact boundary
instead of only a display format. The VM verifier rejects unknown opcodes and
missing required operands before executing saved bytecode.
`ail-compile <artifact.ailbc.json> --action <ActionName> --target
linux-x86_64-elf --out <path>` reads the same saved AIL-Bytecode artifact,
verifies it, and emits a native ELF executable from that artifact boundary
without loading the source package or generating host-language backend source.
With `--artifact-dir`, direct `ail-compile` writes `source.ail-package.md`,
`source.ail-spec.md`, and `source.fingerprint.txt` when a package source is
available; it also writes `artifact.ailbc.json`, `artifact.fingerprint.txt`,
`target.elf`, `target.fingerprint.txt`, `native-bytecode-report.txt`,
`native-bytecode-report.fingerprint.txt`, `dependency-report.txt`,
`dependency-report.fingerprint.txt`, `manifest.ail-compile.txt`, and
`manifest.fingerprint.txt`; compiles from checked AIL-Core also include
`checked.ail-core.txt` and `checked.ail-core.fingerprint.txt`. The
native-bytecode report records the selected action target as ELF64 x86_64
executable bytes. The dependency report records `host-language-runtime none`,
`dynamic-linker none`, `shared-libraries none`, `library-dependencies none`,
and `linker-invocation none` for standalone Linux syscall ELF artifacts. The
compile manifest ties the source package when present, selected action,
verified bytecode artifact, native-bytecode report, dependency report, and
native target executable fingerprint into a reviewable artifact boundary. With
`--agent <agent-package-or-bytecode>`, `ail-compile --artifact-dir` also runs
the AIL-authored `VerifyCompileManifest` action over that manifest and writes
`agent.ailbc.json`, `agent.fingerprint.txt`, `agent-trace.txt`, and native
`agent-<ActionName>.elf` verifier executables recorded as `agent-target`
entries. The verifier reads the source package fingerprint when present and the
native-bytecode and dependency reports and fingerprints before accepting the
manifest.
With `--all-actions --target linux-x86_64-elf --artifact-dir <dir>`,
`ail-compile` compiles every action in the package or saved bytecode artifact
into native `target-<ActionName>.elf` executables. This exposes the same
multi-action native packaging used internally for AIL-authored agents and
compiler passes as a direct bootstrap command, so an AIL-authored toolchain
package can be reviewed as bytecode plus per-action machine-code ELF artifacts
without generating Rust or another host-language backend. The compile manifest
records a source package snapshot and fingerprint when package source is
available, `bundle all-actions`, and one fingerprinted `target` entry per
emitted executable, plus a native-bytecode report fingerprint proving every
target entry is ELF64 x86_64 executable bytes, and a dependency report
fingerprint proving every target entry is a standalone Linux syscall ELF with
no dynamic linker, shared libraries, host-language runtime, library
dependencies, or linker invocation. Adding
`--agent <agent-package-or-bytecode>` runs the AIL-authored
`VerifyCompileBundleManifest` action over the bundle manifest and writes the
agent bytecode, trace, and native verifier executables into the same artifact
directory, making the multi-action native package reviewable through AIL
bytecode rather than host orchestration alone. The verifier reads the source
package fingerprint when present and the native-bytecode and dependency reports
and fingerprints before accepting the bundle manifest.
`ail-bootstrap <toolchain-agent-package> --pass
<compiler-pass-package> --agent <toolchain-agent-package>
--target linux-x86_64-elf --artifact-dir <dir>` packages the AIL-authored
toolchain agent and an AIL-Meta compiler pass into one bootstrap artifact set.
It writes source package snapshots, `toolchain-agent.checked.ail-core.txt`,
`toolchain-agent.pass-output.ail-core.txt`, `toolchain-agent.pass-trace.txt`,
`toolchain-agent.ailbc.json`, `compiler-pass.checked.ail-core.txt`,
`compiler-pass.ailbc.json`, native ELF executables for every action in both
packages, package conformance reports, `agent.ailbc.json`, `agent-trace.txt`,
`bootstrap-fixed-point-report.txt`, `bootstrap-native-bytecode-report.txt`,
`bootstrap-host-boundary-report.txt`, `bootstrap-dependency-report.txt`,
`bootstrap-handoff-report.txt`, and `manifest.ail-bootstrap.txt`. The bootstrap
command runs the AIL-Meta compiler pass bytecode over the toolchain agent
checked IR, reruns the same pass over that output to prove the transformed IR
is stable, compiles the toolchain bytecode from the first transformed IR, and
records the machine-bytecode identity of every emitted native artifact. It also
runs every generated native AIL toolchain-agent action, every generated native
AIL verifier-agent action, and the native AIL-Meta `InferReadPermissions`
compiler pass through the Linux syscall argv ABI, then records that handoff
evidence in a fingerprinted report. The manifest records
`no-host-backend-source true` and deterministic fingerprints for source
packages, checked AIL-Core IR, compiler-pass output IR and trace, fixed-point
report, native-bytecode report, host-boundary report, dependency report,
native-handoff report, bytecode, conformance reports, and native executable
bytes. The AIL-authored `VerifyBootstrapManifest` action reads the
source-package fingerprint, checked-core fingerprint, compiler-pass trace,
fixed-point report fingerprint, native-bytecode report fingerprint, conformance
report fingerprint, host-boundary report fingerprint, dependency report
fingerprint, native-handoff report fingerprint, bytecode fingerprints, and
native target fingerprints before the bundle is accepted. The dependency report
records `host-language-runtime none`, `dynamic-linker none`,
`shared-libraries none`, `library-dependencies none`, and
`linker-invocation none` for standalone Linux syscall ELF artifacts, while the
handoff report records `handoff-native-role ... all-actions ok count ...` and
`handoff-native-action ... ok trace ...` for emitted native toolchain and agent
actions. This keeps the bootstrap boundary reviewable as AIL source, checked
IR, stable AIL compiler-pass output, AIL bytecode, reports, and machine-level
Linux ELF artifacts rather than a Rust or host-language backend source tree.
`ail-requirements` runs the first developer-facing agent capture stage by asking
the package base LLM for an AIL-Requirements artifact, checking profile-specific
coverage, and sending diagnostics back for one repair pass when the artifact is
too thin. It prints only the checked requirements artifact, so developers can
review or compare model-specific capture behavior before committing to AIL-Spec
and bytecode generation.
LLM output may be either the deterministic artifact text directly or the
prompt-pack JSON envelope with `artifact_text`. When the envelope contains
blocking `questions` and no artifact, the CLI prints the questions as a
blocking model response and does not continue into repair, checking, lowering,
or compile stages. Malformed envelopes are rejected as `AIL-PROMPT-001`
prompt protocol errors. For envelope outputs, the CLI validates the requested
`artifact_kind`, requires `checker_handoff.must_check: true`, and requires
`checker_handoff.expected_profile` to match the package profile before it
extracts `artifact_text`.
`ail-spec` runs the next stage from a saved checked AIL-Requirements artifact:
it validates the requirements file, asks the package base LLM for an AIL-Spec
candidate grounded in that artifact, repairs once on checker diagnostics, and
prints only the accepted AIL-Spec. With `--core-file`, it skips the LLM and
renders checked AIL-Core back into canonical AIL-Spec. This makes
requirements-to-spec and core-to-spec conversion reviewable artifact boundaries
instead of internal `ail-build` details.
`ail-patch --core-file <path> <patch.json>` reads a saved checked AIL-Core
artifact and a stage-0 `ail-core.patch.v0` JSON patch, applies supported
`add_node`, `remove_node`, `add_edge`, `remove_edge`,
`replace_edge_attributes`, and `replace_node_attributes` graph operations, runs
the AIL-Core checker on the patched graph, and prints the patched Core
artifact. The patch `base_hash` must match the canonical checked Core hash
before any operation is applied. AIL-Flow exposes the hash as `coreHash`, node
patch labels as `coreLabel`, and patch-ready edge labels as `edgeRefs` on
action, tool, compiler-pass, and system-component projections. Node removals
are guarded: they only delete detached nodes and require relationships to be
removed first. Attribute replacement is key-merge based; node edits can update
`type` and rewire changed stable node ids before checking, while edge attribute
edits rewrite the stable edge id. This is the first concrete AIL-Flow /
agent-edit path that edits Core directly before rendering back to AIL-Spec.
`ail-pass` compiles an AIL-Meta compiler pass package into verified
AIL-Bytecode, or reads a saved Compiler-profile AIL-Bytecode artifact, checks a
target package into AIL-Core, executes the selected pass bytecode over that
checked IR, and prints the transformed AIL-Core artifact. This exposes
AIL-authored compiler passes as a command-line toolchain stage and as reusable
bytecode artifacts without generating Rust or other host-language source. With
`--core-file <path>`, `ail-pass` reads the checked target AIL-Core artifact
directly instead of loading the target source package, so a saved
Compiler-profile bytecode artifact can transform a saved IR artifact as a
standalone compiler stage. With `--artifact-dir`, package-backed pass runs
also write `compiler-pass.source.ail-package.md`,
`compiler-pass.source.ail-spec.md`, `compiler-pass.source.fingerprint.txt`,
`target.source.ail-package.md`, `target.source.ail-spec.md`, and
`target.source.fingerprint.txt`, and include fingerprinted `compiler-pass-source`
and `target-source` manifest entries. Saved compiler-pass bytecode and saved
target core inputs remain source-free artifact boundaries. With
`--agent <agent-package-or-bytecode>`,
`ail-pass` compiles or loads an AIL-authored Application agent and runs its
`AcceptCompilerPassOutput` bytecode action against the transformed core,
compiler-pass bytecode fingerprint, and pass execution trace. This gives the
standalone compiler-pass stage an AIL-bytecode acceptance checkpoint instead of
leaving it as host orchestration. When `--artifact-dir` is present, the agent
also runs `VerifyPassManifest` with the rendered pass manifest and deterministic
manifest fingerprint plus source fingerprints when present and native-bytecode
report and dependency report fingerprints when native ELF tools are emitted, so
the pass artifact set is verified in AIL bytecode too.
With
`--artifact-dir`, the same command writes `pass.ailbc.json`,
`pass.fingerprint.txt`, `input.ail-core.txt`, `output.ail-core.txt`, and
`trace.txt` plus `manifest.ail-pass.txt`, a deterministic index tying the pass
bytecode fingerprint to the input core, output core, and execution trace. It
also writes `manifest.fingerprint.txt` for that manifest's deterministic
fingerprint. When `--agent` is present, the artifact directory also includes
`agent.ailbc.json`, `agent.fingerprint.txt`, and `agent-trace.txt`, and the
manifest indexes the agent bytecode and trace. With
`--target linux-x86_64-elf`, `ail-pass --artifact-dir` also writes
`pass-<ActionName>.elf` for each AIL-authored compiler-pass action and records
each executable as a `compiler-pass-target` manifest entry. When that native
pass run also uses `--agent`, the artifact directory includes
`agent-<ActionName>.elf` for each AIL-authored pass-agent action and records
each executable as an `agent-target` manifest entry. It also writes
`native-bytecode-report.txt` and `native-bytecode-report.fingerprint.txt`,
records the report in `manifest.ail-pass.txt`, and has the AIL-authored
`VerifyPassManifest` action read the report fingerprint before accepting the
manifest. The native pass artifact directory also includes
`dependency-report.txt` and `dependency-report.fingerprint.txt`; the report
records `host-language-runtime none`, `dynamic-linker none`,
`shared-libraries none`, `library-dependencies none`, and `linker-invocation
none` for the native compiler-pass and pass-agent ELF artifacts. The pass
manifest records the dependency report, and `VerifyPassManifest` reads the
dependency report fingerprint before accepting the artifact set. This keeps
pass execution auditable while stdout remains the transformed AIL-Core
artifact.
`ail-build` composes the LLM draft loop with the same checked IR-to-artifact
lowering: the base LLM first drafts an AIL-Requirements artifact from a user
prompt.
`ail-build` checks that artifact for profile-specific coverage before spec
drafting; if it is too thin, the command sends requirements diagnostics back to
the base LLM for one repair pass. It then drafts an AIL-Spec candidate for the
package profile grounded in those checked requirements. Prompt-driven
requirements capture scopes the coverage instruction to the package profile:
Application prompts ask for application domain objects and actions, AgentTool
prompts ask for tool capability and inputs/outputs, Compiler prompts ask for
compiler-pass transformations, and System prompts ask for components,
resources, and effects. During requirements-grounded spec drafting, the checker
also compares permission-bearing requirement bullets against the corresponding
action in the accepted spec; if a requirement says an action needs permission,
approval, authorization, access control, a role, or a forbidden-state guard, the
spec must preserve an explicit action requirement before it can be lowered. With
`--requirements-file <path>`, `ail-build` skips requirements capture, validates
the saved AIL-Requirements artifact, and resumes at the requirements-grounded
spec-drafting stage. If the checker rejects the first candidate, `ail-build`
sends the candidate plus detailed diagnostics and repair suggestions back to the
base LLM for one repair pass. With `--spec-file <path>`, `ail-build` skips all
LLM calls, parses the saved accepted AIL-Spec artifact against the package
metadata, and resumes at checked AIL-Core elaboration. With
`--core-file <path>`, `ail-build` skips both LLM and AIL-Spec parsing stages
and resumes from the saved checked AIL-Core IR artifact. Only a checked
candidate, saved spec, or saved core is lowered to a target artifact. If `--pass
<compiler-pass-package-or-bytecode>` is supplied, `ail-build` loads that
AIL-authored Compiler-profile bytecode, requires exactly one pass action, runs
it over the checked candidate AIL-Core, and re-checks the transformed IR before
lowering. By default the bytecode compiler consumes the resulting checked IR to
emit verified AIL-Bytecode. With `--target linux-x86_64-elf --action
<ActionName> --out <path>`, the native emitter consumes the same checked IR and
writes a Linux x86_64 ELF executable instead of printing the VM artifact. Both
paths emit toolchain artifacts rather than host-language source. With `--agent
<agent-package-or-bytecode>`, `ail-build`
compiles or loads an AIL-authored Application-profile toolchain agent, verifies
its bytecode, and for prompt-driven builds runs its `CaptureRequirements`
action before the base LLM requirements request, then includes the
agent-produced requirements checklist state in that first prompt. After
requirements capture and checking, it runs `PrepareSpecDraft` and includes the
agent-produced spec checklist state in the AIL-Spec prompt. For saved
`--requirements-file` builds, the agent starts from an explicit
`RequirementsLoaded` state, loads the checked requirements into the
`BuildRequest`, and still runs `PrepareSpecDraft` before the base LLM spec
request. Once the checked AIL-Spec draft is accepted, it runs
`AcceptSpecDraft` before AIL-Core elaboration so the `SpecCaptured` transition
is also represented in AIL bytecode. For saved `--spec-file` builds, the agent
starts only after the saved spec has parsed, elaborated to AIL-Core, and passed
the checker. It then starts from an explicit `SpecLoaded` state, loads the
checked spec into the `BuildRequest`, and runs `AcceptSpecDraft`. A rejected
saved spec produces deterministic `ail-build diagnostics` before any build
agent action is run.
It optionally runs
`CompareAgentPromptPortability` when
`--target-model <name>` is supplied, using `--base-model <name>` or the active
LLM endpoint as the source model label. With `--artifact-dir`, that comparison
is persisted as `prompt-portability.txt` with
`prompt-portability.fingerprint.txt`; the report records both the base and
target model labels, `manifest.ail-build.txt` records a `prompt-portability`
entry, and the AIL-authored `VerifyBuildManifest` action reads the
prompt-portability fingerprint before accepting the manifest. When
`--pass` is supplied, it runs
`AcceptCompilerPassOutput` after the AIL-authored compiler pass bytecode
transforms the checked AIL-Core, passing the pass bytecode boundary and VM trace
through the agent state before accepting the post-pass IR. It then runs
`AcceptCoreIR` after AIL-Core checking and any compiler pass, and then runs its
`CompileApplication` action against the completed build state before target
artifact emission. For saved
`--core-file` builds, the agent starts from an explicit `CoreLoaded` state,
loads the checked core into the `BuildRequest`, and still runs `AcceptCoreIR`
before `CompileApplication`. After the Rust bootstrap compiler emits and
verifies the target artifact, the agent runs
`VerifyBytecodeArtifact` with the emitted artifact summary and deterministic
bytecode fingerprint for the VM artifact. When the selected target is native
ELF, the agent also runs `CompileNativeTarget` with the bytecode artifact,
bytecode fingerprint, target platform, native target artifact summary, and
deterministic target fingerprint, recording `NativeTargetCompiled` before
`VerifyTargetArtifact` verifies the final machine-code boundary. When
`--artifact-dir` is present, the agent also runs
`VerifyBuildManifest` with the rendered build manifest, deterministic manifest
fingerprint, source-package fingerprint when a package source is available,
requirements and spec fingerprints when those artifacts are present,
checked-core fingerprint, native target fingerprint when a native target is
present, the native-bytecode report and fingerprint when native ELF artifacts
are present, and the native compiler-pass executable fingerprint when a native
build pass is present, so the whole
requirements/spec/core/pass/agent/artifact boundary is represented in AIL
bytecode. This keeps the
developer-facing build coordinator in AIL bytecode
instead of adding a host-language orchestration layer. With `--artifact-dir`,
the same command writes `source.ail-package.md`, `source.ail-spec.md`, and
`source.fingerprint.txt` when a package source is available. It writes
`accepted.ail-spec.md`, `checked.ail-core.txt`,
`checked.ail-core.fingerprint.txt`, `artifact.ailbc.json`, and
`artifact.fingerprint.txt`; it also writes `requirements.ail-requirements.md`
and `requirements.fingerprint.txt` when the build captured or loaded
requirements, and it writes `accepted.ail-spec.md` and
`accepted.ail-spec.fingerprint.txt` only when an AIL-Spec stage was present.
When a build
pass is present, `checked.ail-core.txt` is the post-pass IR that was actually
lowered, and the artifact directory also includes `pass.ailbc.json`,
`pass.fingerprint.txt`, and `pass-trace.txt` for the compiler-pass bytecode,
deterministic pass fingerprint, and execution trace. When the same build
selects the native `linux-x86_64-elf` target, the artifact directory also
includes `pass-<ActionName>.elf` for each AIL-authored compiler-pass action,
and the manifest records each as a `compiler-pass-target` entry with the
action executable fingerprint. When a native target is selected, the artifact
directory also includes `target.elf`, `target.fingerprint.txt`,
`native-bytecode-report.txt`, `native-bytecode-report.fingerprint.txt`,
`dependency-report.txt`, and `dependency-report.fingerprint.txt`.
The native-bytecode report records the `target.elf` machine identity as
ELF64 little-endian x86_64 executable bytes and includes native compiler-pass
and agent executables when they are emitted. The dependency report records
`host-language-runtime none`, `dynamic-linker none`, `shared-libraries none`,
`library-dependencies none`, and `linker-invocation none` for the target,
native compiler-pass, and native verifier-agent ELF artifacts.
`manifest.ail-build.txt` indexes the native target, native-bytecode report,
dependency report, and fingerprints alongside the VM artifact. When a build
agent is present, the artifact directory also includes
`agent.ailbc.json`, `agent.fingerprint.txt`, and `agent-trace.txt` for the agent
bytecode, deterministic agent fingerprint, and its requirements-capture,
prompt-portability, application-compile, native-bytecode-report,
dependency-report, and bytecode-verification trace. When
that agent build also selects the native `linux-x86_64-elf` target, the
artifact directory includes one `agent-<ActionName>.elf` executable per
AIL-authored agent action, and the manifest records each as an `agent-target`
entry with the action executable fingerprint. The
artifact directory also includes `manifest.ail-build.txt`, a deterministic
index tying the source package fingerprint, emitted requirements fingerprint,
accepted spec fingerprint, checked core, compiler-pass bytecode and trace,
agent bytecode and trace, final bytecode fingerprint, and native target
fingerprint into one review artifact, plus native-bytecode and dependency
report fingerprints when a native target is selected, plus
`manifest.fingerprint.txt` for that manifest's
deterministic fingerprint. This
lets the developer audit the
requirements-to-spec-to-IR-to-pass-to-agent-to-artifact chain, a
saved-spec-to-IR-to-agent-to-artifact chain, or a
saved-core-to-agent-to-artifact chain while stdout remains the parseable
bytecode artifact for VM builds and a status line for native target builds.

### Diagnostics

Diagnostics report checker and runtime failures with:

- failure code
- human-readable message
- source provenance
- affected graph node or edge
- repair suggestion

Diagnostics must be stable enough for tests and agent explanations.
Conformance output and LLM draft candidate checks use the structured diagnostic
representation so rejected fixtures and invalid `ail-draft` candidates can
expose the stable code/message plus source provenance, affected graph item, and
repair guidance without changing the plain-message checker API.
Diagnostics that come from action reads or writes use the behavior bullet as
source provenance and the corresponding semantic graph edge as the affected
graph item. Diagnostics that come from failure declarations distinguish the
action failure edge from the declared failure section so repair guidance can
point at the missing declaration, missing handling, or missing trace coverage.
Unknown field reference diagnostics use the unresolved read/write edge as the
affected graph item so the repair can distinguish read bullets from write
bullets. Field type and requirement-field diagnostics point at the field
declaration or requirement rule that introduced the invalid reference. Semantic
integrity diagnostics for provenance and attachment point at the semantic node
whose graph invariant is incomplete. AgentTool audit trace diagnostics point at
the tool node so the repair can name the missing `The tool records:` section.

## First Accepted Artifact Format

The first accepted artifact can be a deterministic text serialization of
AIL-Core. It must include:

- package metadata
- nodes
- edges
- attributes
- provenance
- conformance level
- stable ordering

This format is a bootstrap artifact. It may later be replaced by a canonical
binary or structured package format if round-trip equivalence and migration
rules are preserved.

## Development Sequence

1. Implement package loading for one local package directory.
2. Parse the Support Ticket AIL-Spec example into a draft document.
3. Elaborate things, fields, actions, failures, guarantees, and traces into
   candidate AIL-Core.
4. Normalize and serialize AIL-Core deterministically.
5. Add checker rules for references, types, secrets, failures, guarantees, and
   provenance.
6. Render checked AIL-Core back into deterministic AIL-Spec.
7. Prove render/reparse graph equivalence.
8. Execute checked actions against simple state, including generic declared
   field reads, field writes, positive and negative field requirements, and
   requirement failure mapping.
9. Execute the NotFound failure path and produce a semantic trace.
10. Add diagnostics for one missing requirement reference, one unknown field
    reference, one secret leak, and one missing failure handler.
11. Render a deterministic no-code AIL-Flow projection from checked AIL-Core.
12. Apply a checked AIL patch from a no-code or agent edit and prove the
    patched AIL-Spec render reparses to equivalent AIL-Core.
13. Load imported package fragments under explicit aliases and prove canonical
    render/reparse equality after namespacing.
14. Package the slice as the first conformance fixture.
15. Lower checked Application-profile AIL-Core to AIL-Bytecode and execute the
    bytecode with the bootstrap VM.

## Out Of Scope For The First Slice

These are required by the long-term language, but not by the first
implementation slice:

- unrestricted natural-language parsing
- general AI-agent interview UI
- full no-code editor patching
- package registry
- self-hosted compiler generation
- systems memory layout lowering
- host-language backend source generation
- full standard library
- concurrent execution

Each remains part of the language framework through the active specs.

## Completion Gate For The First Slice

The first slice is ready when:

- Support Ticket parses into AIL-Core
- AIL-Core checks without unresolved questions
- checked AIL-Core renders to deterministic AIL-Spec
- deterministic AIL-Spec reparses to equivalent AIL-Core
- the success runtime path produces expected state and trace
- the failure runtime path produces expected failure and trace
- generic runtime field reads and writes resolve declared fields, prefer
  qualified field references, enforce positive and negative field requirements,
  and preserve trace events
- invalid fixtures produce stable diagnostics for missing requirement
  references, unknown requirement fields, unknown field references, unknown
  field types, secret writes without protection, secret reads without
  protection, declared failures without handling, and declared failures without
  trace coverage, actions without trace coverage, and semantic nodes without
  provenance, guarantees not attached to actions or tools, and traces not
  recorded by actions or failures, rules not required by actions, and effects
  not attached to actions or failures, and secrets not attached to fields or
  actions
- an AIL-Flow projection renders from checked AIL-Core
- AIL-Core lowers to deterministic AIL-Bytecode for supported Application,
  AgentTool, Compiler, and System packages
- `ail-run` executes supported bytecode packages through the AIL bytecode VM
- saved AIL-Bytecode artifacts parse back into bytecode and execute through
  `ail-vm` without requiring the source package
- saved AIL-Bytecode artifacts are verified for known opcodes and required
  operands before VM execution
- a checked AIL patch applies, renders as deterministic AIL-Spec, and reparses
  to equivalent AIL-Core
- imported package declarations are namespaced under aliases and preserve
  canonical render/reparse equivalence
- conformance validates the entry spec plus accepted and rejected fixture
  directories for Application, AgentTool, Compiler, and System profile packages
- all artifacts keep provenance back to declarations and source behavior bullets
