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

The next implemented profile expansion is the Refund Tool `AgentTool` example.
It proves that the same package loader, parser, AIL-Core store, checker,
renderer, AIL-Flow projection, and LLM draft loop can represent an agent tool
contract with typed inputs and outputs, requirements, reads, writes, external
calls, secret protection, failures, guarantees, and provenance.

The AIL Toolchain Agent Application example is the first AIL-authored toolchain
agent. It models the agent that interviews an application developer, captures
requirements, turns checked specs into AIL-Core IR, compiles verified
AIL-Bytecode, and compares prompt portability across target LLMs. Because it is
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
diagnostic.

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
- read/write/call/effect bullets
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
with `assignee: Option<User>` lowers to `ticket.assignee.role`. Compound input
requirements such as `the customer id and title` lower to `REQUIRE_EXISTS`
checks for `customer.id` and `ticket.title` after application users are
preserved in checked AIL-Core. Time comparison requirements such as
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
With `--artifact-dir`, `ail-lower` writes `checked.ail-core.txt`,
`artifact.ailbc.json`, `artifact.fingerprint.txt`, `manifest.ail-lower.txt`,
and `manifest.fingerprint.txt`, keeping direct IR-to-VM-instruction lowering
auditable while stdout remains the parseable VM instruction artifact.
`ail-check`, `ail-core`, `ail-flow`, `ail-lower`, `ail-compile`, `ail-run`,
and `ail-build` can use `--spec-file <path>` to read a saved generated
AIL-Spec artifact instead of the package entry spec, preserving the package
metadata while making accepted AIL-Spec files reusable inputs to IR rendering,
bytecode lowering, native target emission, and auditable build artifacts.
`ail-lower --core-file <path>`, `ail-compile --core-file <path>`, and
`ail-build --core-file <path>` read a saved checked AIL-Core artifact,
reconstruct the graph from the serialized nodes, edges, and edge attributes,
run the core checker, and compile that IR directly to the VM instruction
artifact or native target without loading the source package spec. This keeps
AIL-Core as a real compiler input boundary rather than only a display format,
and it preserves lowering payloads such as read/write provenance text that
affect emitted bytecode instructions.
`ail-vm` reads a saved AIL-Bytecode artifact and executes it directly without
reparsing the source AIL package, making bytecode a real artifact boundary
instead of only a display format. The VM verifier rejects unknown opcodes and
missing required operands before executing saved bytecode.
`ail-requirements` runs the first developer-facing agent capture stage by asking
the package base LLM for an AIL-Requirements artifact, checking profile-specific
coverage, and sending diagnostics back for one repair pass when the artifact is
too thin. It prints only the checked requirements artifact, so developers can
review or compare model-specific capture behavior before committing to AIL-Spec
and bytecode generation.
`ail-spec` runs the next stage from a saved checked AIL-Requirements artifact:
it validates the requirements file, asks the package base LLM for an AIL-Spec
candidate grounded in that artifact, repairs once on checker diagnostics, and
prints only the accepted AIL-Spec. This makes requirements-to-spec conversion a
reviewable artifact boundary instead of an internal `ail-build` detail.
`ail-pass` compiles an AIL-Meta compiler pass package into verified
AIL-Bytecode, or reads a saved Compiler-profile AIL-Bytecode artifact, checks a
target package into AIL-Core, executes the selected pass bytecode over that
checked IR, and prints the transformed AIL-Core artifact. This exposes
AIL-authored compiler passes as a command-line toolchain stage and as reusable
bytecode artifacts without generating Rust or other host-language source. With
`--core-file <path>`, `ail-pass` reads the checked target AIL-Core artifact
directly instead of loading the target source package, so a saved
Compiler-profile bytecode artifact can transform a saved IR artifact as a
standalone compiler stage. With `--agent <agent-package-or-bytecode>`,
`ail-pass` compiles or loads an AIL-authored Application agent and runs its
`AcceptCompilerPassOutput` bytecode action against the transformed core,
compiler-pass bytecode fingerprint, and pass execution trace. This gives the
standalone compiler-pass stage an AIL-bytecode acceptance checkpoint instead of
leaving it as host orchestration. When `--artifact-dir` is present, the agent
also runs `VerifyPassManifest` with the rendered pass manifest and deterministic
manifest fingerprint, so the pass artifact set is verified in AIL bytecode too.
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
each executable as an `agent-target` manifest entry. This keeps pass execution
auditable while stdout remains the transformed AIL-Core artifact.
`ail-build` composes the LLM draft loop with the same checked IR-to-artifact
lowering: the base LLM first drafts an AIL-Requirements artifact from a user
prompt.
`ail-build` checks that artifact for profile-specific coverage before spec
drafting; if it is too thin, the command sends requirements diagnostics back to
the base LLM for one repair pass. It then drafts an AIL-Spec candidate for the
package profile grounded in those checked requirements. With
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
starts from an explicit `SpecLoaded` state, loads the checked spec into the
`BuildRequest`, and still runs `AcceptSpecDraft` before AIL-Core elaboration.
It optionally runs
`CompareAgentPromptPortability` when
`--target-model <name>` is supplied. When `--pass` is supplied, it runs
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
ELF, the agent also runs `VerifyTargetArtifact` with the native target artifact
summary and deterministic fingerprint so the final machine-code boundary is
represented in AIL bytecode. When `--artifact-dir` is present, the agent also runs
`VerifyBuildManifest` with the rendered build manifest, deterministic manifest
fingerprint, and native target fingerprint when a native target is present, so
the whole requirements/spec/core/pass/agent/artifact boundary is represented in
AIL bytecode. This keeps the
developer-facing build coordinator in AIL bytecode
instead of adding a host-language orchestration layer. With `--artifact-dir`,
the same command writes
`accepted.ail-spec.md`, `checked.ail-core.txt`, `artifact.ailbc.json`, and
`artifact.fingerprint.txt`; it also writes `requirements.ail-requirements.md`
when the build captured or loaded requirements, and it writes
`accepted.ail-spec.md` only when an AIL-Spec stage was present. When a build
pass is present, `checked.ail-core.txt` is the post-pass IR that was actually
lowered, and the artifact directory also includes `pass.ailbc.json`,
`pass.fingerprint.txt`, and `pass-trace.txt` for the compiler-pass bytecode,
deterministic pass fingerprint, and execution trace. When a native target is
selected, the artifact directory also includes `target.elf` and
`target.fingerprint.txt`, and `manifest.ail-build.txt` indexes the native target
and fingerprint alongside the VM artifact. When a build agent is
present, the artifact directory also includes
`agent.ailbc.json`, `agent.fingerprint.txt`, and `agent-trace.txt` for the
agent bytecode, deterministic agent fingerprint, and its requirements-capture,
prompt-portability, application-compile, and bytecode-verification trace. When
that agent build also selects the native `linux-x86_64-elf` target, the
artifact directory includes one `agent-<ActionName>.elf` executable per
AIL-authored agent action, and the manifest records each as an `agent-target`
entry with the action executable fingerprint. The
artifact directory also includes `manifest.ail-build.txt`, a deterministic
index tying the emitted requirements, accepted spec, checked core, compiler-pass
bytecode and trace, agent bytecode and trace, final bytecode fingerprint, and
native target fingerprint into one review artifact, plus
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
repair guidance without changing the legacy plain-message checker API.
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
