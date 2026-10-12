# Executable Unicode text operations

The active user goal is an agent-usable language that can replace Python, with
STE between specifications and binaries. Integer computation now passes the
frozen ladder; useful text processing is the next missing runtime capability.
This architectural increment preserves the larger goal and existing behavior.

## Contract

Actions gain four ordered statements:

```text
- the system joins text input.prefix and input.name into output.message
- the system counts characters in output.message into output.length
- the system slices text input.value from input.start to input.end into output.part
- the system finds text input.needle in input.value into output.position
```

Text operands and text destinations are declared `Text` fields. Count and find
destinations are `Int`. Slice bounds are ordinary checked integer expressions.
Each operation reads all operands before assigning its destination. Missing
text is an error, while an empty string is a valid value. Text is UTF-8; count,
slice and find positions count Unicode scalar values, not bytes or graphemes.
Slice bounds are half-open; negative bounds count from the end, bounds clamp to
the valid range, and end before start produces empty text. Find returns the
first matching character position, -1 if absent, and 0 for an empty needle.
No normalization is implicit. Text-producing results are immutable values:
copying and then overwriting a source cannot change an earlier result.

Limit each text operand/result to 1,048,576 UTF-8 bytes. Text-producing operations
charge max(result bytes, 1) against a shared 67,108,864-byte allocation budget
for the entire root action and its action callees. Count/find do not allocate.
Native uses a bounded immutable arena, so existing value copies remain safe.
VM applies the same deterministic budget even though Rust reclaims values.
Native find uses linear-time KMP with at most 4 MiB of temporary scratch,
released per search and excluded from the text-result allocation budget.
Invalid/missing text reports AIL-RUNTIME-TEXT; size/allocation exhaustion reports
AIL-RUNTIME-TEXT-LIMIT. Existing arithmetic error codes apply to slice bounds.
Errors produce no native result stdout. Invalid native UTF-8 must be rejected.

## Representation

`AilAction.text_operations: Vec<AilTextOperation>` and `AilActionStep::Text(index)`
preserve ordering with writes, control flow and calls. A dedicated text module
owns parsing, canonical source, typed Core construction/checking/reconstruction,
bytecode encoding/verification, and VM evaluation. Executable integer functions
remain integer-only; actions compose these text operations with integer calls.

Core uses `TextOperation` nodes with `text_kind=join|length|slice|find`, ordered
`text_step` edges, field operands `text_source`, `text_other`,
`text_destination`, and typed bound roots `text_start`, `text_end`. No
executable behavior comes from provenance. Duplicate/missing/unexpected edges,
wrong types, cross-function locals and unsupported profiles must be rejected.
Imports qualify global fields and bound expressions. Saved Core retains
duplicate executable edges for the checker. Application and C interop lower
the operations; other profiles reject them rather than silently omit them.

Bytecode:

- TEXT_JOIN: key, left, right
- TEXT_LENGTH: key, source
- TEXT_SLICE: key, source, start, end (bounds are integer postfix)
- TEXT_FIND: key, source, needle

Every opcode has exact operands and validated field/expression syntax. STE
provides exact sentence templates for each operation using technical-data IDs.
The normal checked Core -> STE -> bytecode path remains authoritative.

## Validation and alternatives

Using opaque effects would not provide computation; VM-only execution would
break the Specs-to-Binary objective. Four primitive operations across both
backends give a useful composable base while collections and I/O remain open.
This is not a claim of Python compatibility or full ASD-STE100 conformity.

Test real CLI/VM/ELF results on ASCII, combining marks and multibyte Unicode;
empty/missing inputs; embedded delimiters; negative/extreme bounds; changing
destinations and aliases; loops and nested actions; saved/imported Core;
malformed source, Core and bytecode; budget errors and invalid native UTF-8.
Keep the 16/16 frozen ladder and existing suite green.
