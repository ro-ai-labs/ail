# AIL Language Specification Work Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce the first coherent AIL language specification suite, grounded in the foundation design and ready to guide later toolchain implementation.

**Architecture:** Treat the AIL foundation design as the language constitution, then split the language into focused specification documents with examples, conformance expectations, and round-trip requirements. This plan is for specification work only; implementation of the compiler/toolchain gets a separate plan after the specs are reviewed.

**Tech Stack:** Markdown specifications in `docs/`, current Rust prototype as background context only, shell verification with `rg`, Markdown link checks, and reviewer-driven iteration.

---

## Scope

This plan produces specification documents, examples, and review artifacts. It
does not implement compiler features.

The work should preserve existing prototype files unless a later task explicitly
renames or archives them. The current dirty Rust prototype is out of scope for
this plan.

Old EIGL definition documents must not remain in the active `docs/` root. If
they are kept, they must live under `docs/archive/eigl-prototype/` with an
explicit historical warning.

## File Structure

Create a new AIL specification area:

```text
docs/ail/
  README.md
  00-foundation.md
  01-language-architecture.md
  02-structured-spec.md
  03-semantic-ir.md
  04-no-code-views.md
  05-agent-protocol.md
  06-agent-tools.md
  07-types-values-effects.md
  08-failures-guarantees-traces.md
  09-system-profile.md
  10-meta-profile.md
  11-round-trip-equivalence.md
  12-training-corpus.md
  13-bootstrap-self-hosting.md
  14-evolution-protocol.md
  examples/
    support-ticket.ail-spec.md
    support-ticket.ail-core.md
    refund-tool.ail-spec.md
    refund-tool.ail-core.md
    compiler-pass.ail-spec.md
    compiler-pass.ail-core.md
```

Keep this foundation document as design provenance:

```text
docs/superpowers/specs/2026-05-22-ail-language-foundation-design.md
```

Keep this plan as execution guidance:

```text
docs/superpowers/plans/2026-05-22-ail-language-specification-work.md
```

Keep old EIGL docs only as archived provenance:

```text
docs/archive/eigl-prototype/
```

The active `docs/README.md` must point readers to AIL documents, not old EIGL
definitions.

## Task 0: Clean Up The Active Docs Root

**Files:**
- Modify: `docs/README.md`
- Create: `docs/archive/eigl-prototype/README.md`
- Move: `docs/00-codex-handoff.md` to `docs/archive/eigl-prototype/00-codex-handoff.md`
- Move: `docs/01-vision-and-principles.md` to `docs/archive/eigl-prototype/01-vision-and-principles.md`
- Move: `docs/02-language-architecture.md` to `docs/archive/eigl-prototype/02-language-architecture.md`
- Move: `docs/03-rsl-requirement-surface-language.md` to `docs/archive/eigl-prototype/03-rsl-requirement-surface-language.md`
- Move: `docs/04-rif-readable-intent-format.md` to `docs/archive/eigl-prototype/04-rif-readable-intent-format.md`
- Move: `docs/05-eig-core-semantic-graph.md` to `docs/archive/eigl-prototype/05-eig-core-semantic-graph.md`
- Move: `docs/06-safety-permissions-effects.md` to `docs/archive/eigl-prototype/06-safety-permissions-effects.md`
- Move: `docs/07-visualization-and-views.md` to `docs/archive/eigl-prototype/07-visualization-and-views.md`
- Move: `docs/08-compiler-generation.md` to `docs/archive/eigl-prototype/08-compiler-generation.md`
- Move: `docs/09-self-hosting-bootstrap.md` to `docs/archive/eigl-prototype/09-self-hosting-bootstrap.md`
- Move: `docs/10-prototype-roadmap.md` to `docs/archive/eigl-prototype/10-prototype-roadmap.md`
- Move: `docs/11-examples.md` to `docs/archive/eigl-prototype/11-examples.md`
- Move: `docs/12-open-questions.md` to `docs/archive/eigl-prototype/12-open-questions.md`
- Move: `docs/EIGL_FULL_SPEC.md` to `docs/archive/eigl-prototype/EIGL_FULL_SPEC.md`

- [x] **Step 1: Archive old EIGL documents**

Run:

