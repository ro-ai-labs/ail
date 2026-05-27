# Support Ticket Example

## Purpose

`support_ticket.ail` is the baseline high-level Application profile package for
AIL. It models a practical support workflow with customers, support agents,
support managers, ticket assignment, overdue tickets, public updates, secret
internal notes, scheduler-driven state changes, and permission-sensitive
views.

The package is the first place to inspect when reviewing whether AIL can turn a
business user story into checked specification structure, checked Core,
bytecode, VM traces, native binary evidence, and target-contract reports. It is
also the repeated family behind many prompt-surface examples, so its guide
anchors the difference between real workflow coverage and prompt matrix
coverage.

## Concepts Taught

- Application workflow modeling with users, roles, tickets, views, actions,
  failures, guarantees, traces, and secret fields.
- Required input checks for creating, assigning, closing, and marking tickets
  overdue.
- State transitions from `New` to `Assigned`, `Closed`, and `Overdue`.
- Scheduler behavior for overdue tickets through the
  `MarksOverdueTickets` action.
- Public update history that customers may see.
- secret internal notes that support staff may inspect but customers must not
  receive.
- Failure modeling for missing tickets and denied access.
- Native Linux binary evidence for each compiled action.
- User-story replay artifacts that present the same application workflow as a
  reviewer-facing story view.

## Files To Inspect

- `ail-package.md`: package metadata, Application profile, feature list, and
  target support.
- `spec.ail-spec.md`: canonical Support Ticket specification.
- `../support_composed.ail/spec.ail-spec.md`: package-import variant that
  composes the support workflow with shared package declarations.
- `../examples.md`: entries in the support-ticket families cover application
  prompt surfaces, targets, live LLM captures, rejected diagnostics, and repair
  paths.
- `../stories/example-30.md` through `../stories/example-34.md`: Application
  profile workflow stories tied to Linux target reports.
- `../stories/example-90.md` through `../stories/example-94.md`: System
  profile workflow stories tied to Darwin target-contract reports.
- `../stories/example-99.md`, `../stories/example-101.md`, and
  `../stories/example-102.md`: rejected or prompt-envelope story paths for
  semantic drift, profile mismatch, and missing trace coverage.

## Expected Replay Artifacts

Replay the full corpus to inspect Support Ticket entries:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-support-ticket-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-30/checked.ail-core.txt`
- `examples/example-30/artifact.ailbc.json`
- `examples/example-30/target-report.txt`
- `examples/example-30/user-story.txt`
- `examples/example-90/target-report.txt`
- `examples/example-99/diagnostics.txt`
- `examples/example-101/diagnostics.txt`
- `examples/example-102/diagnostics.txt`

For a focused package check:

```bash
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-support-ticket-conformance
```

For a direct native build of one action:

```bash
cargo run -- ail-build examples/support_ticket.ail --spec-file examples/support_ticket.ail/spec.ail-spec.md --artifact-dir /tmp/ail-support-ticket-build --target linux-x86_64-elf --action CloseTicket
```

For the story-first native runtime trace used by the interactive manual:

```bash
cargo test cli_ail_story_native_target_executes_story_runtime_trace --test ail_toolchain
```

That check starts from a support-ticket story, runs `ail-story` through checked
requirements, accepted spec, checked Core, bytecode, the toolchain agent, and a
Linux x86_64 native target, then executes the generated `CloseTicket` binary
with `ticket.id=T-1` and `ticket.status=Open`. The observed runtime evidence is
`ticket.status=Closed` and `trace TicketClosed`.

## Rejected Fixtures

The current package-level conformance path relies on corpus-level rejected
examples rather than local rejected specs. The important rejected paths are:

- `example-99`: semantic drift over the support-ticket family.
- `example-101`: prompt envelope profile mismatch before spec acceptance.
- `example-102`: missing trace coverage for `CloseTicket`.

v0.3 should add package-local rejected fixtures for customer access to secret
internal notes, scheduler mutation without overdue-time requirements, closing
an already closed ticket, assigning to a non-support user, and dropping
customer-visible public updates.

## Next Example To Read

Read `../support_composed.ail` after this package to see the same workflow move
through package imports. Then read `../refund_tool.ail/README.md` for an
AgentTool safety workflow and `../incident_response.ail/README.md` for the
larger multi-module application benchmark.

## v0.3 Learning Signal

Support Ticket is replay-clean and useful as the Application baseline. Its
accepted and diagnostic story files now carry semantic anchors for the core
ticket action, secret internal notes, prompt surfaces, target reports, and
diagnostic failure taxonomies. v0.3 now has deterministic manual evidence that
starts from a story and reaches native runtime trace output. The next bar is a
guided application tutorial with package-local rejected fixtures, explicit
story amendment examples, and a comparison between prompt-surface replay,
package-import replay, and native binary evidence.
