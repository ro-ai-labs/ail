# AIL Desired Outcome Traceability

## Purpose

This matrix maps desired AIL outcomes to concrete documents, artifacts, schema,
examples, checker rules, conformance tests, and artifact boundaries. No desired
outcome is supported only by strategy prose.

## Matrix

| Desired outcome | Documents | Schema or artifact | Example | Checker or diagnostic | Conformance boundary |
| --- | --- | --- | --- | --- | --- |
| English-first authoring | `00`, `01`, `02`, `05`, `19` | AIL-Requirements, AIL-Spec Canonical, prompt envelope | Support Ticket interview and spec | `AIL-PROMPT-001`, ambiguity rejection | prompt corpus normalizes to AIL-Core or asks questions |
| Deterministic semantic IR | `03`, `18` | `ail-core.schema.v0`, stable hash | Support Ticket AIL-Core | `AIL-TYPE-001`, schema validation | canonical render/reparse hash |
| Non-engineer review | `02`, `04`, `19`, `26` | AIL-Spec Friendly, AIL-Flow blocks, safety review | Refund approval review | safety diagnostics | friendly projection round trip |
| Visual editing | `04`, `18`, `23` | graph patch schema, block model | Escalate ticket patch | `AIL-ROUNDTRIP-001` | view -> patch -> core checker |
| Turing completeness | `17`, `07`, `08` | Turing Core nodes: function, call, branch, loop, state | recursive factorial, map/filter/reduce | `AIL-CONTROL-001..003` | execution semantics fixtures |
| Self-hosting | `10`, `13`, `15` | `SelfHostCore v0`, AIL-Meta packages | Infer read permissions pass | AIL-Meta checker diagnostics | fixed-point package hash report |
| Portable binaries | `15`, `22` | backend conformance manifest | Linux ELF target, Wasm target contract | `AIL-BACKEND-001` | bytecode -> target artifact verifier |
| C interop | `09`, `21`, `26` | C binding schema, ABI layout, ownership rules | `strlen`, `compress2`, callback fixture | `AIL-FFI-OWNERSHIP-001`, `AIL-FFI-ERRNO-001` | foreign call trace and ABI verifier |
| Full-stack applications | `06`, `07`, `08`, `09`, `20`, `23` | package model, UI route/form schema, system levels | Support Ticket, Refund Tool, Network Driver | permission, effect, failure, UI diagnostics | profile-specific package fixtures |
| Round-trip semantic equivalence | `11`, `18`, `19`, `23` | normalization, semantic hash, projection-loss rules | Support Ticket core/spec/flow round trip | `AIL-ROUNDTRIP-001` | normalized graph comparison |
| Prompt portability | `05`, `12`, `19` | prompt pack, portability score | Support Ticket multi-model prompt task | prompt failure taxonomy | model-output corpus |
| Standard library and packages | `20` | package manifest, imports, capability grants | `Option<T>`, collections package | import/version/capability diagnostics | package conformance fixtures |
| Diagnostics as artifacts | `08`, `10`, `24` | diagnostic schema and catalog | action without trace | stable diagnostic IDs | rejected fixture per checker rule |
| Semantic safety | `02`, `06`, `21`, `23`, `26` | safety classes, approval schema, audit trace | high-risk refund and C pointer examples | safety diagnostics | safety review trace |
| Reference governance | `14`, `16`, `24`, `28` | rule identifiers, authority levels, version notes | rule-to-diagnostic entry | primary diagnostic per checker rule | conformance fixture per normative rule |

## Traceability Rules

Every language feature proposal must identify:

- desired outcome rows it affects
- AIL-Core schema nodes and edges
- canonical AIL-Spec form
- friendly rendering behavior
- AIL-Flow block or view behavior
- checker rules and diagnostic codes
- accepted fixture
- rejected fixture
- trace fixture
- prompt pack guidance
- conformance command

## Artifact Boundary Summary

```text
human English
  -> AIL-Requirements
  -> AIL-Spec Canonical
  -> AIL-Core schema v0
  -> checked package hash
  -> AIL-Flow / friendly projection / diagnostics / traces
  -> AIL-Bytecode
  -> backend conformance manifest
  -> native/Wasm artifact
  -> semantic explanation
```

Each arrow has a checker, renderer, conformance fixture, or verifier described
in the documents listed in the matrix.