```bash
mkdir -p docs/archive/eigl-prototype
git mv docs/00-codex-handoff.md docs/01-vision-and-principles.md docs/02-language-architecture.md docs/03-rsl-requirement-surface-language.md docs/04-rif-readable-intent-format.md docs/05-eig-core-semantic-graph.md docs/06-safety-permissions-effects.md docs/07-visualization-and-views.md docs/08-compiler-generation.md docs/09-self-hosting-bootstrap.md docs/10-prototype-roadmap.md docs/11-examples.md docs/12-open-questions.md docs/EIGL_FULL_SPEC.md docs/archive/eigl-prototype/
```

Expected: old EIGL definition documents are no longer direct children of
`docs/`.

- [x] **Step 2: Replace the active docs index**

Write `docs/README.md` so it identifies AIL as the active language direction and
links only to active AIL work plus the historical archive.

- [x] **Step 3: Label the archive**

Write `docs/archive/eigl-prototype/README.md` so it says the archived EIGL files
are historical provenance only and not active specification authority.

- [x] **Step 4: Verify active docs root is clean**

Run:

```bash
find docs -maxdepth 1 -type f | sort
```

Expected: only `docs/README.md` appears as a top-level file.

- [x] **Step 5: Verify EIGL docs are archive-only**

Run:

```bash
find docs -maxdepth 3 -path 'docs/archive/eigl-prototype' -prune -o -type f -print | sort
```

Expected: active docs outside the archive are AIL-oriented docs only.

## Task 1: Establish The AIL Spec Index

**Files:**
- Create: `docs/ail/README.md`

- [x] **Step 1: Create the spec index**

Write `docs/ail/README.md` with this structure:

```markdown
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

## Status

These documents define the first AIL specification suite. They are precise
enough to guide implementation, but still versioned and expected to evolve as
examples, round-trip tests, no-code projections, and compiler prototypes expose
gaps.

## Prototype History

This repository previously explored the language under the EIGL name. New
language design uses AIL names. Existing EIGL prototype code and examples remain
historical implementation scaffolding until migration is planned explicitly.
```

- [x] **Step 2: Verify the index exists**

Run:

```bash
test -f docs/ail/README.md
```

Expected: exit code 0.

## Task 2: Write The Foundation Specification

**Files:**
- Create: `docs/ail/00-foundation.md`
- Read: `docs/superpowers/specs/2026-05-22-ail-language-foundation-design.md`

- [x] **Step 1: Copy and refine the foundation design**

Create `docs/ail/00-foundation.md` by adapting the foundation design into a
reader-facing specification. Keep these sections:

```markdown
# AIL Foundation

## Name

## Goal

## Core Thesis

## Non-Negotiable Invariants

## Layer Model

## Program Profiles

## Core Vocabulary

## Self-Sovereign Toolchain Principle

## Readability Gate

## Flexibility Rule
```

Use the same substance as the foundation design. Remove process-only notes that
belong in the work plan.

- [x] **Step 2: Check required foundation terms**

Run:

```bash
rg -n "Agentic Intent Language|AIL-Core|AIL-Spec|Self-Sovereign Toolchain|Readability Gate|Round-Trip" docs/ail/00-foundation.md
```

Expected: each required term appears at least once.

## Task 3: Specify The Language Architecture

**Files:**
- Create: `docs/ail/01-language-architecture.md`

- [x] **Step 1: Write architecture document**

Create `docs/ail/01-language-architecture.md` with these sections:

```markdown
# AIL Language Architecture

## Pipeline

## Accepted Program Artifact

## Projection Model

## Trusted And Untrusted Components

## Profiles Over One Semantic Substrate

## Compiler Boundary

## Runtime Boundary

## Migration From The Existing Prototype
```

The accepted program artifact must be `AIL-Core` plus validated metadata. State
clearly that English conversation is not compiled directly.

- [x] **Step 2: Verify source-of-truth wording**

Run:

```bash
rg -n "AIL-Core.*source of truth|conversation is not compiled directly|trusted checker|untrusted" docs/ail/01-language-architecture.md
```

Expected: each concept appears in the architecture document.

## Task 4: Specify AIL-Spec Structured English

**Files:**
- Create: `docs/ail/02-structured-spec.md`
- Create: `docs/ail/examples/support-ticket.ail-spec.md`

- [x] **Step 1: Write AIL-Spec document**

Create `docs/ail/02-structured-spec.md` with these sections:

```markdown
# AIL-Spec Structured English

## Purpose

## Required Qualities

## Document Shape

## Application Sections

## Action Sections

## Tool Sections

## Failure Sections

## Secret And Permission Sections

## Human Confirmation Rules

## Invalid Or Ambiguous Specs
```

