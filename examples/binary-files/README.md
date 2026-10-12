# Inspect a binary header and copy a file

This AIL application reads arbitrary file bytes, reports selected PNG header
metadata, and writes an exact binary copy. Its reusable `readU32BE` and
`inspectHeader` functions execute in AIL on both the VM and native backend.

```bash
cargo build
target/debug/ail ail-run examples/binary-files --action Inspect \
  --read-file=input="$PWD/examples/binary-files/input.bin" \
  --write-file=copy=/tmp/ail-binary-copy.png \
  --write-file=report=/tmp/ail-binary-report.json

target/debug/ail ail-compile examples/binary-files --action Inspect \
  --target linux-x86_64-elf --out /tmp/ail-binary-files
/tmp/ail-binary-files \
  --read-file=input="$PWD/examples/binary-files/input.bin" \
  --write-file=copy=/tmp/ail-binary-copy.png \
  --write-file=report=/tmp/ail-binary-report.json
cmp examples/binary-files/input.bin /tmp/ail-binary-copy.png
cat /tmp/ail-binary-report.json
```

The supplied 71-byte fixture is a two-pixel RGBA PNG. It produces:

```json
{"bytes":71,"headerRecognized":true,"width":2,"height":1,"bitDepth":8,"colorType":6}
```

`inspectHeader` checks the minimum 33-byte prefix, PNG signature, first chunk
kind and length, and nonzero dimensions within the PNG integer range. It reads
multibyte values with the most significant byte first. The offsets and fields
follow the [W3C PNG specification](https://www.w3.org/TR/png-3/#11IHDR).

`headerRecognized` describes only these checks. This example does not validate
CRCs, depth/color combinations, compression or interlace settings, later chunks,
or compressed pixels. A damaged or truncated image can still have a recognized
header. An unrecognized prefix returns false and zero metadata while retaining
the byte count. Both kinds of input are copied unchanged; input files must fit
AIL's 1 MiB Bytes limit. Writes are ordinary ordered effects: a later failure
does not undo an earlier copy.

The fixture SHA-256 is
`6fe8b897998e41db82f635c2bc1ee3c552369debf7fe862f043f6c6feb2bb60b`.
The [STE export](../ste/binary-files.ail-ste) preserves both functions and file
effects. See [Bytes](../../docs/ail/56-bytes.md) for all operations and bounds.
