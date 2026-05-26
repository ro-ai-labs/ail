# Support Composed Example

## Purpose

`support_composed.ail` is the package composition teaching example. It keeps
the support-ticket workflow small enough to inspect while proving that AIL can
split a system across packages, resolve explicit imports, check shared types,
lower the composed spec to Core, compile bytecode, and replay a VM trace.

The package imports `support_shared.ail` and uses `Shared.User` as the
customer type on `Ticket`. That makes it the first example to read when the
question is not "can AIL model an action?" but "can AIL model a system made of
multiple specs that still compiles and runs as one checked artifact?"

## Concepts Taught

- package composition with a local package graph instead of a single isolated
  spec file.
- explicit imports through `imports: ../support_shared.ail as Shared`.
- Referencing imported declarations with `Shared.User`.
- Two-module review: `support_composed` owns the ticket workflow while
  `support_shared` owns the reusable user shape.
- End-to-end prompt-surface replay across interview, requirements,
  spec-draft, core-draft, diagnostic-repair, core-to-spec, core-to-summary,
  flow-patch, trace-debug, and interop prompts.
- VM trace evidence for the composed `CloseTicket` action.
- Story journeys that move between `story-to-spec`, `spec-to-story`, and
  `story-amendment` over the same imported package graph.

## Files To Inspect

- `ail-package.md`: Application profile metadata, feature list, and the
  explicit import of `../support_shared.ail`.
- `spec.ail-spec.md`: the composed support workflow that uses `Shared.User`.
- `../support_shared.ail/spec.ail-spec.md`: the shared user declaration that
  proves the import is real rather than catalog-only metadata.
- `../examples.md`: entries `example-10` through `example-19` exercise the
  composed package across ten prompt surfaces.
- `../stories/example-10.md` through `../stories/example-19.md`: story views
  for package-import replay, story regeneration, and amendment paths.
- `../support_ticket.ail/README.md`: the larger standalone workflow that this
  composed package intentionally keeps smaller.

## Expected Replay Artifacts

Replay the corpus with release evidence enabled:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-support-composed-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-10/checked.ail-core.txt`
- `examples/example-10/artifact.ailbc.json`
- `examples/example-10/vm-trace.txt`
- `examples/example-14/user-story.txt`
- `examples/example-17/user-story.txt`
- `examples/example-19/vm-trace.txt`
- `/tmp/ail-support-composed-close-ticket.elf` from the direct native compile
  command below.

For a focused package check:

```bash
cargo run -- ail-conformance examples/support_composed.ail --artifact-dir /tmp/ail-support-composed-conformance
```

For the package-aware native compile path:

```bash
cargo run -- ail-compile examples/support_composed.ail --action CloseTicket --target linux-x86_64-elf --out /tmp/ail-support-composed-close-ticket.elf --artifact-dir /tmp/ail-support-composed-compile
```

## Rejected Fixtures

This package does not yet include package-local rejected fixtures. The current
release corpus uses it as an accepted package-import family. v0.3 should add
rejected fixtures for:

- omitting the `support_shared` import while still referencing `Shared.User`;
- importing the shared package under the wrong alias;
- changing `Ticket.customer` to an unresolved type;
- removing the `TicketClosed` trace from the composed action;
- claiming a dependency capability that is not granted by the package graph.

Those rejected fixtures should produce repair-oriented diagnostics so authors
learn how to fix package graphs, not only that the graph failed.

## Next Example To Read

Read `../support_ticket.ail/README.md` before this guide if you need the full
Application workflow first. After this package, read
`../incident_response.ail/README.md` for a larger multi-module application and
`../compiler_pass.ail` for the Compiler profile that transforms checked Core.

## v0.3 Learning Signal

Support Composed shows that AIL can replay imported package graphs, and its
story files now preserve semantic anchors for `support-composed`,
`support_shared`, `Shared.User`, `Close ticket`, `TicketClosed`, `active
queue`, and each prompt surface from `example-10` through `example-19`. v0.3
should add a dependency review view, package-local rejected fixtures, and a
story-diff that shows how a user request changes either the local spec or the
imported shared package.