Define the regular structured-English slots for applications and actions:

```text
The application ...

A <thing> has:
- <field>

When <actor/event> <action>:
- the system requires ...
- the system reads ...
- the system changes ...
- the system calls ...
- if ... fails ...
- the system guarantees ...
```

- [x] **Step 2: Write support ticket example**

Create `docs/ail/examples/support-ticket.ail-spec.md` with a complete support
ticket application covering:

- Ticket thing
- User thing
- Create ticket action
- Assign ticket action
- Close ticket action
- Customer-visible update rule
- Internal notes secrecy rule
- Overdue ticket view

- [x] **Step 3: Verify example coverage**

Run:

```bash
rg -n "Create ticket|Assign ticket|Close ticket|internal notes|Overdue" docs/ail/examples/support-ticket.ail-spec.md
```

Expected: all named concepts appear.

## Task 5: Specify AIL-Core Semantic IR

**Files:**
- Create: `docs/ail/03-semantic-ir.md`
- Create: `docs/ail/examples/support-ticket.ail-core.md`

- [x] **Step 1: Write semantic IR document**

Create `docs/ail/03-semantic-ir.md` with these sections:

```markdown
# AIL-Core Semantic IR

## Purpose

## Graph Model

## Stable Identity

## Node Kinds

## Edge Kinds

## Attributes

## Provenance

## Normalization

## Equivalence

## Serialization Expectations
```

Include initial node kinds:

```text
Application, Thing, Field, Action, Step, Tool, Event, Rule, View, Value,
Permission, Effect, Failure, Guarantee, Secret, Approval, Trace, Region,
Layout, Lowering, Diagnostic
```

- [x] **Step 2: Write support ticket IR example**

Create `docs/ail/examples/support-ticket.ail-core.md` as a readable pseudo-IR
matching the support ticket AIL-Spec example. It should show stable IDs,
nodes, edges, permissions, failures, guarantees, and provenance.

- [x] **Step 3: Verify IR alignment**

Run:

```bash
rg -n "Ticket|CreateTicket|AssignTicket|CloseTicket|Permission|Guarantee|Provenance" docs/ail/examples/support-ticket.ail-core.md
```

Expected: all named concepts appear.

## Task 6: Specify No-Code Views

**Files:**
- Create: `docs/ail/04-no-code-views.md`

- [x] **Step 1: Write no-code projection document**

Create `docs/ail/04-no-code-views.md` with these sections:

```markdown
# AIL-Flow No-Code Views

## Purpose

## View Types

## Application Map

## Action Cards

## Data Tables

## Rule Lists

## Permission Views

## Failure Maps

## Trace Views

## Editing Through Views

## Validation Of View Patches
```

State that no-code views are deterministic projections of AIL-Core and that
view edits produce graph patches, not opaque text edits.

- [x] **Step 2: Verify no-code invariants**

Run:

```bash
rg -n "deterministic projection|graph patches|Action Cards|Permission Views|Trace Views" docs/ail/04-no-code-views.md
```

Expected: all concepts appear.

## Task 7: Specify The Agent Protocol

**Files:**
- Create: `docs/ail/05-agent-protocol.md`

- [x] **Step 1: Write agent protocol document**

Create `docs/ail/05-agent-protocol.md` with these sections:

```markdown
# AIL-Agent Protocol

## Purpose

## Agent Responsibilities

## Interview Loop

## Required Coverage

## Patch Discipline

## Conversion Tasks

## Prompt Compatibility Standard

## Calibration Examples

## Trust Boundary

## Failure Modes
```

Include the rule: the AI Agent is part of the toolchain but not part of the
trusted compiler core.

- [x] **Step 2: Verify trust-boundary wording**

Run:

```bash
rg -n "part of the toolchain|trusted compiler core|Patch Discipline|Prompt Compatibility" docs/ail/05-agent-protocol.md
```

Expected: all concepts appear.

## Task 8: Specify Agent Tools

**Files:**
- Create: `docs/ail/06-agent-tools.md`
- Create: `docs/ail/examples/refund-tool.ail-spec.md`
- Create: `docs/ail/examples/refund-tool.ail-core.md`

- [x] **Step 1: Write agent tool document**

Create `docs/ail/06-agent-tools.md` with these sections:

```markdown
# AIL Agent Tools

## Purpose

## Tool Contract

## Inputs And Outputs

## Permissions And Capabilities

## Effects

## Secrets

## Human Approval

## Audit Trace

## Runtime Enforcement

## Example: Refund Customer Payment
```

