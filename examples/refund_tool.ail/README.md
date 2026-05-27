# Refund Tool Example

## Purpose

`refund_tool.ail` is the AgentTool teaching package for a payment refund flow
that must stay safe under LLM-authored specifications. It demonstrates how AIL
captures tool inputs, secret handling, permission rules, approval rules,
provider effects, failure handling, guarantees, and audit traces before the
tool is lowered into checked Core, bytecode, VM traces, and target evidence.

The example is intentionally high-level: the user story is not "call a refund
function", but "prove an agent can request a refund without bypassing payment
policy, leaking a token, or hiding provider failure from audit review."

## Concepts Taught

- AgentTool profile boundaries for a tool requested by an AI agent.
- Required inputs, including `Secret<Text>` payment credentials.
- Permission and approval rules that must be explicit instead of inferred from
  natural-language requirements.
- Provider effects such as `PaymentProvider.refund` and ledger writes.
- Failure semantics for provider rejection, including no successful ledger
  entry and no automatic customer notification.
- Audit trace requirements for both requested refunds and rejected provider
  calls.
- Rejected-output diagnostics for missing permissions, missing approvals,
  missing traces, secret leakage, hallucinated capabilities, and unknown input
  types.

## Files To Inspect

- `ail-package.md`: package metadata, profile, and feature declaration.
- `spec.ail-spec.md`: the canonical refund tool specification used by the
  live example corpus.
- `checked.ail-core.md`: checked Core snapshot for reviewing the lowered
  semantic graph.
- `examples/accepted/refund-minimal.ail-spec.md`: smaller accepted tool
  surface for conformance coverage.
- `examples/rejected/*.ail-spec.md`: focused invalid variants that teach one
  missing safety contract at a time.
- `../examples.md`: entries `example-40` through `example-54` exercise the
  same tool across prompt surfaces and target evidence.
- `../stories/example-40.md` through `../stories/example-54.md`: regenerated
  user-story views for the prompt matrix.

## Expected Replay Artifacts

The release corpus replays the Refund Tool entries through `ail-examples`.
Accepted entries should produce checked Core, bytecode, VM traces or target
reports, and user-story fingerprints under the chosen artifact directory, for
example:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-refund-examples --release-evidence
```

Useful artifacts to inspect after replay:

- `examples/example-40/checked.ail-core.txt`
- `examples/example-40/artifact.ailbc.json`
- `examples/example-40/target-report.txt`
- `examples/example-40/user-story.txt`
- `examples/example-103/diagnostics.txt`

The conformance fixture can also be checked directly:

```bash
cargo run -- ail-conformance examples/refund_tool.ail --artifact-dir /tmp/ail-refund-conformance
```

## Rejected Fixtures

The rejected fixtures are part of the teaching value, not incidental negative
tests:

- `approval-without-rule.ail-spec.md`: mentions approval behavior without an
  explicit approval rule.
- `permission-without-rule.ail-spec.md`: mentions requester permission without
  an explicit permission rule.
- `secret-output.ail-spec.md`: attempts to expose a secret as tool output.
- `tool-without-trace.ail-spec.md`: omits the required trace declaration.
- `unknown-input-type.ail-spec.md`: uses an unsupported secret payload type.

The corpus-level rejected entry `example-103` adds a hallucinated-capability
diagnostic path for the same domain.

## Next Example To Read

Read `../incident_response.ail/README.md` after this package. It is the natural
next step from a single AgentTool to a multi-module workflow with identity,
policy, notification, dashboard, and postmortem surfaces.

## v0.3 Learning Signal

The current Refund Tool corpus now emits deterministic agent policy review
artifacts for every accepted AgentTool entry. Those reviews bind multi-agent handoff
roles, `ail-agent-contracts examples/agents`, permission and approval review,
external-call review, secret-redaction review, audit-trace review, and runtime
evidence to the replay bundle. The next v0.3 bar is a human-approved
multi-agent policy handoff import workflow, plus denied refunds, provider retry
or backoff behavior, and richer repair tutorials for package-local rejected
fixtures.
