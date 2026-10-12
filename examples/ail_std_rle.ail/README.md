# AIL binary run-length library

## Purpose

This reusable Application package implements RLE32 encoding, decoding and output
size queries in pure AIL. It operates on arbitrary Bytes without host effects.

## Concepts Taught

- `encodedSize(data: Bytes) -> Int` computes the canonical encoding size.
- `encode(data: Bytes) -> Bytes` emits one count/payload packet per maximal run.
- `decodedSize(data: Bytes) -> Int` validates every packet and sums its counts.
- `decode(data: Bytes) -> Bytes` validates before expanding the output.

Import `../ail_std_rle.ail compatible ^0.1 as Rle` from a neighboring package.
The [file tool](../run-length-codec/README.md) and
[R027 benchmark](../../bench/R027-run-length-codec/intent.md) demonstrate named
calls, recursive composition, immutable bytes and explicit BYTES errors.
Other functions in the package are implementation helpers.

## Files To Inspect

The [source](spec.ail-spec.md) contains the algorithm; the
[reference](../../docs/ail/58-run-length-codec.md) defines the five-byte packet
format, bounds and error behavior. The [package manifest](ail-package.md) declares
an importable library with no root action entrypoint.

## Expected Replay Artifacts

After `cargo build`, run `python3 scripts/ail_rle_differential.py`.
It creates Core, rendered source, STE, bytecode and native artifacts in temporary
directories, comparing actual values and files with an independent sequential
Python reference. The [checked-in STE](../ste/run-length-codec.ail-ste) includes
this library through the importing file tool.

## Rejected Fixtures

The [frozen cases](../../bench/R027-run-length-codec/cases) cover incomplete
packets, zero counts, bad transports, output bounds and invalid modes. These
are runtime domain failures in a valid program. Separate explicit-error tests
reject malformed source, Core and bytecode raise statements before execution.

## Next Example To Read

Use [run-length-codec](../run-length-codec/README.md) to combine the library with
caller-bound binary files, or read the [expression library](../ail_std_expression.ail/README.md)
for a pure AIL parser and evaluator.

## v0.3 Learning Signal

This package supplies actual binary algorithms through both backends. It is
support-only in the historical replay catalog; adding it does not increase the
128 counted corpus examples. R027 and independent file comparisons measure its
behavior. Remaining ladder tiers and general Python replacement remain open.