- [x] **Step 2: Write refund tool spec example**

Create `docs/ail/examples/refund-tool.ail-spec.md` using the refund example from
the foundation design, including approval over USD 500 and payment-token secrecy.

- [x] **Step 3: Write refund tool IR example**

Create `docs/ail/examples/refund-tool.ail-core.md` with nodes and edges for
inputs, rules, payment provider call, refund ledger write, approval rule,
secrets, failures, and guarantees.

- [x] **Step 4: Verify tool coverage**

Run:

```bash
rg -n "Refund|approval|payment token|PaymentProvider|RefundLedger|Guarantee" docs/ail/06-agent-tools.md docs/ail/examples/refund-tool.ail-spec.md docs/ail/examples/refund-tool.ail-core.md
```

Expected: all concepts appear across the three files.

## Task 9: Specify Types, Values, Permissions, And Effects

**Files:**
- Create: `docs/ail/07-types-values-effects.md`

- [x] **Step 1: Write type/effect document**

Create `docs/ail/07-types-values-effects.md` with these sections:

```markdown
# AIL Types, Values, Permissions, And Effects

## Purpose

## Core Types

## Structured Values

## Option And Result

## Secret Values

## Permissions

## Capabilities

## Effects

## Ownership And Sharing

## Human Explanation Rules
```

The document must connect high-level permissions to systems-level ownership and
capability checks.

- [x] **Step 2: Verify safety vocabulary**

Run:

```bash
rg -n "Secret|Permission|Capability|Effect|Ownership|Sharing|Option|Result" docs/ail/07-types-values-effects.md
```

Expected: all concepts appear.

## Task 10: Specify Failures, Guarantees, And Traces

**Files:**
- Create: `docs/ail/08-failures-guarantees-traces.md`

- [x] **Step 1: Write failure/trace document**

Create `docs/ail/08-failures-guarantees-traces.md` with these sections:

```markdown
# AIL Failures, Guarantees, And Traces

## Purpose

## Declared Failures

## Failure Handling

## Compensation

## Guarantees

## Trace Events

## Interactive Debugging

## Human Diagnosis Requirements

## Systems-Level Debugging
```

Include examples for application debugging and systems allocation debugging.

- [x] **Step 2: Verify debugging vocabulary**

Run:

```bash
rg -n "Interactive Debugging|Human Diagnosis|Trace Events|Compensation|allocation" docs/ail/08-failures-guarantees-traces.md
```

Expected: all concepts appear.

## Task 11: Specify The Systems Profile

**Files:**
- Create: `docs/ail/09-system-profile.md`

- [x] **Step 1: Write systems profile document**

Create `docs/ail/09-system-profile.md` with these sections:

```markdown
# AIL-System Profile

## Purpose

## Systems Scope

## Memory And Layout

## Ownership And Borrowing

## Regions And Lifetimes

## Scheduling And Concurrency

## Device And OS Capabilities

## Lowering Obligations

## Human Explanations For Low-Level Semantics
```

State that AIL-System must be able to express kernels, runtimes, drivers, and
compiler backends over time.

- [x] **Step 2: Verify systems scope**

Run:

```bash
rg -n "kernel|runtime|driver|ownership|region|layout|lowering" docs/ail/09-system-profile.md
```

Expected: all concepts appear.

## Task 12: Specify The Meta Profile

**Files:**
- Create: `docs/ail/10-meta-profile.md`
- Create: `docs/ail/examples/compiler-pass.ail-spec.md`
- Create: `docs/ail/examples/compiler-pass.ail-core.md`

- [x] **Step 1: Write meta profile document**

Create `docs/ail/10-meta-profile.md` with these sections:

```markdown
# AIL-Meta Profile

## Purpose

## Language Definition Packages

## Compiler Passes As Actions

## Checker Rules

## Diagnostic Rules

## Renderer Rules

## Agent Prompt Rules

## Lowering Rules

## Self-Hosting Role
```

- [x] **Step 2: Write compiler pass spec example**

Create `docs/ail/examples/compiler-pass.ail-spec.md` for a compiler pass named
`Infer read permissions`. It should explain inputs, outputs, steps, failures,
and guarantees in structured English.

- [x] **Step 3: Write compiler pass IR example**

Create `docs/ail/examples/compiler-pass.ail-core.md` matching the compiler pass
spec. Include graph nodes for pass input, pass output, rule application,
diagnostic, and guarantee.

