# AIL Toolchain Agent AIL-Spec Example

The application AIL Toolchain Agent manages developer interviews, requirements
capture, checked AIL specs, AIL-Core IR lowering, and AIL-Bytecode compilation.

A BuildRequest has:

- id: Text
- developer prompt: Text
- requirements: Text
- spec: Text
- core ir: Text
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
- the system changes the BuildRequest status to RequirementsCaptured
- the system guarantees requirements mention domain objects, actions, failures, guarantees, traces, secrets, and runtime inputs before compilation
- the system records a trace event named RequirementsCaptured

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
