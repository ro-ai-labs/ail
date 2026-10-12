# Split, join, replace and trim text

Actions and value functions can transform Unicode text through checked Core,
STE, the bytecode VM and native Linux x86-64 ELF. These operations compose with
typed lists, Boolean predicates and caller-bound files for line and column processing.

```text
- the system splits text data.raw by data.separator into data.parts
- the system joins list data.parts with data.separator into data.raw
- the system replaces text data.needle in data.raw with data.replacement into data.raw
- the system trims text data.raw into data.raw
```

All operands are declared fields. Split produces `List<Text>`; list join reads
`List<Text>`. Every other operand and result has type `Text`. Executable value
functions use `the function` and declared `local.*` fields. All inputs are read
before the destination is changed, so text results can replace an input field.

## Exact behavior

| Operation | Behavior |
| --- | --- |
| Split | Match the complete, nonempty separator from left to right without overlap. Preserve leading, trailing and consecutive empty fields. Empty source produces `[""]`; empty separator is an error. |
| Join list | Decode all list items and place the exact separator between them. An empty list produces empty text; an empty separator is allowed. |
| Replace | Replace every nonoverlapping occurrence from left to right. Do not search the inserted replacement. An empty needle inserts the replacement at every Unicode scalar boundary, including both ends. |
| Trim | Remove the fixed whitespace set below from both ends. Preserve internal whitespace. |

Trim recognizes U+0009–000D, U+001C–0020, U+0085, U+00A0, U+1680,
U+2000–200A, U+2028–2029, U+202F, U+205F and U+3000. This pinned set of
29 scalars keeps both backends independent of the host Unicode version.
U+0000, U+200B and U+FEFF remain ordinary text. Operations preserve combining
sequences and Unicode spelling; no normalization is implicit.

These semantics follow Python's explicit-separator `split`, `join`,
all-occurrence `replace` and default `strip` operations. See the
[Python string reference](https://docs.python.org/3/library/stdtypes.html#string-methods)
and [Unicode White_Space property](https://www.unicode.org/Public/17.0.0/ucd/PropList.txt).

## Bounds and failures

Text operands and results are limited to **1,048,576 UTF-8 bytes**. List inputs
are limited to **1,048,576 raw bytes** and **65,536 items**. Split results must
fit both the item limit and **1,048,576 canonical JSON bytes**, including quotes,
commas and escaping. A join accepts a valid input list even if its hypothetical
canonical re-encoding would exceed that bound: only its text result is produced.

Each text result charges `max(result_bytes, 1)` once to the shared
**67,108,864-byte Text arena**, including unchanged or empty results. Split
charges its canonical list bytes once to the separate **67,108,864-byte
List/Json arena**. It does not charge Text for individual parts. Function returns
retain the existing one-copy charge. Overwriting a destination does not refund
an allocation; matching and parsing scratch does not consume either result arena.

Validation order is observable:

1. Split checks source text, separator text, nonempty separator, list result
   bounds and list allocation.
2. Join checks the complete source list, separator text, text result bounds and
   text allocation.
3. Replace checks source, needle and replacement text, then text result bounds
   and allocation.
4. Trim checks source text, then text result bounds and allocation.

Missing or malformed text and empty split separators report `AIL-RUNTIME-TEXT`;
text size/allocation failures report `AIL-RUNTIME-TEXT-LIMIT`. Invalid join-list
inputs report `AIL-RUNTIME-LIST`; list bounds and split allocation failures report
`AIL-RUNTIME-LIST-LIMIT`. Failed operations preserve the destination and both
allocation counters. Diagnostics do not include operand contents.

Split and nonempty-needle replace use bounded linear matching, including for
repetitive inputs. Results remain valid across later operations and function calls.

## Core, STE and profiles

`TextOperation` uses `text_kind=split|join_list|replace|trim`. Ordered `text_step`
edges attach operations to executable actions. `text_source`, `text_other` and
`text_destination` carry source, separator/needle and destination fields;
replace also uses `text_replacement`. Trim needs only source and destination.
The checker requires exact operation metadata, attribute-free unique operand
edges and genuine typed fields in the correct scope. Secret-read analysis
includes every input, including all three replace inputs. Saved Core and
namespaced imports preserve these edges. Functions use the existing typed local
operation schema.

The exact STE templates and bytecode operations are:

| Opcode | STE template |
| --- | --- |
| `TEXT_SPLIT` | `Split text in field {source} by field {separator} into field {key}.` |
| `TEXT_JOIN_LIST` | `Join text list in field {source} with field {separator} into field {key}.` |
| `TEXT_REPLACE` | `Replace text from field {needle} in field {source} with field {replacement} into field {key}.` |
| `TEXT_TRIM` | `Trim whitespace from field {source} into field {key}.` |

Operands remain separately declared technical data. These operations require the
Application or C interop profile. Legacy descriptive functions and compiler-pass
bodies cannot execute them. Static failures use `AIL-TEXT-001`; malformed
bytecode uses `AILBC012`. Actions retain the trace `computed text <destination>`;
local function computations add no implicit trace.

## Run a file-processing example

[Tabular Report](../../examples/tabular-report) normalizes CRLF, splits lines
and columns, trims fields, filters records and joins accepted names. It uses
reusable functions and writes a JSON report. Its checked-in sample yields
2 accepted rows, 2 rejected rows, 5 units and 850 cents.

```bash
cargo run -- ail-run examples/tabular-report --action Summarize \
  --read-file=input=examples/tabular-report/input.tsv \
  --write-file=report=/tmp/tabular-report.json
```

The [exported STE](../../examples/ste/tabular-report.ail-ste) exposes the same
program. After `cargo build`, run
`python3 scripts/ail_text_transform_differential.py` for independent generated
comparisons with Python and actual file reports. The
[implementation record](../superpowers/plans/2026-10-11-text-transformations.md)
records review and verification. Broader libraries and Python replacement remain
part of the active objective.

The 2026-10-11 verification passed 1,064 Rust tests, strict Clippy and formatting,
independent Python comparisons for 1,000 transformation datasets and 54 generated
TSV reports across the public artifact routes. The frozen benchmark remains 16/16.
