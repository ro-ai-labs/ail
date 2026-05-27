# AIL Toolchain Agent Example

## Purpose

This package is the AIL-authored agent used by the development toolchain. It
models the handoff from developer intent through requirements, accepted spec,
checked Core, bytecode, target artifacts, prompt portability reports, and
manifest verification.

## Concepts Taught

- AI Agent participation without making the agent part of the trusted compiler
  core.
- `BuildRequest` state transitions across requirements, spec, Core, flow,
  compiler-pass, bytecode, native target, and manifest review.
- Agent-authored trace events such as `RequirementsCaptured`,
  `ApplicationBytecodeCompiled`, `TargetArtifactVerified`, and
  `BuildManifestVerified`.

## Files To Inspect

- `ail-package.md`: package identity and Application profile metadata.
- `spec.ail-spec.md`: the BuildRequest model and agent action sequence.
- `../agents/README.md`: Codex-style executor contracts used for stored
  transcript imports.
- `../support_ticket.ail/README.md`: user-story and build flows that run this
  package as an agent.

## Expected Replay Artifacts

Toolchain tests compile this package into `agent.ailbc.json` and use it to
write `agent-trace.txt` entries for `ail-build`, `ail-story`, compiler-pass,
target verification, and manifest verification flows.

## Rejected Fixtures

This package includes package-local conformance fixtures:

- `examples/accepted/bytecode-verification-minimal.ail-spec.md` shows the
  minimal accepted bytecode-verification action.
- `examples/rejected/bytecode-verification-without-fingerprint.ail-spec.md`
  rejects an agent action that verifies a bytecode artifact without reading
  the matching fingerprint first.

Additional rejection coverage comes from the build, compile, prompt-envelope,
and manifest-verification tests that stop before trusting malformed or
incomplete agent handoffs.

## Next Example To Read

Read `compiler_pass.ail/README.md` next for the compiler-pass side of the same
self-hosting path, then `support_ticket.ail/README.md` for story-mode authoring
through this agent.

## v0.3 Learning Signal

AIL v0.3 should turn this package from a deterministic verifier participant
into a richer multi-agent handoff tutorial with first-class agent semantics,
default entrypoint policy, prompt-portable handoff contracts, and repair
guidance for failed agent actions. The current local fixtures establish the
first agent-specific artifact-verification boundary.
