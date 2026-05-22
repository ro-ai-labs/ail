# Unknown Value Type Compiler Pass AIL-Spec Example

Compiler pass: Infer read permissions.

The pass analyzes read edges and proposes permission updates.

The pass needs:

- input graph: AIL-Core graph

The pass produces:

- diagnostics: List<MysteryDiagnostic>

When the compiler runs Infer read permissions:

- the system reads input graph
- the system emits diagnostics
- the system guarantees every diagnostic has provenance from this pass
- the system records a trace event named ReadPermissionAdded
