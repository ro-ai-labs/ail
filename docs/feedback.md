## Executive assessment

The documents are directionally strong. They already define AIL as an English-first, agent-assisted, deterministic semantic programming language rather than a prompt format or no-code product. They also establish the key trust boundary: conversation and LLM outputs are not compiled directly; only checked deterministic artifacts are accepted, with AIL-Core as the semantic source of truth. That matches your desired architecture very closely.  

The main issue is that the suite is still more of a strategic specification than a complete language definition. To reach the desired outcome, the next step is to add formal executable semantics, explicit Turing-completeness constructs, a canonical IR schema, a real prompt pack, a C/ABI interop profile, standard library/package definitions, and conformance fixtures that prove round-trip stability across LLMs, visual views, IR, bytecode, and native binaries.

## What is already well aligned

The foundation and architecture documents already capture the most important invariant: AIL starts from ordinary English, but compiles only deterministic artifacts derived from clarification, structured English, and AIL-Core. The documents also correctly state that AIL-Core is the accepted source of truth and that structured English, graphical views, traces, diagnostics, and lower-level explanations are projections of that source of truth.  

AIL-Spec is already framed correctly as deterministic structured English that is readable by non-engineers but regular enough for an AI Agent to generate, patch, normalize, and explain. It already requires actors, data, actions, rules, failures, secrets, permissions, effects, guarantees, and views. 

AIL-Core is also heading in the right direction: it is defined as a canonical typed semantic graph, not a syntax tree, with stable IDs, typed nodes and edges, attributes, provenance, normalization, and equivalence rules. This is the right core for an LLM-assisted language because it gives the compiler a deterministic authority that is independent of model wording. 

The agent protocol has the correct trust stance: the AI Agent interviews, drafts, explains, debugs, and proposes patches, but the checker remains the authority. The protocol also already requires calibration examples, prompt compatibility, provenance preservation, and detection of hallucinated fields, secret disclosure, hidden external calls, incomplete failure handling, projection drift, and trace/explanation mismatch. 

The no-code view model is also conceptually right. AIL-Flow is specified as a deterministic projection of AIL-Core, with application maps, action cards, data tables, rules, permissions, failures, traces, tool views, system views, lowering views, and diagnostic views. It also says visual edits must become checked graph patches, not opaque UI edits. 

The self-hosting direction is present. The documents allow Rust and other systems as bootstrap scaffolding while requiring that the mature toolchain define the compiler, runtime, standard library, package system, debugger, agent protocol, build system, and conformance suite in AIL itself. 

## The critical gaps

The first major gap is **Turing completeness**. The current documents imply a complete language, but they do not yet formally define the constructs needed to prove it: functions, action composition, recursion, loops, conditionals, pattern matching, mutable state, closures or callable values, modules, and operational semantics. AIL currently has actions, steps, rules, effects, values, failures, traces, and compiler passes, but the docs should explicitly define the minimal Turing-complete core and its projection into structured English and AIL-Core.  

The second gap is **formal AIL-Core schema and grammar**. The IR document lists node kinds and edge kinds, but the next version needs actual cardinality rules, type rules, attribute schemas, canonical serialization, graph validation rules, and stable hashing algorithms. Right now the docs say each edge kind defines source/target kinds, cardinality, and checker rules, but the actual definitions are not yet enumerated as machine-checkable schema. 

The third gap is **actual agent prompt assets**. The protocol defines what prompts must accomplish, but the uploaded suite does not yet include the complete system prompts for interview, requirements capture, spec drafting, IR generation, IR-to-spec rendering, diagnostics explanation, trace debugging, patch proposal, and prompt portability testing. The protocol says each feature should have compact rules, canonical forms, valid/invalid examples, diagnostics, round-trip expectations, no-code rendering expectations, and trace/debugging expectations; those should become versioned prompt-pack artifacts. 

The fourth gap is **round-trip determinism as an algorithm**. The round-trip document defines strong, behavioral, and explanation equivalence, and correctly says graph checks are authoritative while embedding distance is only a drift signal. But it still needs precise normalization algorithms, semantic diff rules, projection-loss rules, and conformance fixtures for every conversion path. 

The fifth gap is **C interop and ABI interop**. The System profile already mentions ABI-visible layout and examples such as `repr(C), align 8`, and the implementation guide discusses direct ELF generation, but C interop is not yet a first-class profile with declarations for imported C functions, headers, structs, unions, pointers, ownership transfer, errno/failure mapping, callbacks, dynamic/static linking, symbol visibility, and unsafe boundaries.  

