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

## Secrets

Secrets are explicit. A tool that receives a payment token must declare it as a
secret and must guarantee that the payment token never appears in a response,
trace summary, log, or agent-visible explanation unless a redacted form is
specified.

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

## Example: Refund Customer Payment

A Refund tool may read an order, read a payment record, call a payment provider,
write a refund ledger entry, and notify a human reviewer when approval is
required. It must protect the payment token and guarantee that the refund amount
does not exceed the captured amount.

The related examples are:

- `examples/refund-tool.ail-spec.md`
- `examples/refund-tool.ail-core.md`
