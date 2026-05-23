# AIL Toolchain Agent AIL-Spec Example

The application AIL Toolchain Agent manages developer interviews, requirements
capture, checked AIL specs, AIL-Core IR lowering, and VM or native target
artifact compilation.

A BuildRequest has:

- id: Text
- developer prompt: Text
- requirements: Text
- requirements fingerprint: Text
- requirements coverage checklist: Text
- spec coverage checklist: Text
- source package: Text
- source package fingerprint: Text
- spec: Text
- spec fingerprint: Text
- spec review report: Text
- core ir: Text
- core ir fingerprint: Text
- core review report: Text
- compiler pass artifact: Text
- compiler pass fingerprint: Text
- compiler pass target artifact fingerprint: Text
- compiler pass trace: Text
- compiler pass review report: Text
- conformance report: Text
- conformance report fingerprint: Text
- fixed point report: Text
- fixed point report fingerprint: Text
- native bytecode report: Text
- native bytecode report fingerprint: Text
- bytecode artifact: Text
- bytecode fingerprint: Text
- bytecode verification report: Text
- target platform: Text
- target artifact: Text
- target artifact fingerprint: Text
- target artifact compilation report: Text
- target artifact verification report: Text
- artifact manifest: Text
- artifact manifest fingerprint: Text
- artifact manifest verification report: Text
- prompt portability report: Text
- prompt portability report fingerprint: Text
- status: State<PromptReceived, RequirementsLoaded, RequirementsCaptured, SpecLoaded, SpecCaptured, CoreLoaded, PassApplied, CoreChecked, BytecodeReady, NeedsClarification>
- target model: Text

The application shows:

- a developer interview queue
- a requirements coverage view
- a target artifact review view

Action: Capture requirements.

When the toolchain agent interviews an application developer:

- the system requires the BuildRequest to exist
- the system reads the BuildRequest developer prompt
- the system changes the BuildRequest requirements coverage checklist to Prepared
- the system changes the BuildRequest status to RequirementsCaptured
- the system guarantees requirements mention domain objects, actions, failures, guarantees, traces, secrets, and runtime inputs before compilation
- the system records a trace event named RequirementsCaptured

Action: Prepare spec draft.

When the toolchain agent prepares a checked AIL spec prompt:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be RequirementsCaptured or RequirementsLoaded
- the system reads the BuildRequest requirements
- the system changes the BuildRequest spec coverage checklist to Prepared
- the system guarantees the spec prompt preserves captured or loaded requirements, domain model, actions, failures, guarantees, traces, secrets, runtime inputs, and bytecode compilation path
- the system records a trace event named SpecDraftPrepared

Action: Accept spec draft.

When the toolchain agent accepts a checked AIL spec draft:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be RequirementsCaptured or RequirementsLoaded or SpecLoaded
- the system reads the BuildRequest requirements
- the system reads the BuildRequest spec
- the system changes the BuildRequest spec review report to Accepted
- the system changes the BuildRequest status to SpecCaptured
- the system guarantees the accepted spec preserves the checked requirements or saved spec artifact boundary and remains eligible for AIL-Core lowering and VM or native target compilation
- the system records a trace event named SpecDraftAccepted

Action: Accept compiler pass output.

When the toolchain agent accepts an AIL compiler pass output:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be SpecCaptured or CoreLoaded
- the system reads the BuildRequest requirements
- the system reads the BuildRequest spec
- the system reads the BuildRequest core ir
- the system reads the BuildRequest compiler pass artifact
- the system reads the BuildRequest compiler pass fingerprint
- the system reads the BuildRequest compiler pass trace
- the system changes the BuildRequest compiler pass review report to Accepted
- the system changes the BuildRequest status to PassApplied
- the system guarantees the AIL compiler pass bytecode transformed checked AIL-Core without host-language backend source
- the system records a trace event named CompilerPassOutputAccepted

Action: Accept core IR.

When the toolchain agent accepts checked AIL-Core IR:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be SpecCaptured or CoreLoaded or PassApplied
- the system reads the BuildRequest requirements
- the system reads the BuildRequest spec
- the system reads the BuildRequest core ir
- the system changes the BuildRequest core review report to Accepted
- the system changes the BuildRequest status to CoreChecked
- the system guarantees the checked AIL-Core IR preserves the accepted spec or saved core artifact boundary and remains eligible for VM or native target compilation
- the system records a trace event named CoreIrAccepted

Action: Compile application.

When the toolchain agent compiles a captured application:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be SpecCaptured or CoreChecked
- the system reads the BuildRequest requirements
- the system reads the BuildRequest spec
- the system changes the BuildRequest core ir to Checked
- the system changes the BuildRequest bytecode artifact to Emitted
- the system changes the BuildRequest status to BytecodeReady
- the system guarantees the compiler emits a verified toolchain artifact and no Rust or host-language backend source
- the system records a trace event named ApplicationBytecodeCompiled

Action: Compile native target.

When the toolchain agent requests executable bytes for a native target:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest bytecode artifact
- the system reads the BuildRequest bytecode fingerprint
- the system reads the BuildRequest target platform
- the system reads the BuildRequest target artifact
- the system reads the BuildRequest target artifact fingerprint
- the system changes the BuildRequest target artifact compilation report to Emitted
- the system guarantees the native target call produces machine-level executable bytes for the selected target, Linux x86_64 ELF bytes for linux-x86_64-elf, and no Rust or host-language backend source
- the system records a trace event named NativeTargetCompiled

Action: Verify bytecode artifact.