The sixth gap is **standard libraries and package model**. The foundation says AIL must eventually define its standard library, runtime primitives, package system, build system, and conformance suite in AIL, but the suite does not yet define the standard library modules, library authoring rules, package compatibility rules, version constraints, capability grants, or documentation/projection requirements for libraries. 

The seventh gap is **full-stack scope control**. The docs correctly state that AIL-System must eventually express kernels, runtimes, drivers, schedulers, memory managers, filesystems, network stacks, embedded apps, compiler backends, and performance-critical libraries. But that ambition needs a staged capability matrix so the language does not claim “kernels to advanced UI/UX applications” before the semantics for each layer are independently specified and tested. 

## Recommended action set

### 1. Add a formal “AIL execution semantics” document

Create a new document, for example `17-execution-semantics.md`, that defines how checked AIL-Core executes. This should include evaluation order, action invocation, step execution, branching, loops, recursion, state mutation, event emission, tool calls, failure propagation, compensation, guarantees, traces, and concurrency boundaries.

The document should define a minimal executable core that can express every higher-level profile. For Turing completeness, explicitly include unbounded recursion or unbounded iteration, conditional branching, state/value construction, function or action invocation, and a proof sketch showing that the core can encode a known Turing-complete model.

Definition of done: one accepted example that computes over recursive or iterative data; one rejected example with non-terminating behavior where termination is required by a profile; one trace example showing loop/recursion execution; one AIL-Core rendering; one AIL-Flow rendering.

### 2. Add a machine-checkable AIL-Core schema

Expand `03-semantic-ir.md` into a real schema reference or add `18-ail-core-schema.md`. For every node and edge kind, define required attributes, optional attributes, allowed source and target node kinds, cardinality, normalization rules, provenance requirements, checker rules, and stable serialization order.

This is essential because AIL-Core is the compiler boundary. The current document correctly states that AIL-Core is a directed attributed graph with stable identity, provenance, deterministic normalization, and equivalence rules, but the compiler team will need a concrete schema to implement validators and generate conformance tests. 

Definition of done: a canonical JSON or text schema; a hash algorithm; a graph normalization algorithm; stable examples for Application, AgentTool, Compiler, and System profiles; invalid graph examples with expected diagnostics.

### 3. Define the Turing-complete AIL-Core subset before expanding surface English

Do not try to make free-form English Turing-complete first. Define the Turing-complete subset at AIL-Core level, then define how AIL-Spec and AIL-Flow project to and from that subset.

Add constructs such as:

`Action`, `Function`, `Call`, `Return`, `Branch`, `Loop`, `ForEach`, `While`, `Match`, `Let`, `Assign`, `Read`, `Write`, `Emit`, `Fail`, `HandleFailure`, `GuaranteeCheck`, `ExternalCall`, `Allocate`, `Borrow`, `Release`.

Then define structured English forms for each:

“When the system repeats for each item in <list>…”
“When <condition> remains true, the system repeats…”
“The action calls <action> with…”
“If <result> is Failure, the system handles <failure> by…”

Definition of done: AIL-Core can encode recursive factorial, list map/filter/reduce, stateful counter, event loop, and a compiler pass over a graph.

### 4. Split AIL-Spec into “controlled structured English” and “rendered explanation English”

AIL-Spec is currently both human-readable and parser-friendly. That is correct, but the next spec should separate two modes:

`AIL-Spec Canonical`: deterministic, parser-owned, normalized structured English.

`AIL-Spec Friendly`: user-facing paraphrase generated from AIL-Core for review.

The compiler should only parse canonical AIL-Spec. The agent may show friendly explanations, but friendly prose must be traceable to canonical sections.

This reduces ambiguity and improves LLM portability. The current AIL-Spec document already requires deterministic structured English, stable provenance, required slots, and rejection of ambiguous specs; the next step is to formalize the exact accepted grammar. 

Definition of done: grammar for canonical headings and bullets; canonical renderer; friendly renderer; examples where two friendly specs normalize to the same canonical AIL-Core.

### 5. Add a complete agent prompt pack

Create `prompts/` or a new document `19-agent-prompt-pack.md` with versioned system prompts and tool prompts. The current protocol says the agent should interview users, identify missing semantics, produce AIL-Spec, elaborate AIL-Core, render back to English, propose patches, explain diagnostics, debug traces, generate examples, and preserve provenance. That needs to become actual prompt artifacts. 

