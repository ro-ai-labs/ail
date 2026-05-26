# C Interop Example

## Purpose

`c_interop.ail` is the low-level host-boundary teaching package for ABI-safe C
integration. It shows how AIL can describe external libraries, pointer
ownership, borrowed mutable buffers, callbacks, `repr(C)` layout, status-map
failures, capability grants, and trace evidence in a form that can be checked
before host calls are trusted.

The package is useful because interop mistakes are common and expensive. AIL
must make pointer lifetimes, callback escape rules, nullable contracts, status
mapping, and layout assumptions explicit enough for both the compiler and a
reviewing agent to reject unsafe bindings.

## Concepts Taught

- C library imports for `zlib.compress2` and `libc.qsort`.
- Pointer ownership and `borrowed mutable` parameters.
- Callback contracts with `Callback<Pointer<Void>,Pointer<Void>,CInt>` and
  `noescape`.
- ABI layout checks for a packet header using `repr(C)`, size, alignment,
  offsets, and target metadata.
- Status-map conversion from C return values to AIL failures.
- Capability gates for host-library calls.
- Trace evidence for foreign calls, callback comparison, and failure mapping.

## Files To Inspect

- `ail-package.md`: C interop profile metadata, safety level, host-import
  features, and Wasm host-import target support.
- `spec.ail-spec.md`: canonical C interop specification.
- `../examples.md`: entries `example-85` through `example-89` cover accepted C
  interop prompt surfaces; `example-105` covers an invalid nullable-to-non-null
  pointer diagnostic.
- `../stories/example-85.md` through `../stories/example-89.md`: regenerated
  story views for accepted interop examples.
- `../stories/example-105.md`: diagnostic story view for invalid interop
  repair.

## Expected Replay Artifacts

Replay the corpus to inspect C interop artifacts:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-c-interop-examples --release-evidence
```

Useful artifacts to inspect after replay:

- `examples/example-85/checked.ail-core.txt`
- `examples/example-85/artifact.ailbc.json`
- `examples/example-85/vm-trace.txt`
- `examples/example-89/target-report.txt`
- `examples/example-105/diagnostics.txt`

The direct conformance check is:

```bash
cargo run -- ail-conformance examples/c_interop.ail --artifact-dir /tmp/ail-c-interop-conformance
```

## Rejected Fixtures

`example-105` is the current corpus-level invalid interop fixture. It verifies
that AIL rejects a nullable pointer where the imported C boundary requires a
non-null contract.

The package should grow package-local rejected fixtures for:

- owned pointer parameters without release semantics;
- borrowed pointer escape past the call boundary;
- mutable pointer aliasing across arguments;
- callback storage when the callback is declared `noescape`;
- missing or incomplete status-map entries.

## Next Example To Read

Read `../darwin_linux_effect.ail/README.md` after this package. It shifts from
C ABI contracts to target-specific OS effects and explains why Linux syscall
behavior must be represented differently for Darwin target contracts.

## v0.3 Learning Signal

C Interop proves that AIL can describe host bindings, but v0.3 needs richer
repair tutorials around status-map coverage, pointer lifetimes, callback
escape, and layout drift. Those examples should teach the agent how to amend a
spec safely rather than simply report that interop is invalid.
