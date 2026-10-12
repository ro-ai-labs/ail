# Run-length encoding and decoding in AIL

The [binary codec application](../../examples/run-length-codec/README.md) implements
the capability ladder's R027 using ordinary AIL functions and Bytes operations.
It imports [ail.std.rle](../../examples/ail_std_rle.ail/spec.ail-spec.md), compiles
through Core and STE, and executes on the VM and native Linux x86-64 backend.
There is no dedicated codec instruction or external host implementation.

RLE32 stores each maximal equal-byte run as a four-byte unsigned big-endian count
and one payload byte. Empty input encodes as empty. The encoder is canonical;
the decoder also accepts consecutive packets with equal payload. Counts must be
nonzero and complete packets must contain exactly five bytes. This format is
defined by this project and contains no integrity checksum or archive metadata.

The public library entry points are `encodedSize(Bytes) -> Int`,
`encode(Bytes) -> Bytes`, `decodedSize(Bytes) -> Int` and `decode(Bytes) -> Bytes`.
Size queries can exceed the Bytes value bound. Encoding checks the expected size
before construction; decoding validates every packet and checks total size before
expansion. Invalid format or a result over 1,048,576 bytes raises `BYTES` through
the new [explicit error statement](57-explicit-errors.md).

The encoder composes small blocks recursively and merges equal boundary runs.
The decoder composes packet ranges and expands individual runs by doubling.
These are library algorithms, not compiler special cases. Normal depth, fuel and
cumulative Bytes allocation limits still apply, so some valid inputs can exhaust
the shared arena even when the final result fits. Such exhaustion is fatal
`BYTES-LIMIT`; it is distinct from a preflight domain error.

`Transform` returns selected `out.value` and `out.error` for agent calls.
`EncodeFile` and `DecodeFile` use explicit input/output file grants and complete
the transformation before writing. An invalid source leaves an existing output
untouched. The application README gives source, STE, VM and standalone commands.

The [R027 contract](../../bench/R027-run-length-codec/intent.md) and independent
Python reference were frozen before implementation. The reference scans runs
sequentially; the AIL implementation uses balanced recursive composition.

```bash
cargo build
python3 scripts/ail_bench.py --rung R027-run-length-codec
python3 scripts/ail_rle_differential.py
```

The acceptance suite compares actual values and binary files through source,
saved Core, rendered source, STE and bytecode on both backends. It includes all
256 payload bytes, split-boundary runs, malformed packets, large count fields,
output preservation and a full-size decoded value. Passing this rung advances
the ladder; it does not establish the full Python-replacement or alpha gate.