Minimum prompt set:

`interview.system.md`: turns user intent into questions.
`requirements.system.md`: produces AIL-Requirements.
`spec-draft.system.md`: converts requirements into canonical AIL-Spec.
`core-draft.system.md`: converts AIL-Spec into candidate AIL-Core text.
`diagnostic-repair.system.md`: repairs rejected artifacts without inventing semantics.
`core-to-spec.system.md`: renders AIL-Core back to structured English.
`core-to-summary.system.md`: explains AIL-Core simply to a non-engineer.
`flow-patch.system.md`: converts visual edits into graph patches.
`trace-debug.system.md`: explains runtime traces without inventing facts.
`interop.system.md`: asks safe questions for external libraries and C APIs.

Definition of done: each prompt includes purpose, forbidden behavior, input schema, output schema, few-shot examples, invalid examples, provenance rules, and checker handoff rules.

### 6. Add a prompt portability and LLM equivalence harness

Your goal says any capable LLM given the right prompts should reconstruct a context-equivalent specification. The current implementation guide mentions comparing prompt portability from a base model to a target model, but this should become a first-class conformance suite. 

Add tests where multiple models receive the same user request and prompt pack. Acceptance should not depend on identical wording. It should depend on whether the generated AIL-Spec normalizes into equivalent AIL-Core, or whether unresolved questions are correctly surfaced instead of guessed.

Definition of done: benchmark tasks, model-output artifact capture, normalized graph comparison, diagnostic comparison, failure taxonomy, and a “portable prompt compatibility score.”

### 7. Formalize round-trip equivalence algorithms

The round-trip document already defines the required conversions: AIL-Spec to AIL-Core to AIL-Spec, AIL-Core to AIL-Spec to AIL-Core, AIL-Core to AIL-Flow patch, trace to explanation, and lower-level artifact to semantic explanation. 

Now add:

normalization pseudocode;
semantic hash computation;
graph isomorphism constraints;
allowed alias normalization;
default expansion rules;
projection loss rules;
semantic diff format;
explanation-equivalence rubric;
LLM-assisted explanation drift checks;
blocking versus non-blocking equivalence failures.

Definition of done: every required round trip has at least one accepted fixture, one rejected fixture, and one diagnostic repair example.

### 8. Define AIL-Flow as a block/graph UI model, not only a view list

The no-code document currently defines view types and graph-patch editing. To reach your Scratch-like goal, add a concrete visual language model: block categories, sockets, connectors, cards, validation highlights, graph patch schema, block-to-AIL-Core mapping, and accessibility-friendly structured text equivalents. 

Recommended visual primitives:

Application Map nodes: actors, things, actions, external systems, tools, views.
Action Card blocks: trigger, inputs, requirements, reads, writes, calls, failures, approvals, guarantees, traces.
Rule blocks: condition, scope, source, dependent actions.
Failure blocks: trigger, compensation, response, trace.
Permission blocks: actor/capability/resource/effect/audience.
System blocks: resource, owner, borrow, region, layout, allocation, context, effect.
Compiler blocks: pass input, pass output, graph pattern, rewrite, diagnostic.

Definition of done: a JSON view model; patch format; UI validation states; round-trip from AIL-Core to view to patch to AIL-Core; at least one low-code visual edit fixture.

### 9. Add a standard library and package-system specification

Create `20-standard-library-and-packages.md`. The foundation says the mature toolchain must define the standard library, runtime primitives, package system, build system, and conformance suite in AIL. 

The first standard library should not be huge. It should define stable packages for:

core values: Text, Bool, Int, Decimal, Money, Time, Duration;
collections: List, Map, Set, Option, Result;
actions: validation, transformation, filtering, sorting;
effects: state, file, network, message, clock, random, process;
security: Secret, redaction, permissions, capabilities;
runtime: trace, diagnostics, failures, guarantees;
UI: form, table, route, event, view;
system: memory, layout, region, allocation, device, ABI;
compiler: graph traversal, graph patch, diagnostic emission, renderers.

Definition of done: package metadata format, versioning rules, import rules, capability grants, standard docs projection, and conformance fixtures for each package.

### 10. Add a C interop and ABI profile

Create `21-c-interop-abi.md`. This is mandatory for the desired outcome because “interop with C based libraries” is a separate semantic problem from systems programming.

