# Compiler Pass AIL-Spec Example

Compiler pass: Infer read permissions.

The pass analyzes an AIL-Core graph and adds missing read permission
requirements for actions, tools, views, and compiler passes that read fields or
values.

The pass needs:

- input graph: AIL-Core graph
- package policy: permission inference policy

The pass produces:

- output graph: AIL-Core graph
- diagnostics: List<Diagnostic>

When the compiler runs Infer read permissions:

- the system reads every edge whose kind is reads
- the system reads package policy
- the system finds the actor, tool, view, or pass that performs the read
- the system checks whether an explicit Permission already allows the read
- if no permission exists, the system adds a candidate read Permission
- if the read target contains Secret, the system emits a diagnostic instead of silently adding permission
- the system guarantees it does not add write permissions
- the system guarantees every added permission has provenance from this pass
- the system records a trace event named ReadPermissionAddedScenario062

Failure SecretReadNeedsHumanConfirmation happens when a secret read has no explicit human-confirmed permission:

- the system leaves the graph unchanged for that read
- the system emits a diagnostic
- the trace records SecretReadInferenceBlockedScenario062
