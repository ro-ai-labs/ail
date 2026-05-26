# AIL Standard Security Example

## Purpose

`ail_std_security.ail` teaches standard security concepts: secret values,
permissions, capabilities, and redaction. It defines `SecretEnvelope` and the
`Reveal secret` action.

This package is the standard-library bridge between high-level business
workflows and low-level safety rules. It shows that AIL can model
`Secret<Text>`, require explicit reveal permission and secret capability, read
secret payloads, and still guarantee that secret payload is redacted from
traces.

## Concepts Taught

- `SecretEnvelope` as a standard secret container.
- `Secret<Text>` fields and secret protection edges.
- Permission and capability requirements before reveal.
- Secret reads that do not leak values to traces.
- Redaction guarantees through `secret payload is redacted from traces`.
- Trace coverage through `SecretRevealed`.

## Files To Inspect

- `ail-package.md`: imports `../ail_std_core.ail compatible ^0.2 as Core`.
- `spec.ail-spec.md`: canonical security package specification.
- `examples/accepted/reveal-secret-minimal.ail-spec.md`: accepted minimal
  reveal fixture.
- `../refund_tool.ail/README.md`: AgentTool example that applies secret and
  approval concepts in a workflow.
- `../support_ticket.ail/README.md`: Application example with secret internal
  notes.

## Expected Replay Artifacts

Run focused conformance:

```bash
cargo run -- ail-conformance examples/ail_std_security.ail --artifact-dir /tmp/ail-std-security-conformance
```

Run the standard-library package artifact test:

```bash
cargo test cli_ail_stdlib_packages_have_checked_package_artifacts --test ail_toolchain
```

Useful artifacts include the conformance report and checked Core graph showing
secret fields, permission requirements, capability requirements, and redaction
guarantees.

## Rejected Fixtures

This package currently has no local rejected fixture. v0.3 should add rejected
fixtures for revealing `Secret<Text>` without permission, missing capability
requirements, leaking secret payloads into traces, and treating redaction as a
comment instead of a checked guarantee.

## Next Example To Read

Read `../refund_tool.ail/README.md` next to see security concepts in an
AgentTool workflow with approvals and policy review.

## v0.3 Learning Signal

AIL Standard Security needs stronger negative examples. v0.3 should make
secret leakage, missing permission, missing capability, and incorrect
redaction repair paths first-class standard-library tutorials.