Specify:

importing C functions;
importing structs, unions, enums, typedefs, constants, and macros where possible;
`repr(C)` layout rules;
pointer types and nullability;
borrowed pointer versus owned pointer;
mutable pointer rules;
lifetime and free rules;
callbacks and function pointers;
errno and error-code mapping into AIL `Failure`;
thread-safety annotations;
unsafe capability requirements;
dynamic library loading and static linking;
symbol naming and calling conventions;
header binding generation;
trace events for foreign calls;
secret redaction across FFI boundaries.

The System profile already has ABI-visible layout and resource/capability semantics, so C interop should reuse those rather than becoming a separate unsafe escape hatch. 

Definition of done: one C library import example, one callback example, one struct layout example, one ownership-transfer example, one rejected unsafe pointer example, and one trace explaining a foreign call.

### 11. Reframe the native compiler roadmap around portable targets

The implementation guide already defines a native Linux x86_64 ELF path that emits machine-code executable bytes directly, without generating Rust, invoking a linker, using libc, or relying on LLVM. 

That is a strong bootstrap artifact, but the desired outcome says portability should come from OS-ported compilers. Add `22-backend-portability.md` covering:

target triples;
OS ABI contracts;
syscall surfaces;
object/executable formats: ELF, Mach-O, PE/COFF, Wasm;
calling conventions;
runtime availability;
standard library target support;
capability mapping per OS;
native trace preservation;
backend conformance tests;
portable bytecode versus native executable boundaries.

Definition of done: Linux x86_64 as target 1, Wasm as portable sandbox target 2, one additional OS target plan, and a backend conformance manifest format.

### 12. Add a UI/UX application profile

The current Application profile covers apps, APIs, background jobs, workflows, services, dashboards, notifications, and integrations at a high level. 

To satisfy “advanced UI/UX applications,” add `23-ui-profile.md` with semantics for:

routes and navigation;
forms and validation;
views and view state;
components;
events;
accessibility;
responsive layout constraints;
local versus remote state;
optimistic updates;
permissions in UI;
error states;
traces for user interactions;
binding UI elements to AIL-Core actions.

Definition of done: CRUD app example, dashboard example, multi-step workflow example, accessibility trace, and AIL-Flow projection.

### 13. Add a kernel/system staged capability matrix

AIL-System is ambitious and generally well framed. It covers memory, layout, ownership, borrowing, regions, scheduling, concurrency, device capabilities, and low-level explanations. 

However, kernels, drivers, filesystems, network stacks, embedded software, runtimes, and compiler backends need staged capability levels. Add a matrix like:

System Level 0: resource/effect declarations only.
System Level 1: ownership, borrowing, regions, layout, allocation.
System Level 2: interrupts, scheduler tasks, locks, timing.
System Level 3: device register access and DMA.
System Level 4: filesystem/network stack components.
System Level 5: kernel/runtime self-hosting subset.

Definition of done: each level has accepted examples, rejected examples, diagnostics, traces, lowering obligations, and backend requirements.

### 14. Define the self-hosting subset explicitly

The bootstrap/self-hosting document has the right stages: bootstrap prototype, foundation specs, AIL-defined compiler rules, generated compiler, self-hosted fixed point, and legacy independence. 

But the docs should define the exact first self-hosting subset. Do not wait for the full language. Specify which features are sufficient to write parser rules, checker rules, renderers, diagnostics, graph normalization, bytecode lowering, and conformance checks in AIL-Meta.

Definition of done: `SelfHostCore v0` includes graph traversal, graph pattern matching, graph patch construction, diagnostics, deterministic sorting, hashing, serialization, parser rule definitions, renderer rules, bytecode emission, and conformance assertions.

### 15. Convert AIL-Meta from concept to executable language-definition packages

AIL-Meta currently says it represents language definition packages, parser rules, checker rules, diagnostics, renderers, agent prompts, compiler passes, lowering rules, optimizer rules, tests, metadata, and evolution proposals. 

That needs concrete syntax and IR mappings. Add one complete AIL-Meta package for a small feature, for example `Option<T>` or “Action reads field.” Include:

AIL-Spec form;
AIL-Core node/edge mapping;
checker rule;
diagnostic rule;
renderer rule;
AIL-Flow block rule;
prompt rule;
valid example;
invalid example;
round-trip fixture;
trace fixture;
migration notes.

