# Binary run-length codec

This application compresses and expands arbitrary binary files using the reusable
[AIL RLE library](../ail_std_rle.ail/spec.ail-spec.md). The algorithms are written
in AIL and compile through checked Core and executable STE to VM or native ELF.
Generated binaries perform their own computation without a Python interpreter
or external codec process.

## Format and library

RLE32 is a project-defined format. Each five-byte packet contains a nonzero
unsigned 32-bit big-endian count, followed by one byte to repeat. Empty input
encodes as empty. For example, `AAABB` becomes `00000003410000000242` in hex.
Encoding emits one packet per maximal run. Decoding accepts adjacent equal runs
as well as canonical encodings. It rejects incomplete packets and zero counts.
There is no archive header, checksum, filename or automatic format detection.
Data with short runs can grow; each distinct consecutive byte costs five bytes.

Import `../ail_std_rle.ail compatible ^0.1 as Rle` from a sibling example:

| Function | Result |
| --- | --- |
| `Rle.encodedSize(data: Bytes)` | Int canonical encoded byte count |
| `Rle.encode(data: Bytes)` | Bytes canonical encoding |
| `Rle.decodedSize(data: Bytes)` | Int decoded byte count after full format validation |
| `Rle.decode(data: Bytes)` | Bytes decoded data |

The other library functions support these four entry points internally. Encoding
preflights its size. Decoding checks the entire format and output size before
expanding a run. Both raise catchable `BYTES` for a result larger than 1,048,576
bytes; malformed packet streams also raise `BYTES`. Size queries can return
larger values. Ordinary invalid Bytes transports retain their existing errors.

The normal Bytes size, shared 64 MiB cumulative allocation, call-depth and fuel
limits apply. Fitting the per-value limit does not guarantee sufficient allocation
budget for a complex input. Arena exhaustion remains fatal `BYTES-LIMIT`.
Earlier successful intermediate allocations remain charged. The codec uses
balanced composition and repeated doubling to make useful large inputs possible.

## Run a transformation

From the repository root, after `cargo build`:

```bash
target/debug/ail ail-run examples/run-length-codec --action Transform \
  data.mode=encode data.input=hex:4141414242
target/debug/ail ail-run examples/run-length-codec --action Transform \
  data.mode=decode data.input=hex:00000003410000000242
```

`Transform` selects `out.value` (Bytes) and `out.error` (Text). Success has an empty
error; a caught domain error returns `hex:` and `BYTES`. An unknown or missing mode returns
`hex:` and `MODE`, without inspecting the input. Other runtime failures propagate.

## Process actual files

```bash
printf 'AAABB' > /tmp/rle-input.bin
target/debug/ail ail-run examples/run-length-codec --action EncodeFile \
  --read-file=input=/tmp/rle-input.bin --write-file=output=/tmp/rle-packed.bin
target/debug/ail ail-run examples/run-length-codec --action DecodeFile \
  --read-file=input=/tmp/rle-packed.bin --write-file=output=/tmp/rle-restored.bin
cmp /tmp/rle-input.bin /tmp/rle-restored.bin
```

Both file actions finish their pure transformation before opening the output.
Malformed data or an oversized result leaves an existing output untouched.
Successful writes replace its contents exactly. Existing file-grant rules and
partial-write behavior still apply.

Compile and execute a standalone decoder:

```bash
target/debug/ail ail-compile examples/run-length-codec --action DecodeFile \
  --target linux-x86_64-elf --out /tmp/rle-decode
/tmp/rle-decode --read-file=input=/tmp/rle-packed.bin \
  --write-file=output=/tmp/rle-native-restored.bin
cmp /tmp/rle-input.bin /tmp/rle-native-restored.bin
```

Export or run the same controlled-English intermediate:

```bash
target/debug/ail ail-ste examples/run-length-codec > /tmp/rle.ail-ste
target/debug/ail ail-run /tmp/rle.ail-ste --action Transform \
  data.mode=decode data.input=hex:00000003410000000242
```

The checked-in [STE example](../ste/run-length-codec.ail-ste) contains the flattened
library. See [R027](../../bench/R027-run-length-codec/intent.md) for the independent
benchmark contract and [the reference](../../docs/ail/58-run-length-codec.md) for
verification commands.
