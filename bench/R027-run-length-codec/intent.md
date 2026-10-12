rung: R027-run-length-codec
tier: 2
action: Transform
targets: vm,native
blocked-on: explicit domain errors and pure AIL RLE32 encoding and decoding

# R027 — Encode and decode binary runs

Transform reads `data.mode: Text` and `data.input: Bytes`, selecting only
`out.error: Text` and `out.value: Bytes`. Modes are exactly `encode` and `decode`.
Success has empty error; a caught BYTES error has empty Bytes and error BYTES.
Unknown mode has empty Bytes and error MODE before inspecting the input.

RLE32 is a sequence of five-byte packets: a nonzero unsigned big-endian u32
count followed by one arbitrary byte. Empty input encodes as empty. Canonical
encoding has one packet per maximal equal-byte run. Decoding accepts adjacent
packets with equal payloads, rejects zero counts and truncated packets, and
validates the entire stream before expansion. Counts are raw bytes, not Unicode.

Bytes use exact lowercase `hex:` transport and at most 1,048,576 raw bytes.
Encoded/decoded results over the raw bound explicitly raise BYTES. Existing
transport/allocation bounds remain fatal BYTES-LIMIT. No smaller fixed codec
bound, dedicated codec opcode, host codec, or external process may be substituted
for ordinary AIL functions. Cumulative allocation limits still apply.

The independent Python reference uses a sequential run scan and sequential
expansion, distinct from the required balanced AIL implementation. Inputs were
frozen and the unimplemented RED recorded before program.ail was added.
Full contract: docs/superpowers/specs/2026-10-11-explicit-errors-rle-design.md.
