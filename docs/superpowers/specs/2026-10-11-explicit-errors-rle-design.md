# Explicit errors and a pure AIL binary codec

## Intent and evidence

Continue the user goal: Specs → ASD-STE100-based executable intermediate → binary,
then a language usable by agents in place of Python. The previous goal turn was
progress: Bytes and binary file I/O passed 1,353 Rust tests, 20 differential
suites and the 17/17 ladder. Its 146 frozen inputs were rehashed unchanged.
R027 run-length encoding/decoding remains an explicit unimplemented ladder item.
Reusable functions also cannot explicitly reject domain-invalid input; only
incidental runtime failures can currently trigger recovery.

Add an explicit raise statement and implement the binary codec in ordinary AIL,
with independent Python expectations. This is an architectural increment, not
completion of the overall goal. Networking, broader data abstractions and other
ladder tiers remain open. The continuing user instruction authorizes reversible
implementation; preserve the existing shared worktree and read-only .git. Record
reviews and baseline hashes without commits or worktree creation.

Alternatives considered: sentinel return values make every caller guess whether
empty output is success; a dedicated codec opcode would avoid testing the
language's ability to implement a real algorithm. Use ordinary typed functions
and the existing structured recovery machinery, with one explicit raise opcode.

## Explicit errors

Actions use `the system raises error CATEGORY`; executable functions use
`the function raises error CATEGORY`. CATEGORY is one literal from the existing
fourteen catchable categories: INT, OVERFLOW, DIVZERO, FLOAT, BOOL, TEXT, LIST,
INDEX, JSON, JSON-PATH, PROCESS, TIMEOUT, PATH, BYTES. No message, dynamic operand,
`any`, compound category or fatal category is allowed. Malformed reserved forms
reject, including in descriptive functions and unsupported profiles.

Represent this as `AilControl::Raise { category: String }`, with Core
`Control` and `control_kind=raise`, exactly one `error_category` attribute,
no operand edges, ordinary ownership/provenance and ordered `control_step`.
Functions use their existing owned control schema. Reject duplicate/extra/hidden
attributes, operand edges, forged identities/provenance and unrelated ownership.
Roundtrip source, saved Core, rendered source and imports without changing the
category or instruction position. Application and C interop profiles support it.

Bytecode `RAISE` has exactly one operand `category`. STE template is
`Raise error {category}.` with the category in technical data. Reuse existing
control/function static diagnostics and AILBC019 for invalid raise protocol.
Whole-program verification rejects invalid instructions even in dead code.

RAISE fails the current statement with `AIL-RUNTIME-CATEGORY`. Existing recovery
selects the nearest matching protected scope; a raise in a recovery body searches
outer scopes. Unmatched errors propagate out of functions/actions and terminate
with the same category on both backends. Do not publish a function result, execute
later statements or roll back earlier state/file effects. No operand content or
custom diagnostic payload is exposed. Keep category indices/masks unchanged.
Use the existing native error labels and unwind machinery, including depth,
handler/output selection, fuel and temporary mapping cleanup. Include integer
functions as well as general typed functions. Raise is a terminating path;
functions whose reachable paths all raise need no unreachable dummy return.
Raise-containing functions must not have reachable fallthrough; preserve the
existing treatment of functions with no raise statement.

## RLE32 binary format

This is a small project-defined format, not a standard archive format. Each run
is five bytes: a nonzero unsigned 32-bit count in big-endian order followed by
one arbitrary byte. Empty data encodes as empty. The encoder emits one packet per
maximal equal-byte run; its output is canonical. A decoder accepts adjacent runs
of the same byte, combines their meaning and rejects truncated packets or zero
counts. All 256 byte values are ordinary payload, including NUL and invalid UTF-8.

Public pure library `examples/ail_std_rle.ail`, imported as `Rle`:

- `encodedSize(data: Bytes) -> Int`: canonical encoded size, without constructing
  the encoding. This may exceed the Bytes bound.
- `encode(data: Bytes) -> Bytes`: canonical RLE32; reject an encoded result over
  1,048,576 raw bytes by preflighting encodedSize and explicitly raising BYTES
  before constructing the encoding.
- `decodedSize(data: Bytes) -> Int`: validate the entire packet stream and return
  its mathematical output size (can exceed the Bytes bound); malformed format
  explicitly raises BYTES. Sum is bounded by input size and u32 counts, fits i64.
- `decode(data: Bytes) -> Bytes`: fully validate first; if decoded size exceeds
  1,048,576, explicitly raise BYTES before expansion; otherwise decode exactly.

Invalid Bytes transports use the existing BYTES error. Oversized transport and
shared allocation exhaustion still produce fatal BYTES-LIMIT, and all existing
function depth/fuel/arena limits apply. The library does not impose a smaller
fixed input-size limit or bypass normal allocation accounting. As with other
AIL programs, fitting the per-value bound does not guarantee enough cumulative
arena for an arbitrary workload. Document this distinction and test large useful
inputs, including a full-size decoded result.

Use balanced recursive composition with small bounded scanning blocks and
doubling for repeated-byte expansion, to avoid appending across an entire large
output one byte at a time. Encoder boundary packets with the same byte merge by
adding their counts, so block boundaries never change the canonical format.
The reference must use an independent sequential run scan/expansion algorithm.
No compiler special case, codec opcode, external process or host evaluator.

## Entry points and observable behavior

`examples/run-length-codec` imports the library and provides:

- `Transform`: data.mode Text (`encode` or `decode`), data.input Bytes; returns
  selected out.value Bytes and out.error Text. Success error is empty. Unknown or missing
  mode returns empty Bytes and `MODE`; a caught BYTES returns empty and `BYTES`.
  Mode is checked before calling either codec. Other runtime errors stay fatal.
- `EncodeFile` and `DecodeFile`: read bytes from resource input, perform the pure
  codec, then write bytes to output. Errors terminate; malformed input must leave
  an existing output untouched. Existing grant and fatal-error contracts apply.

R027-run-length-codec uses Transform with independently frozen cases and a Python
oracle. Include exact canonical encodings, noncanonical accepted input, all byte
values, run counts around 255/256/65535/65536, cross-block runs, malformed counts,
odd/truncated lengths, bad transports, mode priority and seeded mixtures. Large
file tests additionally check exact 1 MiB decode, over-limit rejection before any
write, real binary roundtrips, and unchanged output on errors. Keep every old
ladder input/fingerprint fixed; only promote the new rung after all cases pass.

## Verification

Write/run RED public protocol tests before raise implementation, and freeze/run
the new rung as unimplemented before adding its AIL program. Test all fourteen
explicit categories, nonmatching/outer handlers, re-raise, recursive calls,
all-raise functions, immutable outputs, aliasing and later-effect suppression
through source/Core/rendered/STE/bytecode and both VM/native. Direct malformed
bytecode must reject before effects, including unreachable instructions.

Independent codec differential testing must compare actual bytes with sequential
Python expectations across all frontend/backend routes, test imported calls and
file effects, and demonstrate sensitivity to a meaningful implementation mutation.
Reciprocal frontend/native/library/oracle reviews precede final verification.
Run fmt, diff, build, strict all-target Clippy, serial all-target Rust tests,
focused raise/RLE and existing differential suites, full unchanged prior ladder
plus R027, exact STE export, local docs links and baseline-input checks.