Definition of done: the bootstrap compiler can consume this AIL-Meta package and generate at least one checker or renderer component.

### 16. Strengthen diagnostics as first-class language artifacts

The implementation guide already defines diagnostics with code, human-readable message, source provenance, affected graph node or edge, and repair suggestion. 

Add a diagnostic catalog with stable IDs. Each diagnostic should define:

condition;
affected node/edge;
message template;
non-engineer explanation;
agent follow-up question;
repair suggestion;
AIL-Flow highlight;
severity;
blocking behavior;
valid repair example.

Definition of done: every checker rule has a diagnostic, and every diagnostic has at least one invalid fixture.

### 17. Expand the training and conformance corpus from “needed” to “versioned asset”

The training corpus document correctly requires vague requests, interviews, specs, IR, no-code views, valid and invalid examples, diagnostics, runtime traces, debugging conversations, patches, round-trip examples, and conformance expectations. 

Now make this concrete. Add directories such as:

`corpus/interviews/`
`corpus/specs/accepted/`
`corpus/specs/rejected/`
`corpus/core/accepted/`
`corpus/core/rejected/`
`corpus/flow/`
`corpus/traces/`
`corpus/prompts/`
`corpus/roundtrip/`
`corpus/interop/`
`corpus/selfhost/`

Definition of done: every feature proposal must add corpus entries before acceptance.

### 18. Add missing example artifacts or make them visibly part of the docs

The readiness checklist requires paired examples for Support Ticket, Refund Tool, Compiler Pass, and Network Driver, with Support Ticket as the first executable conformance target. 

The uploaded document set references these examples, but the examples themselves were not included in the uploaded files I reviewed. To make the spec suite self-contained, either include the examples with the docs or add an explicit “example inventory” file showing path, status, test coverage, and last conformance result.

Definition of done: every referenced example has AIL-Spec, AIL-Core, AIL-Flow, trace, accepted/rejected fixtures, diagnostics, and round-trip tests.

### 19. Add a “semantic safety model” for non-engineer programming

Because AIL targets non-engineers, the language needs a safety layer above normal compiler correctness. Add a document defining which operations require confirmation, approval, permissions, or expert review.

The current AIL-Spec already says human confirmation is required before compiling inferred rules that affect permissions, effects, secrets, money, safety, or external calls. 

Extend this into policy:

low-risk changes can be accepted through normal review;
medium-risk changes require explicit confirmation;
high-risk changes require approval and trace;
dangerous system/FFI/kernel operations require expert-mode capability and conformance fixtures.

Definition of done: safety classification rules, UI review requirements, agent refusal/escalation rules, and audit traces.

### 20. Add a “desired outcome traceability matrix”

Create a matrix that maps each desired outcome to specific docs, artifacts, tests, and open gaps.

For example:

English-first authoring → `00`, `01`, `02`, prompt pack, interview corpus.
Deterministic strict IR → `03`, schema, normalization tests.
Non-engineer review → `02`, `04`, readability tests.
Visual editing → `04`, visual block schema, graph patch fixtures.
Turing completeness → new execution semantics doc.
Self-hosting → `10`, `13`, self-host subset, fixed-point tests.
Portable binaries → `15`, backend portability doc.
C interop → new C interop profile.
Full-stack support → Application, UI, AgentTool, System, Compiler profiles.
Round-trip semantic equivalence → `11`, conformance suite, prompt portability tests.

Definition of done: no desired outcome is supported only by prose; each one has at least one schema, example, checker rule, conformance test, and artifact boundary.

## Suggested document changes by file

`00-foundation.md`: Add an explicit “AIL must become Turing complete” invariant, but phrase it as a semantic requirement over AIL-Core rather than surface English. Add a short note that non-engineer friendliness does not mean the language avoids formal execution semantics.

`01-language-architecture.md`: Add a compiler-stage diagram that includes AIL-Requirements, AIL-Spec Canonical, AIL-Core, AIL-Bytecode, backend-specific native artifacts, C interop bindings, and visual patch validation.

`02-structured-spec.md`: Add canonical grammar sections for control flow, function/action calls, loops, recursion, external calls, UI views, C imports, library imports, and standard library usage.

`03-semantic-ir.md`: Convert the node/edge lists into a schema catalog. Add formal graph patch schema, semantic hash rules, versioned package schema, and a canonical serialization format.

