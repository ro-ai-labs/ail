# Minimal Compiler Pass AIL-Spec Example

Compiler pass: Infer read permissions.

The pass analyzes read edges and proposes permission updates.

The pass needs:

- input graph: AIL-Core graph

The pass produces:

- output graph: AIL-Core graph
- diagnostics: List<Diagnostic>

When the compiler runs Infer read permissions:

- the system reads input graph
- the system finds reads without permissions
- the system adds a candidate read Permission
- the system guarantees every added permission has provenance from this pass
- the system records a trace event named ReadPermissionAdded
