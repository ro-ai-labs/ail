# AIL Agent Tools

## Purpose

AIL agent tools are explicit, auditable capabilities that an AI Agent may
request. A tool is accepted only when its purpose, inputs, outputs,
permissions, effects, secrets, approvals, failures, guarantees, and audit trace
are declared.

## Tool Contract

A tool contract includes:

- purpose
- allowed use
- inputs
- outputs
- preconditions
- permissions
- effects
- external calls
- secrets
- human approval
- failures
- guarantees
- audit trace

## Inputs And Outputs

Inputs and outputs use AIL types. Inputs may include `Secret<T>` values. Outputs
must not disclose secret values unless the declared audience has permission.
The checker rejects a `Secret<T>` output unless a reviewed permission explicitly
allows that secret to be revealed or disclosed.

## Permissions And Capabilities

Permissions describe what the tool may read, write, call, approve, or reveal.
Capabilities are runtime-enforced grants derived from those permissions.

Structured AIL-Spec declares tool permission gates in a `The tool requires
permission:` section. If tool behavior mentions permission, the checker
requires at least one explicit permission rule so the runtime can derive a
capability from a reviewed contract rather than from loose prose.

## Effects

Effects name every state change, external call, message, file write, payment
action, network request, device action, or compiler mutation the tool may
perform.

External API bindings must declare target endpoint, input and output schema,
permission, capability, failure mapping, retry behavior, timeout, redaction,
and trace event. A natural-language mention of an API is not enough.

## Secrets

Secrets are explicit. A tool that receives a payment token must declare it as a
secret and must guarantee that the payment token never appears in a response,
trace summary, log, or agent-visible explanation unless a redacted form is
specified.

Secret-redaction fixtures must show:

- accepted redacted output
- rejected raw secret output
- trace summary with redacted value
- diagnostic code when a secret reaches an agent-visible field

## Human Approval

Approval rules name when a human must authorize execution. Approval decisions
are trace events and are part of the accepted behavior.

Structured AIL-Spec declares tool approval gates in a `The tool requires
approval:` section. If tool behavior mentions approval or creates approval
work, the checker requires at least one explicit approval rule so the runtime
can enforce the gate before external effects occur.

## Audit Trace

The runtime records who requested the tool, which rules allowed it, which
approval was used, which data was read or written, which external systems were
called, what failed, and which guarantees were checked.

Structured AIL-Spec declares the main tool audit events in a `The tool
records:` section. A tool without at least one declared audit trace event is
not accepted because the agent runtime would have no stable event to anchor
requests, approvals, external calls, and guarantee checks.

## Runtime Enforcement

The LLM can request a tool, but the runtime enforces the declared capability.
If the request exceeds the contract, the runtime rejects it before the external
effect occurs.

## Authorization Flow

Runtime tool calls follow this order:

1. Decode request into declared input types.
2. Check requester permission.
3. Check capability grant.
4. Check approval conditions.
5. Check secret redaction rules.
6. Check sandbox or external binding policy.
7. Execute the call.
8. Map failures into declared AIL failures.
9. Record audit trace.
10. Check guarantees.

## Sandboxing Rules

Tool execution has no ambient authority. File, network, process, clock, random,
payment, C interop, and device effects require explicit capabilities. A tool
that requests an undeclared effect is rejected before the effect occurs.

## External API Binding Rules

An external API binding declares:

- service name
- operation name
- endpoint or symbolic binding
- input schema
- output schema
- authentication secret handling
- permission and capability
- effect class
- rate limit or retry behavior
- failure mapping
- trace event

The binding is represented in AIL-Core as an `ExternalBinding` and checked like
any other effectful call.

## Example: Refund Customer Payment

A Refund tool may read an order, read a payment record, call a payment provider,
write a refund ledger entry, and notify a human reviewer when approval is
required. It must protect the payment token and guarantee that the refund amount
does not exceed the captured amount.

The related examples are:

- `../../examples/refund_tool.ail/spec.ail-spec.md`
- `../../examples/refund_tool.ail/checked.ail-core.md`
