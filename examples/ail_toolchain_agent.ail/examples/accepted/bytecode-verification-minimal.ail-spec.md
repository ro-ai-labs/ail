# Bytecode Verification Minimal Accepted AIL-Spec Example

The application AIL Toolchain Agent Bytecode Verification manages accepted
agent artifact verification examples.

A BuildRequest has:

- id: Text
- status: State<BytecodeReady>
- bytecode artifact: Text
- bytecode fingerprint: Text
- bytecode verification report: Text

Action: Verify bytecode artifact.

When the toolchain agent verifies emitted bytecode:

- the system requires the BuildRequest to exist
- the system requires the BuildRequest status to be BytecodeReady
- the system reads the BuildRequest bytecode artifact
- the system reads the BuildRequest bytecode fingerprint
- the system changes the BuildRequest bytecode verification report to Verified
- the system guarantees the bytecode artifact is AIL-Bytecode with a deterministic fingerprint and not Rust or host-language backend source
- the system records a trace event named BytecodeArtifactVerified
