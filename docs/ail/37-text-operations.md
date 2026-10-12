# Unicode text operations

AIL actions can join, count, slice and search text through checked Core, STE,
the bytecode VM and native Linux x86-64 ELF. These operations compose with
integer expressions, control flow and integer function calls.

[Text transformations](49-text-transformations.md) extend these operations with
split, list join, replace and trim, in actions and typed value functions.

```text
An Input has:
- first: Text
- second: Text
- start: Int
- end: Int

An Output has:
- joined: Text
- part: Text
- count: Int
- position: Int

Action: Process.

When Process happens:
- the system joins text input.first and input.second into output.joined
- the system counts characters in output.joined into output.count
- the system slices text output.joined from input.start to input.end into output.part
- the system finds text input.second in output.joined into output.position
- the system returns output.joined
- the system returns output.part
- the system returns output.count
- the system returns output.position
- the system records a trace event named Processed
```

The complete runnable package is [examples/unicode-text](../../examples/unicode-text).
Its exported [STE program](../../examples/ste/unicode-text.ail-ste) can also be
checked, run and compiled directly.

## Values and positions

Text operands and text destinations must be declared `Text` fields. Character
counts and search positions require `Int` destinations. Slice bounds accept
checked integer expressions, for example `input.start + 1` and `output.count`.
Each operation reads all operands before replacing its destination, so joining
or slicing into a source field is safe. Copies retain their value when the
source is overwritten later.

Positions count Unicode scalar values. `Aé🙂` has length 3; `e` followed by a
combining acute accent has length 2. Positions do not count UTF-8 bytes or
visible grapheme clusters. No Unicode normalization is implicit.

Slices include the start and exclude the end. Negative bounds count from the
end; bounds outside the text clamp to its endpoints. If the end precedes the
start, the result is empty. For `Aé🙂`, `from -2 to 99` yields `é🙂`.
Search returns the first matching character position, or -1 if absent. An
empty needle matches at position 0. Missing text is an error; empty text is a
present, valid value.

## Runtime limits

Each text operand and text result can contain at most **1,048,576 UTF-8 bytes**.
Each join or slice charges `max(result_bytes, 1)` against a **67,108,864-byte**
allocation budget shared by the root action and all its action callees.
Count and search consume no text allocation budget. Replacing a value does not
refund the budget. The native backend uses an immutable arena for text results;
the VM applies the same budget while Rust manages its strings.
Native search uses a linear-time prefix-table algorithm with at most 4 MiB of
temporary scratch memory, released after each search. This scratch does not
consume the text-result budget.

Missing text or invalid native UTF-8 reports `AIL-RUNTIME-TEXT`. Text size or
allocation exhaustion reports `AIL-RUNTIME-TEXT-LIMIT`. Slice-bound expressions
use the existing integer errors for missing values, overflow and zero divisors.
Operations validate text inputs before evaluating slice bounds. Native errors
exit unsuccessfully with diagnostics on stderr and no result stdout.

These operations are available in actions under the Application and C interop
profiles. [Value functions](44-value-functions.md) use `the function` forms on
declared local Text fields. Other profiles reject text
operations; contract descriptions do not make them executable.

## Core and STE

Each `TextOperation` node carries `text_kind=join|length|slice|find`. Ordered
`text_step` edges place the operation among an action's writes, calls and control
statements. `text_source`, `text_other` and `text_destination` reference typed
global fields. Slice bounds use `text_start` and `text_end` edges to typed
`IntegerExpression` trees. The checker rejects wrong types, function-local
operands, missing or duplicate operands, unexpected edges and invalid ordering.
Saved Core and imported packages preserve these semantics.
Existing secret-read and secret-write rules apply to these executable operands,
including slice bounds, even if saved Core omits descriptive access edges.

The four exact STE templates are:

```text
Join text left_data and right_data into field destination_data.
Count characters in text source_data into field destination_data.
Slice text source_data from integer start_data to integer end_data into field destination_data.
Find text needle_data in text source_data into field destination_data.
```

Operands refer to declared technical data; data cannot inject instructions.
They lower to `TEXT_JOIN`, `TEXT_LENGTH`, `TEXT_SLICE` and `TEXT_FIND`.
Slice bounds use the same postfix integer encoding as other expressions.
Bytecode verification requires exactly the documented operands and valid
field/expression syntax.

## Run the example

```bash
cargo run -- ail-run examples/unicode-text --action Process \
  'input.first=Aé🙂' 'input.second=World' input.start=-4 input.end=99
cargo run -- ail-compile examples/unicode-text --all-actions \
  --target linux-x86_64-elf --artifact-dir /tmp/ail-text
/tmp/ail-text/target-Process.elf \
  'input.first=Aé🙂' 'input.second=World' input.start=-4 input.end=99
```

Expected results are `output.count=8`, `output.joined=Aé🙂World`,
`output.part=orld` and `output.position=3`. [Typed lists](38-typed-lists.md) now
support integer and text collections. General I/O, richer text operations and
broader Python compatibility remain part of the active goal.

For repeatable independent checks, run `python3 scripts/ail_text_differential.py`
after `cargo build`. It compares 605 fixed-seed VM/native cases with Python's
string operations, checks native UTF-8 rejection, and probes saved-Core access
rules. At the text milestone, the compiler suite passed 631 tests; the frozen
numeric ladder remains 16/16; neither result proves general Python replacement.