- [x] **Step 4: Verify meta coverage**

Run:

```bash
rg -n "Infer read permissions|Compiler Passes|Checker Rules|Diagnostic Rules|Self-Hosting" docs/ail/10-meta-profile.md docs/ail/examples/compiler-pass.ail-spec.md docs/ail/examples/compiler-pass.ail-core.md
```

Expected: all concepts appear across the three files.

## Task 13: Specify Round-Trip Equivalence

**Files:**
- Create: `docs/ail/11-round-trip-equivalence.md`

- [x] **Step 1: Write equivalence document**

Create `docs/ail/11-round-trip-equivalence.md` with these sections:

```markdown
# AIL Round-Trip And Equivalence

## Purpose

## Required Round Trips

## Strong Equivalence

## Behavioral Equivalence

## Explanation Equivalence

## Embedding Distance As Regression Signal

## Equivalence Failures

## Conformance Tests
```

State explicitly that embedding distance is not the compiler authority.

- [x] **Step 2: Verify equivalence wording**

Run:

```bash
rg -n "Strong Equivalence|Behavioral Equivalence|Explanation Equivalence|embedding distance is not the compiler authority" docs/ail/11-round-trip-equivalence.md
```

Expected: all concepts appear.

## Task 14: Specify Training Corpus And Conformance

**Files:**
- Create: `docs/ail/12-training-corpus.md`

- [x] **Step 1: Write training corpus document**

Create `docs/ail/12-training-corpus.md` with these sections:

```markdown
# AIL Training Corpus And Conformance

## Purpose

## Example Artifact Set

## Fine-Tuning Data

## Prompt Calibration Data

## Invalid Examples

## Diagnostics Dataset

## Trace Dataset

## Human Review Dataset

## Conformance Suite
```

Include the required paired artifact list from the foundation design.

- [x] **Step 2: Verify corpus vocabulary**

Run:

```bash
rg -n "fine-tuning|Prompt Calibration|Invalid Examples|Diagnostics Dataset|Trace Dataset|Conformance Suite" docs/ail/12-training-corpus.md
```

Expected: all concepts appear.

## Task 15: Specify Bootstrap And Self-Hosting

**Files:**
- Create: `docs/ail/13-bootstrap-self-hosting.md`

- [x] **Step 1: Write bootstrap document**

Create `docs/ail/13-bootstrap-self-hosting.md` with these sections:

```markdown
# AIL Bootstrap And Self-Hosting

## Purpose

## Bootstrap Allowance

## Self-Sovereign Toolchain Principle

## Stage 0: Bootstrap Prototype

## Stage 1: AIL Foundation Specs

## Stage 2: AIL-Defined Compiler Rules

## Stage 3: Generated AIL Compiler

## Stage 4: Self-Hosted Fixed Point

## Stage 5: Legacy Independence

## Fixed-Point Checks
```

State that legacy languages may bootstrap AIL but must not own AIL.

- [x] **Step 2: Verify self-hosting wording**

Run:

```bash
rg -n "legacy languages may bootstrap AIL but must not own AIL|Self-Hosted Fixed Point|Legacy Independence|Fixed-Point Checks" docs/ail/13-bootstrap-self-hosting.md
```

Expected: all concepts appear.

## Task 16: Specify Language Evolution

**Files:**
- Create: `docs/ail/14-evolution-protocol.md`

- [x] **Step 1: Write evolution protocol document**

Create `docs/ail/14-evolution-protocol.md` with these sections:

```markdown
# AIL Language Evolution Protocol

## Purpose

## Stable Invariants

## Versioned Decisions

## Experimental Surfaces

## Proposal Requirements

## Readability Gate

## LLM Teachability Gate

## Compiler Checkability Gate

## Acceptance Process
```

Include the full proposal checklist from the foundation design.

- [x] **Step 2: Verify evolution gates**

Run:

```bash
rg -n "Readability Gate|LLM Teachability Gate|Compiler Checkability Gate|Proposal Requirements" docs/ail/14-evolution-protocol.md
```

Expected: all concepts appear.

## Task 17: Add Local Link And Placeholder Verification

**Files:**
- Modify: `docs/ail/README.md`

- [x] **Step 1: Check for placeholder text**

Run:

```bash
rg -n "TBD|TODO|FIXME|implement later|fill in details" docs/ail docs/superpowers/specs
```

Expected: no matches.

- [x] **Step 2: Check README links**

