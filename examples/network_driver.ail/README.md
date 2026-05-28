# Network Driver Example

## Purpose

`network_driver.ail` is the low-level System profile teaching package for
device-facing code. It shows how AIL can model ownership, borrowing, allocation
placement, scheduler tasks, interrupt contexts, device effects, capability
grants, and trace guarantees before any native or target-contract artifact is
trusted.

This package is intentionally small. Its value is that the same concepts used
by application workflows also apply to packet receive, packet transmit, and
interrupt-handler components that declare resources, capabilities, effects,
runtime traces, and target constraints before native evidence is trusted.

## Concepts Taught

- System profile packages with explicit resources, capabilities, effects, and
  guarantees.
- Ownership and borrowing for `rx buffer` and `packet metadata`.
- Transmit-path ownership for `tx descriptor` and borrowed `tx queue`.
- Region placement for packet processing memory.
- Scheduler task, priority, and timing declarations.
- Interrupt context, priority, and mask declarations.
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
- `examples/accepted/*.ail-spec.md`: minimal accepted fixtures for layout,
  allocation, locks, moves, mutable borrows, scheduler tasks, and interrupt
  rules.
- `examples/rejected/*.ail-spec.md`: minimal rejected fixtures for unknown
  resources, missing capabilities, invalid scheduler/interrupt references,
  use-after-release, use-after-move, and borrow conflicts.
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

The interactive manual Systems profile chapter composes conformance, native
compile, runtime trace, and v0.3 variant audit evidence:

```bash
python3 scripts/run_ail_interactive_manual.py --chapter systems-profile --run-checks
```

The native compile step used by the manual is:

```bash
cargo run -- ail-compile examples/network_driver.ail \
  --action NetworkPacketReceiver \
  --target linux-x86_64-elf \
  --out /tmp/ail-manual-systems-profile-network-driver.elf \
  --artifact-dir /tmp/ail-manual-systems-profile-native
```

Running `/tmp/ail-manual-systems-profile-network-driver.elf` should emit
resource, capability, effect, guarantee, and `trace PacketReceived` evidence.

The v0.3 Systems audit runs the receive, transmit, and interrupt-handler
variants as one fingerprinted bundle:

```bash
python3 scripts/run_v03_systems_profile_audit.py --artifact-dir /tmp/ail-v03-systems-profile-audit
```

Useful audit artifacts:

- `systems-profile-audit-report.txt`
- `manifest.v03-systems-profile-audit.txt`
- `receive-runtime-trace.txt`
- `transmit-runtime-trace.txt`
- `interrupt-handler-runtime-trace.txt`

The report ties the rejected unsupported-target `example-104` diagnostic
`AIL-BACKEND-001` to migration guidance: move Linux-only syscall effects
behind target-support metadata or choose `linux-x86_64-elf`.

## Rejected Fixtures

`example-106` is the catalog rejected fixture for this package. The
package-local rejected fixtures are broader and verify that the System profile
rejects a network device effect without a matching capability, blocking effects
in interrupt context, scheduler references to unknown contexts or tasks,
interrupt masks for unknown contexts, writes without ownership, use after move,
use after release, and conflicting shared/mutable borrows.

High-value next fixtures should add:

- borrowed packet metadata escaping the packet processing region;
- double release of `rx buffer`;
- interrupt-context effects that require a stronger capability;
- scheduler priority or interrupt-mask combinations that violate the package
  guarantees.

## Next Example To Read

Read `../c_interop.ail/README.md` after this package. It moves from
device-facing resources and effects to ABI-safe host calls, pointers, callbacks,
layout, and status-map diagnostics.

## v0.3 Learning Signal

Network Driver is the current low-level System profile anchor, but it is still
too narrow to prove realistic driver development. v0.3 now has deterministic
manual evidence for receive-path conformance, scheduler and interrupt fixtures,
packet-transmit conformance, native target artifacts, receive/transmit/
interrupt-handler runtime traces, and unsupported-target migration guidance.
The next bar is a small driver family with richer hardware target diversity and
device-specific failure recovery contracts.
