# AIL Backend Portability

## Purpose

AIL portability is defined through explicit target contracts, not by assuming a
single native backend. The Linux x86_64 ELF backend is the first native target,
but AIL packages lower through portable AIL-Core and AIL-Bytecode boundaries
before backend-specific artifacts.

## Target Triple

Target identity:

```text
architecture-vendor-os-abi-format
```

Initial targets:

- `x86_64-unknown-linux-syscall-elf`
- `wasm32-unknown-sandbox-wasm`
- `aarch64-apple-darwin-libsystem-macho`

The target triple participates in package conformance, backend reports, native
trace metadata, and stable artifact manifests.

## Backend Contract

Every backend declares:

- executable format
- object format
- calling convention
- ABI data layout
- syscall or host API surface
- standard library support
- C interop support
- supported effect classes
- unsupported effect diagnostics
- trace preservation strategy
- artifact verifier

## Artifact Boundaries

AIL lowering uses explicit boundaries:

```text
AIL-Core
  -> checked AIL-Bytecode
  -> target lowering plan
  -> target artifact
  -> verifier report
  -> semantic explanation
```

Native artifacts are never treated as self-explanatory. Each native artifact
has a manifest linking it to package hash, AIL-Core hash, bytecode hash, target
triple, backend version, imported symbols, syscalls, and trace mapping.

## Linux x86_64 ELF

Target 1:

```text
target: x86_64-unknown-linux-syscall-elf
format: ELF64 executable
calling convention: System V AMD64 for internal calls
host boundary: Linux syscall ABI
runtime input: argv key=value pairs for first slice
```

The current prototype CLI spells this backend target as `linux-x86_64-elf`.
Manifest `target-support` matching treats that CLI name as an alias for the
canonical `x86_64-unknown-linux-syscall-elf` target triple.

Supported first-slice effects:

- process exit
- stdout write
- deterministic argument decoding
- trace emission

Unsupported effects produce blocking diagnostics before native emission.

## Wasm Sandbox

Target 2:

```text
target: wasm32-unknown-sandbox-wasm
format: WebAssembly module
host boundary: declared imports only
memory: linear memory
capabilities: host import capabilities
```

The Wasm backend is the portable sandbox target for browser, plugin, and
embedded host contexts. Host imports map to `ExternalBinding` nodes with
explicit capability grants.

Current implementation status: saved AIL-Bytecode can be compiled to a
deterministic Wasm sandbox contract report with:

```text
ail ail-compile <package-or-artifact.ailbc.json> \
  --action <ActionName> \
  --target wasm32-unknown-sandbox-wasm \
  --artifact-dir <dir>
```

Checked-core input is also supported through `--core-file <checked-core>`.
The package, checked core, or saved bytecode must declare
`wasm32-unknown-sandbox-wasm: supported` or
`wasm32-unknown-sandbox-wasm: supported-with-host-imports` in `target-support`.
The command writes `wasm-contract-report.txt`,
`wasm-contract-report.fingerprint.txt`, `dependency-report.txt`,
`dependency-report.fingerprint.txt`, `manifest.ail-compile.txt`, and
`manifest.fingerprint.txt` alongside the saved `artifact.ailbc.json` and its
fingerprint. It intentionally does not write `target.elf` or an executable
`.wasm` module yet. The contract report records `bytecode-level
portable-vm-contract`, `bytecode-container wasm-sandbox-contract`,
`bytecode-format wasm32-contract-report`, `host-boundary
declared-imports-only`, `host-import-metadata present-in-saved-bytecode`,
selected action, target-support status, and trace preservation requirements
across the selected action's reachable `CALL_ACTION` graph. Saved bytecode
preserves `ExternalBinding` declarations, so the report enumerates each
declared host import with library, symbol, ABI, input, output, status-map,
capability, and trace metadata. Reusing an artifact directory that still
contains executable outputs such as `target.elf`, `target.wasm`, or native
bytecode reports is rejected. Older saved bytecode without the
`external_bindings` field remains loadable, but the report marks host-import
metadata as absent and does not enumerate import dependencies.

## Additional OS Target Plan

Target 3:

```text
target: aarch64-apple-darwin-libsystem-macho
format: Mach-O executable or dylib
calling convention: Apple arm64
host boundary: libSystem and platform entitlements
```

The first Darwin target must prove:

- package hash is preserved in the manifest
- Mach-O symbol table links to AIL external bindings
- libSystem calls map to declared capabilities
- traces map back to AIL-Core node IDs
- unsupported Linux-only syscalls are rejected

## Executable Formats

Backend conformance covers:

- ELF
- Mach-O
- PE/COFF
- Wasm
- relocatable object files
- static libraries
- dynamic libraries

Each format has its own manifest section and verifier.

## Standard Library Target Support

Every standard library package declares target support:

```text
target-support:
  x86_64-unknown-linux-syscall-elf: supported
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
  aarch64-apple-darwin-libsystem-macho: planned-contract
```

The status vocabulary is closed: `supported`, `supported-with-host-imports`,
and `planned-contract`.

The checker rejects packages that lower to unsupported target/effect pairs.

## Capability Mapping Per OS

Capabilities are target-specific at the host boundary:

- file access maps to Linux syscalls, Wasm host imports, or OS framework calls
- network access maps to socket syscalls, host imports, or platform frameworks
- process access maps to syscall sets or is rejected in sandbox targets
- random and clock map to declared runtime providers
- C interop maps to target ABI and linker contracts

## Native Trace Preservation

Native trace mapping requires:

- source package hash
- AIL-Core node ID
- bytecode instruction ID
- native symbol or offset
- emitted trace event
- backend verifier confirmation

If a native target cannot preserve trace mapping for an effect, that effect is
not supported for that target.

## Backend Conformance Manifest

```json
{
  "schema": "ail-backend-conformance.v0",
  "target": "x86_64-unknown-linux-syscall-elf",
  "backend": "ail-native-linux",
  "package_hash": "ail-package:fnv64:...",
  "core_hash": "ail-core:fnv64:...",
  "bytecode_hash": "ail-bytecode:fnv64:...",
  "supported_effects": ["stdout", "process-exit"],
  "imported_symbols": [],
  "syscalls": ["write", "exit"],
  "trace_mapping": [],
  "verifier": {
    "status": "accepted",
    "diagnostics": []
  }
}
```
