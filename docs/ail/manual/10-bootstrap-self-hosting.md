# AIL Manual: Bootstrap Self-Hosting

## Purpose

The bootstrap self-hosting chapter runs the AIL-authored toolchain agent and
the AIL-Meta `InferReadPermissions` compiler pass as one deterministic
bootstrap bundle. It proves that the toolchain can compile its own agent
package, run an AIL-authored compiler pass over that agent's checked Core,
rerun the pass to a fixed point, emit native Linux ELF artifacts, and verify
the resulting manifest with the AIL-authored agent.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter bootstrap-self-hosting --run-checks
```

## Workflow

Run the bootstrap bundle:

```sh
cargo run -- ail-bootstrap examples/ail_toolchain_agent.ail \
  --pass examples/compiler_pass.ail \
  --agent examples/ail_toolchain_agent.ail \
  --target linux-x86_64-elf \
  --artifact-dir /tmp/ail-manual-bootstrap-self-hosting
```

`ail-bootstrap` preserves repeated `--pass` values as an ordered user-supplied
pass sequence. The current promoted path accepts the single
`InferReadPermissions` pass and verifies its fixed point; a duplicate pass
before the fixed-point gate is rejected as a pass-order conflict fixture.

The command writes source snapshots for both AIL packages, checked Core,
bytecode, native ELF artifacts, conformance reports, fixed-point evidence,
pass-composition evidence, host-boundary evidence, dependency evidence, native
handoff evidence, and a fingerprinted bootstrap manifest.

The fixed-point report must include:

```text
bootstrap-fixed-point-report.txt
bootstrap-fixed-point-report.fingerprint.txt
fixed-point: ok
second-pass-changed false
```

The pass-composition report must include:

```text
bootstrap-pass-composition-report.txt
bootstrap-pass-composition-report.fingerprint.txt
composition-pass-count 1
composition-variant-count 2
composition-pass 1 InferReadPermissions
composition-variant 1 toolchain-agent-fixed-point pass InferReadPermissions status ok
composition-variant 2 compiler-pass-self-check pass InferReadPermissions status ok
composition-input toolchain-agent.checked.ail-core.txt
composition-output toolchain-agent.pass-output.ail-core.txt
composition-fixed-point bootstrap-fixed-point-report.txt
pass-order-diagnostics bootstrap-pass-order-diagnostics.txt
pass-order-status ok
```

The pass-order diagnostics report is reviewer-visible even when the accepted
composition remains valid:

```text
bootstrap-pass-order-diagnostics.txt
bootstrap-pass-order-diagnostics.fingerprint.txt
AIL-Bootstrap-Pass-Order-Diagnostics:
user-pass-sequence-count 1
user-pass 1 examples/compiler_pass.ail
accepted-pass-sequence-count 1
reviewed-pass-order-conflict-count 1
reviewed-pass-order-conflict AIL-BOOTSTRAP-PASS-ORDER-001 duplicate-pass-before-fixed-point pass InferReadPermissions
pass-order-status ok
conflict-resolution fixed-point-gate-required
composition-variant-count 2
```

The conflicting-order fixture is exercised by the manual and release audit:

```sh
cargo test cli_ail_bootstrap_rejects_duplicate_user_pass_sequence_with_diagnostics --test ail_toolchain -- --exact
```

It records:

```text
bootstrap-pass-order-diagnostics.txt
pass-order-status conflict
user-pass-sequence-count 2
reviewed-pass-order-conflict AIL-BOOTSTRAP-PASS-ORDER-001 duplicate-pass-before-fixed-point
conflict-resolution fixed-point-gate-required
diagnostic-visibility reviewer-visible
```

The host-boundary and dependency reports must include:

```text
bootstrap-host-boundary-report.txt
bootstrap-host-boundary-report.fingerprint.txt
no-host-backend-source true
generated-host-language-source none
bootstrap-dependency-report.txt
bootstrap-dependency-report.fingerprint.txt
host-language-runtime none
dynamic-linker none
shared-libraries none
```

The native handoff report must include:

```text
bootstrap-handoff-report.txt
bootstrap-handoff-report.fingerprint.txt
handoff-native-role toolchain-agent all-actions ok count 18
handoff-native-role compiler-pass all-actions ok count 1
handoff-native-role agent all-actions ok count 18
handoff-native-action compiler-pass-InferReadPermissions.elf ok trace ReadPermissionAdded
handoff-native-action agent-VerifyBootstrapManifest.elf ok trace BootstrapManifestVerified
```

The manifest must include:

```text
manifest.ail-bootstrap.txt
manifest.fingerprint.txt
AIL-Bootstrap-Manifest:
target linux-x86_64-elf
no-host-backend-source true
toolchain-agent-pass-output toolchain-agent.pass-output.ail-core.txt
toolchain-agent-pass-trace toolchain-agent.pass-trace.txt
bootstrap-fixed-point bootstrap-fixed-point-report.txt
bootstrap-pass-composition bootstrap-pass-composition-report.txt
bootstrap-pass-order-diagnostics bootstrap-pass-order-diagnostics.txt
bootstrap-native-bytecode bootstrap-native-bytecode-report.txt
bootstrap-host-boundary bootstrap-host-boundary-report.txt
bootstrap-dependencies bootstrap-dependency-report.txt
bootstrap-handoff bootstrap-handoff-report.txt
```

This is not a claim that the full compiler is self-hosted. It is the v0.3
bootstrap evidence slice: AIL-authored toolchain and compiler-pass packages
produce checked artifacts, the compiler pass reaches a stable fixed point over
the toolchain agent Core, the compiler pass is also replayed over its own
Compiler-profile Core as a second composition variant, pass-order conflicts are
reported in fingerprinted diagnostics artifacts for both the accepted sequence
and a duplicate user-supplied sequence fixture, and the native handoff remains
executable without generated host-language source.
