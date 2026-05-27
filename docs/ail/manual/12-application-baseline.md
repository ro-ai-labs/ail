# AIL Manual: Application Baseline

## Purpose

The Application Baseline chapter checks `examples/support_ticket.ail` and
`examples/incident_response.ail` as high-level workflow packages used by User
Story mode, prompt matrices, package composition, native target evidence, and
diagnostic repair examples.

This chapter is intentionally focused. It does not replay the whole corpus.
Instead, it proves the support-ticket and incident-response packages carry
package-local conformance fixtures that accept minimal workflows and reject
representative application mistakes with stable diagnostics.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter application-baseline --run-checks
```

The direct command is:

```sh
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-manual-application-baseline-conformance
cargo run -- ail-conformance examples/incident_response.ail --artifact-dir /tmp/ail-manual-incident-response-conformance
```

## What It Proves

- `examples/support_ticket.ail/spec.ail-spec.md` remains the accepted
  Application baseline.
- `examples/support_ticket.ail/examples/accepted/close-ticket-minimal.ail-spec.md`
  validates the minimal `CloseTicket` workflow.
- `examples/support_ticket.ail/examples/rejected/*.ail-spec.md` rejects local
  application failures instead of relying only on corpus-level diagnostics.
- `examples/incident_response.ail/examples/accepted/incident-escalation-minimal.ail-spec.md`
  validates escalation, notification audit, and lifecycle predecessor checks.
- `examples/incident_response.ail/examples/rejected/*.ail-spec.md` rejects
  notification and lifecycle mistakes directly in the high-level
  multi-module package.
- Rejected package-local fixtures write `repair-tutorial.txt`,
  `repair-proof.txt`, `repair-candidate.ail-spec.md`,
  `repair-checked.ail-core.txt`, and `repair-artifact.ailbc.json` plus
  fingerprints under the conformance artifact directory, so local diagnostics
  become checked repair chains instead of report-only failures.
- The report, manifest, repair tutorials, repair proofs, checked Core, and
  bytecode artifacts are fingerprinted in the same way as other conformance
  chapters.

## Expected Evidence

The chapter should surface:

```text
conformance-report.txt
manifest.ail-conformance.txt
accepted: close-ticket-minimal.ail-spec.md
accepted: incident-escalation-minimal.ail-spec.md
rejected: secret-leak.ail-spec.md AIL002
rejected: action-without-trace.ail-spec.md AIL-TRACE-001
rejected: failure-without-trace.ail-spec.md AIL-TRACE-002
rejected: unknown-field-type.ail-spec.md AIL-TYPE-001
rejected: assignment-without-role-requirement.ail-spec.md AIL-APP-001
rejected: overdue-without-time-requirement.ail-spec.md AIL-APP-002
rejected: status-change-without-public-update.ail-spec.md AIL-APP-003
rejected: notification-without-responder-pager.ail-spec.md AIL-APP-004
rejected: resolve-without-mitigating-status.ail-spec.md AIL-APP-005
rejected: postmortem-without-resolved-status.ail-spec.md AIL-APP-005
rejected: private-notes-public-timeline-leak.ail-spec.md AIL-APP-006
rejected: escalation-without-commander-review.ail-spec.md AIL-APP-007
rejected: route-missing-permission.ail-spec.md AIL-UI-PERMISSION-002
rejected: dashboard-missing-permission.ail-spec.md AIL-UI-PERMISSION-001
rejected-repair-tutorial-count 7
rejected/private-notes-public-timeline-leak.ail-spec.md/repair-tutorial.txt
rejected-repair-proof-count 7
rejected/private-notes-public-timeline-leak.ail-spec.md/repair-proof.txt
rejected/private-notes-public-timeline-leak.ail-spec.md/repair-candidate.ail-spec.md
rejected/private-notes-public-timeline-leak.ail-spec.md/repair-checked.ail-core.txt
rejected/private-notes-public-timeline-leak.ail-spec.md/repair-artifact.ailbc.json
ail conformance: ok
```

Additional package-local rejected fixtures cover missing references, missing
failure handlers, unknown fields, unknown requirement fields, secret reads
without protection, unhandled failure paths, assignee role requirements,
overdue scheduler time requirements, and public-update preservation for ticket
status changes.

Incident-response rejected fixtures cover responder notification without a
pager requirement, resolving before the incident is Mitigating, and starting
postmortem before the incident is Resolved. They also cover private-note
leakage into the public timeline, escalation without commander review, command
routes without read permission, and service-owner dashboards without read
permission. Each incident rejection now has a package-local conformance repair
tutorial and checked repair proof that preserve the diagnostic, source
provenance, graph item, repair suggestion, corrected fixture candidate, checked
Core, and verified bytecode before a repaired variant is considered for
promotion.

## Relationship To User Story Mode

User Story mode proves a support-ticket story can travel through requirements,
accepted spec, checked Core, bytecode, the AIL toolchain agent, a Linux x86_64
native executable, runtime trace output, and story-amendment comparison
evidence. This chapter is the package-local conformance companion: it proves
the same Application baseline also teaches accepted and rejected authoring
boundaries directly inside the package.
