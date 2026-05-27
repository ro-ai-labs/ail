# Compiler Pass Example

## Purpose

`compiler_pass.ail` is the low-level Compiler profile example for AIL. It
models `InferReadPermissions`, a pass that reads an AIL-Core graph, finds
field or value reads, and adds candidate read permissions when the graph does
not already contain explicit permission edges.

This example matters for v0.3 because it moves AIL beyond application
workflows. It shows that AIL can describe compiler behavior, lower that
description to checked Core, compile it to executable pass bytecode, run the
pass over another checked package, and emit auditable transform traces and
native target reports.

## Concepts Taught

- Compiler profile packages and compiler-pass declarations.
- AIL-Core graph inputs and transformed AIL-Core graph outputs.
- Permission inference over `reads` edges.
- Provenance requirements for compiler-generated permissions.
- Secret-read safety through the `SecretReadNeedsHumanConfirmation` failure.
- Diagnostics instead of silent permission inference when a read target
  contains `Secret`.
- Native Linux target evidence for the pass action `InferReadPermissions`.
- Self-hosting direction: AIL-authored passes now need multiple composed pass
  variants and reviewer-visible pass-order conflict diagnostics before v0.3
  can raise the bar again.

## Files To Inspect

- `ail-package.md`: Compiler profile metadata and compiler-pass feature list.
- `spec.ail-spec.md`: canonical spec for `InferReadPermissions`.
- `reference.ail-spec.md`: reference text used by conformance fixtures.
- `checked.ail-core.md`: checked Core form for reviewing the pass graph.
- `examples/accepted/infer-read-permissions-minimal.ail-spec.md`: accepted
  local fixture for the minimal pass behavior.
- `examples/rejected/unknown-value-type.ail-spec.md`: rejected local fixture
  proving diagnostics fire before the pass is accepted.
- `../examples.md`: entries `example-55` through `example-64` replay this pass
  across Core, story, repair, trace, interop, interview, requirements,
  spec-draft, and diagnostic prompt surfaces.
- `../stories/example-55.md` through `../stories/example-64.md`: story views
  for the compiler pass family.

## Expected Replay Artifacts

Replay the corpus to inspect compiler-pass artifacts:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-compiler-pass-examples --release-evidence
```

Useful artifacts after replay include:

- `examples/example-55/checked.ail-core.txt`
- `examples/example-55/artifact.ailbc.json`
- `examples/example-55/target-report.txt`
- `examples/example-57/user-story.txt`
- `examples/example-58/target-report.txt`
- `examples/example-64/target-report.txt`

For focused conformance:

```bash
cargo run -- ail-conformance examples/compiler_pass.ail --artifact-dir /tmp/ail-compiler-pass-conformance
```

For a direct pass run over the support workflow:

```bash
cargo run -- ail-pass examples/compiler_pass.ail examples/support_ticket.ail --action InferReadPermissions --target linux-x86_64-elf --artifact-dir /tmp/ail-compiler-pass-run
```

## Rejected Fixtures

This package already includes one package-local rejected fixture:

- `examples/rejected/unknown-value-type.ail-spec.md`: rejects a compiler-pass
  spec that refers to an unknown value type.

v0.3 should add rejected fixtures for secret reads without human-confirmed
permission, write-permission inference, permissions without provenance,
diagnostics that mutate the graph anyway, and pass output that is not a valid
checked AIL-Core graph.

## Next Example To Read

Read `../support_composed.ail/README.md` before this guide if you want the
package graph being transformed. After this package, read
`../ail_toolchain_agent.ail` for the toolchain-agent verifier and
`../incident_response.ail/README.md` for a larger application that will need
compiler-pass review as the language grows.

## v0.3 Learning Signal

Compiler Pass is the current bridge from language use cases to language
implementation. It is replay-clean, and bootstrap now records a pass
composition, fixed-point report, compiler-pass self-check variant, and
reviewer-visible `AIL-BOOTSTRAP-PASS-ORDER-001` pass-order diagnostic for
`InferReadPermissions`. The bootstrap CLI now preserves ordered user-supplied
`--pass` sequences and rejects a duplicate pass before the fixed-point gate as a
fingerprinted conflicting-order fixture. The next v0.3 bar is richer accepted
multi-pass compiler families, secret-read fixtures, and a walkthrough that
compares the input Core graph, transformed graph, provenance edges, bytecode
trace, and native target report side by side.
