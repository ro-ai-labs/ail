# AIL Toolchain Agent AIL-Spec Example

The application AIL Toolchain Agent manages developer interviews, requirements
capture, checked AIL specs, AIL-Core IR lowering, and AIL-Bytecode compilation.

A BuildRequest has:

- id: Text
- developer prompt: Text
- requirements: Text
- requirements coverage checklist: Text
- spec coverage checklist: Text
- spec: Text
- spec review report: Text
- core ir: Text
- core review report: Text
- bytecode artifact: Text
- bytecode verification report: Text
- prompt portability report: Text
- status: State<PromptReceived, RequirementsCaptured, SpecCaptured, CoreChecked, BytecodeReady, NeedsClarification>
- target model: Text

The application shows:

- a developer interview queue
- a requirements coverage view
- a bytecode artifact review view

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
- the system requires the BuildRequest status to be RequirementsCaptured
- the system reads the BuildRequest requirements
- the system changes the BuildRequest spec coverage checklist to Prepared
- the system guarantees the spec prompt preserves requirements, domain model, actions, failures, guarantees, traces, secrets, runtime inputs, and bytecode compilation path
- the system records a trace event named SpecDraftPrepared

Action: Accept spec draft.

When the toolchain agent accepts a checked AIL spec draft:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be RequirementsCaptured
- the system reads the BuildRequest requirements
- the system reads the BuildRequest spec
- the system changes the BuildRequest spec review report to Accepted
- the system changes the BuildRequest status to SpecCaptured
- the system guarantees the accepted spec preserves the checked requirements and remains eligible for AIL-Core lowering and AIL-Bytecode compilation
- the system records a trace event named SpecDraftAccepted

Action: Accept core IR.

When the toolchain agent accepts checked AIL-Core IR:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be SpecCaptured
- the system reads the BuildRequest requirements
- the system reads the BuildRequest spec
- the system reads the BuildRequest core ir
- the system changes the BuildRequest core review report to Accepted
- the system changes the BuildRequest status to CoreChecked
- the system guarantees the checked AIL-Core IR preserves the accepted spec and remains eligible for AIL-Bytecode compilation
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
- the system guarantees the compiler emits AIL-Bytecode and no Rust or host-language backend source
- the system records a trace event named ApplicationBytecodeCompiled

Action: Verify bytecode artifact.

When the toolchain agent verifies emitted bytecode:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest bytecode artifact
- the system changes the BuildRequest bytecode verification report to Verified
- the system guarantees the bytecode artifact is AIL-Bytecode and not Rust or host-language backend source
- the system records a trace event named BytecodeArtifactVerified

Action: Compare agent prompt portability.

When the toolchain agent evaluates a target model:

- the system requires the BuildRequest to exist
- the system reads the BuildRequest target model
- the system reads the BuildRequest requirements
- the system changes the BuildRequest prompt portability report to Compared
- the system guarantees the agent prompt preserves developer-interview role, requirements coverage, IR conversion, and bytecode compilation duties across model ports
- the system records a trace event named AgentPromptPortabilityCompared
