# Compiler Pass AIL-Core Example

```text
package ail-meta-permissions
profile Compiler

node Action InferReadPermissions
  kind: CompilerPass
  Provenance: spec:compiler-pass

node Value PassInputGraph type AIL-Core
node Value PackagePolicy type PermissionInferencePolicy
node Value PassOutputGraph type AIL-Core
node Value Diagnostics type List<Diagnostic>

node Rule ReadEdgeRequiresPermission
node Rule SecretReadNeedsHumanConfirmation
node Diagnostic MissingReadPermission
node Diagnostic SecretReadInferenceBlocked

edge InferReadPermissions reads PassInputGraph
edge InferReadPermissions reads PackagePolicy
edge InferReadPermissions writes PassOutputGraph
edge InferReadPermissions writes Diagnostics
edge InferReadPermissions applies Rule.ReadEdgeRequiresPermission
edge InferReadPermissions may-fail-with Failure.SecretReadNeedsHumanConfirmation

node Step ScanReads
node Step MatchExistingPermission
node Step AddCandidatePermission
node Step EmitSecretReadDiagnostic

edge InferReadPermissions contains Step.ScanReads
edge InferReadPermissions contains Step.MatchExistingPermission
edge InferReadPermissions contains Step.AddCandidatePermission
edge InferReadPermissions contains Step.EmitSecretReadDiagnostic

node Guarantee NoWritePermissionsAdded
node Guarantee AddedPermissionsHavePassProvenance
edge InferReadPermissions guarantees Guarantee.NoWritePermissionsAdded
edge InferReadPermissions guarantees Guarantee.AddedPermissionsHavePassProvenance

node Trace SecretReadInferenceBlocked
node Trace ReadPermissionAdded
```
