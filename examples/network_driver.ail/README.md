# Network Driver Example

## Purpose

`network_driver.ail` is the low-level System profile teaching package for
device-facing code. It shows how AIL can model ownership, borrowing, allocation
placement, device effects, capability grants, and trace guarantees before any
native or target-contract artifact is trusted.

This package is intentionally small. Its value is that the same concepts used
by application workflows also apply to a packet receiver that owns a buffer,
borrows packet metadata, performs a device read, writes into the receive
buffer, releases it, and records packet processing evidence.

## Concepts Taught

- System profile packages with explicit resources, capabilities, effects, and
  guarantees.
- Ownership and borrowing for `rx buffer` and `packet metadata`.
- Region placement for packet processing memory.
- Device effects that require declared capability grants.
- Trace evidence for packet receive behavior.
- Prompt-surface coverage across summary, flow patch, trace debug, interop,
  interview, requirements, spec draft, Core draft, and diagnostic repair.
- Rejected permission/capability behavior when a device effect is used without
  the matching capability.

## Files To Inspect

- `ail-package.md`: System profile metadata, feature list, and conformance
  boundary.
- `spec.ail-spec.md`: canonical packet receiver specification.
- `checked.ail-core.md`: checked Core projection for the package.
- `../examples.md`: entries `example-66` through `example-74` cover accepted
  network-driver prompt surfaces; `example-106` covers the rejected
  effect-without-capability diagnostic.
- `../stories/example-66.md` through `../stories/example-74.md`: regenerated
  story views for the accepted low-level prompt surfaces.
- `../stories/example-106.md`: diagnostic story view for the missing capability
  case.

## Expected Replay Artifacts

Replay the corpus to inspect network-driver artifacts:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-network-driver-examples --release-evidence
```

Useful artifacts to inspect after replay:

- `examples/example-66/checked.ail-core.txt`
- `examples/example-66/artifact.ailbc.json`
- `examples/example-66/vm-trace.txt`
- `examples/example-69/target-report.txt`
- `examples/example-74/diagnostics.txt`
- `examples/example-106/diagnostics.txt`

The direct conformance check is:

```bash
cargo run -- ail-conformance examples/network_driver.ail --artifact-dir /tmp/ail-network-driver-conformance
```

## Rejected Fixtures

`example-106` is the current rejected fixture for this package. It verifies that
the System profile rejects a network device effect when the spec does not
declare the matching capability.

High-value v0.3 rejected fixtures should add:

- borrowed packet metadata escaping the packet processing region;
- double release of `rx buffer`;
- writing to the receive buffer after release;
- interrupt-context effects that require a stronger capability;
- scheduler priority or interrupt-mask combinations that violate the package
  guarantees.

## Next Example To Read

Read `../c_interop.ail/README.md` after this package. It moves from
device-facing resources and effects to ABI-safe host calls, pointers, callbacks,
layout, and status-map diagnostics.

## v0.3 Learning Signal

Network Driver is the current low-level System profile anchor, but it is still
too narrow to prove realistic driver development. v0.3 should expand it into a
small driver family with receive, transmit, interrupt handler, scheduler task,
and rejected lifetime fixtures so AIL can prove low-level programs as strongly
as it proves high-level workflows.
