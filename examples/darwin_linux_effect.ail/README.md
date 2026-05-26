# Darwin Linux Effect Example

## Purpose

`darwin_linux_effect.ail` is the target-portability teaching package. It shows
how a Linux syscall effect can be valid for a Linux-oriented program while
remaining unsupported by a Darwin target contract. The example keeps target
behavior explicit instead of pretending one backend can silently reinterpret
another operating system's host boundary.

This package is small by design. It is a diagnostic example for backend
portability: AIL should reject or report unsupported target effects before a
binary or target contract is treated as valid.

## Concepts Taught

- Target-specific effects for a Linux syscall.
- Capability declarations for OS host boundaries.
- Darwin target support as a planned target contract, not a native Linux
  syscall runtime.
- Unsupported target diagnostics for effects that cannot be preserved across
  the selected backend.
- Release evidence that includes target contract reports and diagnostics
  instead of only VM execution.

## Files To Inspect

- `ail-package.md`: package target support for
  `aarch64-apple-darwin-libsystem-macho`.
- `spec.ail-spec.md`: canonical Linux syscall effect specification.
- `../examples.md`: `example-104` covers the rejected Darwin target-contract
  path for a Linux syscall effect.
- `../stories/example-104.md`: diagnostic story view for the unsupported target
  case.
- `../../docs/ail/22-backend-portability.md`: backend portability rules for
  Linux syscall ELF, Wasm sandbox contracts, and Darwin Mach-O contracts.

## Expected Replay Artifacts

Replay the corpus to inspect the target-portability diagnostic:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-darwin-linux-effect-examples --release-evidence
```

Useful artifacts to inspect after replay:

- `examples/example-104/diagnostics.txt`
- `examples/example-104/target-report.txt`
- `examples/example-104/user-story.txt`
- `failure-taxonomy.txt`

The direct Darwin target-contract check is:

```bash
cargo run -- ail-compile examples/darwin_linux_effect.ail \
  --target aarch64-apple-darwin-libsystem-macho \
  --artifact-dir /tmp/ail-darwin-linux-effect-contract
```

## Rejected Fixtures

`example-104` is the current rejected fixture for this package. It verifies that
a Linux syscall effect is not silently accepted as a Darwin Mach-O target
contract.

Useful v0.3 rejected fixtures should add:

- Linux file descriptor operations without a portable file capability;
- process effects that are unavailable in Wasm sandbox contracts;
- Darwin framework effects missing entitlement metadata;
- target reports that cannot preserve the trace mapping for an OS effect.

## Next Example To Read

Read `../stateful_counter.ail/README.md` after this package if you want the
smallest stateful Application example. Then read
`../incident_response.ail/README.md` for the high-level multi-module workflow
end of the capability ladder.

## v0.3 Learning Signal

Darwin Linux Effect proves the current unsupported-target diagnostic, but v0.3
needs a broader portability matrix. The next bar is a paired Linux, Wasm, and
Darwin example family where each target records the same user intent, the
different host boundary, and the exact reason an effect is accepted, adapted,
or rejected.
