# Open Questions

This file records unresolved design choices.

## 1. How strict should RSL be?

Options:

```text
A. Very strict controlled English, easy to parse.
B. Moderately flexible English with deterministic phrase matching.
C. LLM-assisted loose English with RIF review and strict checker.
```

Recommended path:

```text
Start with A.
Add B gradually.
Allow C only as an authoring assistant, never as final authority.
```

## 2. What is the exact RIF syntax?

RIF could be:

```text
indentation-based Markdown-like text
YAML-like structured text
S-expression-like normalized format
JSON/TOML plus explanation layer
projectional-editor-only model
```

Recommended path:

```text
Use indentation-based Markdown-like RIF for the prototype.
Also provide JSON serialization for EIG-Core.
```

## 3. How formal should contracts be initially?

Options:

```text
A. Plain text contracts, checked only for reference validity.
B. Expression language with static checks.
C. SMT-backed formal constraints.
D. Proof-assistant-level proofs.
```

Recommended path:

```text
Start with B for simple expressions.
Add C for selected domains.
Do not begin with D.
```

## 4. What is the first backend?

Options:

```text
custom bytecode interpreter
Wasm backend
MLIR/LLVM backend
workflow engine backend
Rust transpilation as temporary experiment
```

Recommended path:

```text
Start with custom bytecode interpreter for process semantics.
Add Wasm for pure computations.
Consider MLIR/LLVM later for native performance.
Avoid Rust/Python/JS transpilation as the conceptual execution model.
```

## 5. How should persistent state map to ownership?

In-memory values can use ownership and borrow-like permissions.
Persistent entities require a different backend mechanism.

Possible lowerings:

```text
transaction locks
optimistic version checks
actor mailbox ownership
event-sourced command tokens
linear write capabilities
```

Open question:

```text
Should EIGL expose these as backend policies, or should they be part of the core effect system?
```

## 6. What is the minimum trusted kernel?

Candidate trusted kernel responsibilities:

```text
EIG-Core schema validation
reference resolution
type checking
permission checking
effect checking
failure completeness checking
contract/proof-object checking
backend obligation checking
```

Open question:

```text
Which of these can be generated safely, and which must remain manually audited?
```

## 7. Should RIF be editable by non-programmers?

RSL is clearly for non-programmers. RIF is more explicit.

Open question:

```text
Should non-programmers edit RIF directly, or only review it through views and explanations?
```

Likely answer:

```text
RIF should be readable by non-programmers but primarily edited by expert users, LLMs, or guided tools.
```

## 8. How should ambiguity be represented?

Possibilities:

```text
unresolved question nodes in RIF
compiler diagnostics only
interactive editor prompts
LLM clarification dialogs
```

Recommended path:

```text
Represent ambiguities as explicit unresolved question objects in RIF.
Compilation fails until resolved.
```

## 9. How should visual edits be validated?

A visual edit creates a graph patch.

Open questions:

```text
What patch format should be used?
How are partial edits represented?
How does the editor display invalid intermediate states?
Can an invalid graph be temporarily edited before being accepted?
```

Recommended path:

```text
Allow transient invalid editor states, but only accept valid graph patches into EIG-Core.
```

## 10. How should LLMs interact with the system?

Possible interfaces:

```text
RSL authoring
RIF patch generation
EIG-Core patch generation
explanation generation
ambiguity clarification
test generation
compiler-rule drafting
```

Recommended path:

```text
LLMs propose RIF/EIG-Core patches.
The checker validates patches.
LLMs explain validated RIF/EIG-Core, not raw guesses.
```

## 11. How close to English should the final surface be?

The tension:

```text
more English -> easier for humans, more ambiguity
more structure -> easier for compilers, less natural
```

Likely solution:

```text
Use guided controlled English with editor support, phrase dictionaries, and progressive disclosure.
```

## 12. How should compiler generation be bootstrapped?

Open questions:

```text
What seed language should be used?
How large should the seed compiler be?
When is self-hosting considered achieved?
Should the backend be self-hosted early or late?
```

Recommended path:

```text
Seed compiler in Rust or Python for speed of development.
Trusted checker in Rust or another systems language for auditability.
Self-host RIF compiler before RSL compiler.
Backend self-hosting comes later.
```

## 13. What should the first real domain be?

Candidates:

```text
commerce workflows
access control
password reset/authentication
data pipelines
embedded device state machines
ML model pipelines
```

Recommended first domain:

```text
commerce workflows or access control
```

Reason:

```text
They naturally demonstrate state transitions, effects, failures, permissions, compensation, and visual explanations.
```

## 14. How should performance claims be validated?

Needed benchmarks:

```text
pure computation
state-machine execution
workflow orchestration
data processing
allocation-heavy object construction
parallel independent steps
foreign/trusted capsule calls
```

Open question:

```text
At what point can EIGL honestly claim Rust-like performance for a subset?
```

Likely answer:

```text
Only after a native backend and cost model are implemented and benchmarked.
```

## 15. What are the formal semantics?

Eventually EIGL needs formal definitions for:

```text
values
types
ownership/permissions
lifetimes/effect scopes
state
resources
time
failure
contracts
refinement
lowering correctness
```

Recommended path:

```text
Start with operational semantics for a small core.
Add type/permission/effect soundness proofs for the safe subset.
```