Run:

```bash
while IFS= read -r path; do test -f "docs/ail/$path" || { echo "missing $path"; exit 1; }; done <<'EOF'
00-foundation.md
01-language-architecture.md
02-structured-spec.md
03-semantic-ir.md
04-no-code-views.md
05-agent-protocol.md
06-agent-tools.md
07-types-values-effects.md
08-failures-guarantees-traces.md
09-system-profile.md
10-meta-profile.md
11-round-trip-equivalence.md
12-training-corpus.md
13-bootstrap-self-hosting.md
14-evolution-protocol.md
EOF
```

Expected: exit code 0.

- [x] **Step 3: Check Markdown whitespace**

Run:

```bash
git diff --check -- docs/ail docs/superpowers/specs docs/superpowers/plans
```

Expected: no output and exit code 0.

## Task 18: Review Against Foundation Invariants

**Files:**
- Read: `docs/superpowers/specs/2026-05-22-ail-language-foundation-design.md`
- Read: `docs/ail/*.md`
- Read: `docs/ail/examples/*.md`

- [x] **Step 1: Verify invariant coverage**

Run:

```bash
rg -n "English starts|canonical semantic|AI Agent|Round-Trip|Self-Sovereign|Readability Gate|LLM" docs/ail
```

Expected: each foundation invariant appears in at least one spec document.

- [x] **Step 2: Manual review checklist**

Read each `docs/ail/*.md` file and confirm:

- The document has a clear purpose.
- The document avoids committing to implementation details prematurely.
- The document preserves English-to-IR-to-English round-tripping.
- The document keeps the trusted checker separate from the AI Agent.
- The document keeps non-engineer review and diagnosis visible.
- The document supports later self-hosting in AIL.

- [x] **Step 3: Record review notes**

If review finds gaps, update the relevant spec document immediately. If review
finds a larger unresolved decision, add it to `docs/ail/14-evolution-protocol.md`
under an "Open Decisions For Later Specification" section with concrete options
and the current recommended default.

## Task 19: Stop For User Review

**Files:**
- Read: `docs/ail/README.md`
- Read: `docs/superpowers/plans/2026-05-22-ail-language-specification-work.md`

- [x] **Step 1: Summarize the produced spec suite**

Prepare a concise summary listing:

- which files were created
- what each file defines
- what verification commands passed
- any open decisions intentionally left for later

- [x] **Step 2: Ask for review**

Ask the user to review `docs/ail/README.md`,
`docs/ail/00-foundation.md`, and the example files first. Do not begin
compiler/toolchain implementation until the language specification direction is
approved.

## Task 20: Add Implementation Readiness Bridge

**Files:**
- Create: `docs/ail/15-toolchain-implementation-guide.md`
- Create: `docs/ail/16-implementation-readiness-checklist.md`
- Modify: `docs/ail/README.md`
- Modify: `docs/README.md`

- [x] **Step 1: Define the first toolchain vertical slice**

Write `docs/ail/15-toolchain-implementation-guide.md` so it identifies the
Support Ticket example as the first implementation target and names the minimum
toolchain components: package loader, AIL-Spec parser, elaborator, AIL-Core
store, checker, renderer, trace runtime, and diagnostics.

- [x] **Step 2: Define the development-start gate**

Write `docs/ail/16-implementation-readiness-checklist.md` so it lists required
documents, required examples, validation commands, manual review criteria, and
the decision rule for starting compiler and runtime implementation.

- [x] **Step 3: Link the readiness bridge**

Update `docs/ail/README.md` and `docs/README.md` so readers can find the
implementation guide and readiness checklist from the active documentation
indexes.

- [x] **Step 4: Verify implementation-readiness vocabulary**

Run:

```bash
rg -n "Package Loader|AIL-Spec Parser|Elaborator|AIL-Core Store|Checker|Renderer|Trace Runtime|Diagnostics" docs/ail/15-toolchain-implementation-guide.md
rg -n "Development Start Decision|Required Documentation|Validation Commands|Manual Review Gate" docs/ail/16-implementation-readiness-checklist.md
```

Expected: all implementation-readiness terms appear in the new bridge docs.

## Self-Review Checklist For This Plan

- The plan is specification-only.
- The plan does not require modifying Rust prototype code.
- The plan gives exact paths for every file.
- The plan includes concrete verification commands.
- The plan includes review gates before implementation.
- The plan keeps AIL flexible but preserves non-negotiable invariants.
