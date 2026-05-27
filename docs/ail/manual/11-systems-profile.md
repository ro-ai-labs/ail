# AIL Manual: Systems Profile

## Purpose

The Systems profile chapter proves that low-level AIL is not only prose. It
checks the `network_driver.ail` package through conformance fixtures, lowers the
accepted `NetworkPacketReceiver` component through AIL-Core and bytecode into a
native Linux x86_64 ELF executable, then runs that executable to observe the
resource/effect trace.

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

This chapter raises the v0.3 bar for Systems examples: low-level examples must
show hardware-facing resources, capabilities, scheduler or interrupt rules,
diagnostics for rejected contracts, native target evidence, and an observable
runtime trace.
