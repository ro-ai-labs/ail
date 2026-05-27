# AIL Manual: Application Baseline

## Purpose

The Application Baseline chapter checks `examples/support_ticket.ail` as the
high-level workflow package used by User Story mode, prompt matrices, package
composition, native target evidence, and diagnostic repair examples.

This chapter is intentionally focused. It does not replay the whole corpus.
Instead, it proves the support-ticket package carries package-local conformance
fixtures that accept the minimal closing workflow and reject representative
application mistakes with stable diagnostics.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter application-baseline --run-checks
```

The direct command is:

```sh
cargo run -- ail-conformance examples/support_ticket.ail --artifact-dir /tmp/ail-manual-application-baseline-conformance
```

## What It Proves

- `examples/support_ticket.ail/spec.ail-spec.md` remains the accepted
  Application baseline.
- `examples/support_ticket.ail/examples/accepted/close-ticket-minimal.ail-spec.md`
  validates the minimal `CloseTicket` workflow.
- `examples/support_ticket.ail/examples/rejected/*.ail-spec.md` rejects local
  application failures instead of relying only on corpus-level diagnostics.
- The report and manifest are fingerprinted in the same way as other
  conformance chapters.

## Expected Evidence

The chapter should surface:

```text
conformance-report.txt
manifest.ail-conformance.txt
accepted: close-ticket-minimal.ail-spec.md
rejected: secret-leak.ail-spec.md AIL002
rejected: action-without-trace.ail-spec.md AIL-TRACE-001
rejected: failure-without-trace.ail-spec.md AIL-TRACE-002
rejected: unknown-field-type.ail-spec.md AIL-TYPE-001
ail conformance: ok
```

Additional package-local rejected fixtures cover missing references, missing
failure handlers, unknown fields, unknown requirement fields, secret reads
without protection, and unhandled failure paths.

## Relationship To User Story Mode

User Story mode proves a support-ticket story can travel through requirements,
accepted spec, checked Core, bytecode, the AIL toolchain agent, a Linux x86_64
native executable, runtime trace output, and story-amendment comparison
evidence. This chapter is the package-local conformance companion: it proves
the same Application baseline also teaches accepted and rejected authoring
boundaries directly inside the package.