`04-no-code-views.md`: Add a Scratch-like block model and exact patch payloads. Define which visual edits are allowed directly and which require agent clarification.

`05-agent-protocol.md`: Add the actual prompt pack references, output schemas, prompt versioning, model portability tests, and failure-handling behavior when an LLM cannot resolve ambiguity.

`06-agent-tools.md`: Add runtime tool-call authorization flow, sandboxing rules, external API binding rules, and stronger secret-redaction fixtures.

`07-types-values-effects.md`: Add callable types, pointer/reference types, function/action signatures, generic constraints, collection operations, and FFI-safe types.

`08-failures-guarantees-traces.md`: Add failure semantics for recursion, loops, async/concurrent actions, external calls, C interop, memory faults, and backend lowering failures.

`09-system-profile.md`: Keep the current resource/capability model, but add staged system levels and a C ABI subsection or cross-reference to the new interop profile.

`10-meta-profile.md`: Add one complete executable AIL-Meta language-feature package as the model for all future features.

`11-round-trip-equivalence.md`: Add algorithms and fixtures, not only definitions. Include model-output equivalence and semantic explanation equivalence tests.

`12-training-corpus.md`: Turn the corpus requirements into directory structure, artifact formats, and acceptance metrics.

`13-bootstrap-self-hosting.md`: Add the exact `SelfHostCore v0` subset and the minimum fixed-point proof required before deprecating Rust as anything other than bootstrap scaffolding.

`14-evolution-protocol.md`: Make “Turing completeness,” “C interop,” “standard library,” “visual block model,” and “backend portability” explicit proposal tracks. The current protocol already defines readability, LLM teachability, and compiler checkability gates, which are the right acceptance gates. 

`15-toolchain-implementation-guide.md`: Split the very large guide into implementation phases. Keep the first vertical slice narrow, but add separate milestones for Turing core, prompt pack, AIL-Flow patching, C interop, UI profile, System profile, AIL-Meta self-host subset, bytecode VM, and native backends.

`16-implementation-readiness-checklist.md`: Add a second checklist: “ready for language MVP,” separate from “ready for first vertical slice.” The current checklist is suitable for starting implementation, but not sufficient to claim the full desired language outcome. 

## Proposed milestone sequence

Milestone 1 should be “semantic MVP,” not native compilation. Build package loading, canonical AIL-Spec parsing, AIL-Core schema, checker, renderer, round-trip tests, diagnostics, and one trace runtime. This is consistent with the implementation guide’s first vertical slice. 

Milestone 2 should be “Turing Core.” Add control flow, action/function calls, recursion or loops, collections, state mutation, and operational semantics. Prove AIL-Core can execute non-trivial programs independently of LLMs.

Milestone 3 should be “Agent Prompt Pack.” Implement interview, requirements capture, spec drafting, repair, IR rendering, trace explanation, and patch prompts with schemas and conformance tests.

Milestone 4 should be “Visual Review and Patch.” Implement AIL-Flow block/card views, graph patches, patch validation, and round-trip visual editing.

Milestone 5 should be “Standard Library and Packages.” Define core packages and imports, then test package versioning, capability grants, and standard library projections.

Milestone 6 should be “Bytecode and VM.” Lower checked AIL-Core to AIL-Bytecode, verify saved bytecode, and run through a VM artifact boundary.

Milestone 7 should be “Native Target 1.” Continue with Linux x86_64 ELF, but define portability contracts before adding more targets.

Milestone 8 should be “C Interop.” Add ABI-safe imports, pointer ownership, failure mapping, and traceable foreign calls.

Milestone 9 should be “AIL-Meta Compiler Rules.” Move checker, renderer, parser, and lowering rule definitions into AIL-Meta packages.

Milestone 10 should be “Self-host fixed point.” Use the bootstrap compiler to generate the next compiler from AIL-defined toolchain specs, then prove fixed-point equivalence.

## Bottom-line recommendation

The current documents are good enough to start a first vertical slice, but not yet sufficient to claim the full AIL outcome. The highest-priority changes are: formal execution semantics, explicit Turing-complete core, machine-checkable AIL-Core schema, real prompt pack, round-trip algorithms, visual block/patch schema, standard library/package spec, C interop profile, backend portability spec, and a self-hosting subset.

I would not expand the implementation aggressively until those are written. Otherwise, the project risks implementing a sophisticated prototype whose actual language semantics are still implicit in Rust code, prompt behavior, or examples rather than owned by AIL itself.
