# AIL Manual: Systems Profile

## Purpose

The Systems profile chapter proves that low-level AIL is not only prose. It
checks the `network_driver.ail` package through conformance fixtures, lowers the
accepted `NetworkPacketReceiver` component through AIL-Core and bytecode into a
native Linux x86_64 ELF executable, then runs that executable to observe the
resource/effect trace. It also runs the v0.3 Systems audit, which compiles and
runs receive, transmit, and interrupt-handler runtime variants and ties
unsupported-target migration guidance to the rejected catalog entry
`example-104`.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter systems-profile --run-checks
```

## Checks

The chapter runs these deterministic steps:

```text
check-network-driver-conformance
compile-network-driver-native
run-network-driver-native
audit-systems-profile-variants
```

The conformance step validates the package-local accepted and rejected fixtures:

```sh
cargo run -- ail-conformance examples/network_driver.ail --artifact-dir /tmp/ail-manual-systems-profile-conformance
```

This must accept the scheduler and interrupt examples:

```text
accepted: scheduler-task-minimal.ail-spec.md
accepted: scheduler-task-priority-minimal.ail-spec.md
accepted: scheduler-task-timing-minimal.ail-spec.md
accepted: interrupt-context-minimal.ail-spec.md
accepted: interrupt-mask-minimal.ail-spec.md
accepted: interrupt-priority-minimal.ail-spec.md
accepted: packet-transmit-minimal.ail-spec.md
```

It must also reject invalid hardware-facing contracts with stable diagnostics:

```text
AIL033 system component TimerInterruptHandler performs blocking effect 'wait for scheduler' in interrupt context
AIL035 system component PacketScheduler schedules task 'packet poller' for unknown context 'process'
AIL040 system component TimerMask configures interrupt mask for unknown context 'interrupt'
```

The native compile step runs:

```sh
cargo run -- ail-compile examples/network_driver.ail \
  --action NetworkPacketReceiver \
  --target linux-x86_64-elf \
  --out /tmp/ail-manual-systems-profile-network-driver.elf \
  --artifact-dir /tmp/ail-manual-systems-profile-native
```

Then the chapter executes:

```sh
/tmp/ail-manual-systems-profile-network-driver.elf
```

## Evidence

The conformance artifacts should include:

```text
/tmp/ail-manual-systems-profile-conformance/conformance-report.txt
/tmp/ail-manual-systems-profile-conformance/conformance-report.fingerprint.txt
/tmp/ail-manual-systems-profile-conformance/manifest.ail-conformance.txt
```

The native compile artifacts should include:

```text
/tmp/ail-manual-systems-profile-native/checked.ail-core.txt
/tmp/ail-manual-systems-profile-native/artifact.ailbc.json
/tmp/ail-manual-systems-profile-native/native-bytecode-report.txt
/tmp/ail-manual-systems-profile-native/dependency-report.txt
/tmp/ail-manual-systems-profile-native/manifest.ail-compile.txt
```

The compile manifest must record the target contract:

```text
machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable
```

The runtime trace must include:

```text
system component Network packet receiver started
system resource network device:Device
system resource packet metadata:Buffer
system resource rx buffer:Buffer
system owns rx buffer
system borrows packet metadata
system places rx buffer in packet processing region
system capability access network device
system effect read network device
system effect release rx buffer
trace PacketReceived
```

The v0.3 Systems audit runs:

```sh
python3 scripts/run_v03_systems_profile_audit.py --artifact-dir /tmp/ail-manual-systems-profile-audit
```

It must preserve:

```text
/tmp/ail-manual-systems-profile-audit/systems-profile-audit-report.txt
/tmp/ail-manual-systems-profile-audit/systems-profile-audit-report.fingerprint.txt
/tmp/ail-manual-systems-profile-audit/manifest.v03-systems-profile-audit.txt
/tmp/ail-manual-systems-profile-audit/receive-runtime-trace.txt
/tmp/ail-manual-systems-profile-audit/transmit-runtime-trace.txt
/tmp/ail-manual-systems-profile-audit/interrupt-handler-runtime-trace.txt
```

The audit report must include:

```text
runtime-variant receive action NetworkPacketReceiver target linux-x86_64-elf trace PacketReceived
runtime-variant transmit action NetworkPacketTransmitter target linux-x86_64-elf trace PacketTransmitted
runtime-variant interrupt-handler action TimerInterruptHandler target linux-x86_64-elf trace TimerInterruptHandled
unsupported-target-migration example-104 AIL-BACKEND-001 aarch64-apple-darwin-libsystem-macho
unsupported-target-guidance move linux-only syscall effects behind target-support metadata or choose linux-x86_64-elf
audit-result accepted
```

This chapter raises the v0.3 bar for Systems examples: low-level examples must
show hardware-facing resources, capabilities, scheduler or interrupt rules,
diagnostics for rejected contracts, native target evidence, observable receive,
transmit, and interrupt-handler runtime traces, and a reviewable migration path
when a package requests an unsupported target.