When the toolchain agent verifies emitted bytecode:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest bytecode artifact
- the system reads the BuildRequest bytecode fingerprint
- the system changes the BuildRequest bytecode verification report to Verified
- the system guarantees the bytecode artifact is AIL-Bytecode with a deterministic fingerprint and not Rust or host-language backend source
- the system records a trace event named BytecodeArtifactVerified

Action: Verify lower manifest.

When the toolchain agent verifies AIL-Core lowering artifacts:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest core ir
- the system reads the BuildRequest core ir fingerprint
- the system reads the BuildRequest source package
- the system reads the BuildRequest source package fingerprint
- the system reads the BuildRequest bytecode artifact
- the system reads the BuildRequest bytecode fingerprint
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the lower manifest ties checked AIL-Core and AIL-Bytecode with deterministic fingerprints and no Rust or host-language backend source
- the system records a trace event named LowerManifestVerified

Action: Verify target artifact.

When the toolchain agent verifies the emitted target artifact:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest target artifact
- the system reads the BuildRequest target artifact fingerprint
- the system changes the BuildRequest target artifact verification report to Verified
- the system guarantees the target artifact is a selected VM or native executable artifact with a deterministic fingerprint and not Rust or host-language backend source
- the system records a trace event named TargetArtifactVerified

Action: Verify build manifest.

When the toolchain agent verifies the build artifact manifest:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system reads the BuildRequest source package
- the system reads the BuildRequest source package fingerprint
- the system reads the BuildRequest requirements fingerprint
- the system reads the BuildRequest spec fingerprint
- the system reads the BuildRequest core ir fingerprint
- the system reads the BuildRequest bytecode fingerprint
- the system reads the BuildRequest target artifact fingerprint
- the system reads the BuildRequest compiler pass target artifact fingerprint
- the system reads the BuildRequest prompt portability report fingerprint
- the system reads the BuildRequest native bytecode report
- the system reads the BuildRequest native bytecode report fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the build manifest ties requirements, spec, AIL-Core, compiler-pass, agent, prompt-portability, bytecode, native bytecode report, and native target artifacts with deterministic fingerprints and no Rust or host-language backend source
- the system records a trace event named BuildManifestVerified

Action: Verify compile manifest.

When the toolchain agent verifies direct compile artifacts:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system reads the BuildRequest bytecode fingerprint
- the system reads the BuildRequest target artifact
- the system reads the BuildRequest target artifact fingerprint
- the system reads the BuildRequest native bytecode report
- the system reads the BuildRequest native bytecode report fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the compile manifest ties verified AIL-Bytecode and the native target artifact with deterministic fingerprints and no Rust or host-language backend source
- the system records a trace event named CompileManifestVerified

Action: Verify compile bundle manifest.

When the toolchain agent verifies direct all-action compile artifacts:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system reads the BuildRequest bytecode fingerprint
- the system reads the BuildRequest target artifact
- the system reads the BuildRequest target artifact fingerprint
- the system reads the BuildRequest native bytecode report
- the system reads the BuildRequest native bytecode report fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the compile bundle manifest ties verified AIL-Bytecode and all native action target artifacts with deterministic fingerprints and no Rust or host-language backend source
- the system records a trace event named CompileBundleManifestVerified

Action: Verify bootstrap manifest.

When the toolchain agent verifies an AIL bootstrap bundle:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system reads the BuildRequest source package
- the system reads the BuildRequest source package fingerprint
- the system reads the BuildRequest core ir
- the system reads the BuildRequest core ir fingerprint
- the system reads the BuildRequest bytecode fingerprint
- the system reads the BuildRequest compiler pass fingerprint
- the system reads the BuildRequest compiler pass trace
- the system reads the BuildRequest fixed point report
- the system reads the BuildRequest fixed point report fingerprint
- the system reads the BuildRequest conformance report
- the system reads the BuildRequest conformance report fingerprint
- the system reads the BuildRequest native bytecode report
- the system reads the BuildRequest native bytecode report fingerprint
- the system reads the BuildRequest target artifact fingerprint
- the system reads the BuildRequest compiler pass target artifact fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the bootstrap manifest ties AIL source packages, checked AIL-Core IR, AIL-Bytecode, conformance reports, AIL-authored toolchain agent, AIL-Meta compiler pass, and Linux native executable bytes with deterministic fingerprints and no Rust or host-language backend source
- the system records a trace event named BootstrapManifestVerified

Action: Verify pass manifest.

When the toolchain agent verifies the standalone compiler pass manifest:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be PassApplied
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system reads the BuildRequest compiler pass fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the pass manifest ties compiler-pass bytecode, transformed AIL-Core, pass trace, and agent artifacts with deterministic fingerprints and no Rust or host-language backend source
- the system records a trace event named PassManifestVerified

Action: Verify conformance manifest.

When the toolchain agent verifies conformance artifacts:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest conformance report
- the system reads the BuildRequest conformance report fingerprint
- the system reads the BuildRequest artifact manifest
- the system reads the BuildRequest artifact manifest fingerprint
- the system changes the BuildRequest artifact manifest verification report to Verified
- the system guarantees the conformance manifest ties the conformance report, accepted fixtures, rejected fixtures, and fingerprints with no Rust or host-language backend source
- the system records a trace event named ConformanceManifestVerified

Action: Compare agent prompt portability.

When the toolchain agent evaluates a target model:

- the system requires the BuildRequest to exist
- the system reads the BuildRequest target model
- the system reads the BuildRequest requirements
- the system changes the BuildRequest prompt portability report to Compared
- the system guarantees the agent prompt preserves developer-interview role, requirements coverage, IR conversion, and bytecode compilation duties across model ports
- the system records a trace event named AgentPromptPortabilityCompared
