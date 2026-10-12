# Integer comparisons, branches and loops

AIL executes structured control flow through checked Core, executable STE,
verified bytecode, and the VM or native Linux x86-64 ELF backend.

## Source syntax

Every statement remains a bullet. Indentation is cosmetic; explicit closing
markers determine block structure. For example:

```text
The application Sum manages integer totals.

A Number has:

- value: Int

A Work has:

- i: Int
- total: Int

Action: sum numbers.

When sum numbers happens:

- the system computes work.i as 1
- the system computes work.total as 0
- while work.i <= number.value:
- the system computes work.total as work.total + work.i
- the system computes work.i as work.i + 1
- end while
- the system returns work.total
- the system records a trace event named Calculated
```

Branches use `if <comparison>:`, an optional `otherwise:`, and `end if`.
Loops use `while <comparison>:` and `end while`. Blocks can nest. `break`
exits the nearest loop; `continue` returns to its condition, including when
used inside nested branches. Both are errors outside a loop. The checker
rejects unmatched, duplicate and crossed delimiters. Each action permits at
most 64 nested blocks and 1,024 control statements.

Conditions compare two [integer expressions](34-integer-expressions.md) using
one of `==`, `!=`, `<`, `<=`, `>`, `>=`. Both sides support arithmetic and
parenthesized expressions. Chained comparisons and Boolean `and`/`or` are
not implemented. Every referenced field must be a declared `Int`, including
references in unreachable branches. Comparison does not subtract operands,
so comparing minimum and maximum signed integers cannot overflow by itself.
Arithmetic within either operand retains the usual checked overflow rules.
Condition limits include both operands: 65,536 bytes, 1,024 nodes and 128
levels, with the comparison itself counting as one node and one level.

Blocks can contain writes, calls, repetitions and other control statements.
Requirements, explicit reads, outputs, guarantees and trace declarations stay
outside blocks. They describe the action contract. `returns` still selects
result fields and does not exit an action early. Repeated calls accept
`the system repeats <action> <positive count> times`, optionally followed by
`sequentially`; invalid or missing counts are errors.

Only the chosen branch and executed loop iterations evaluate their bodies.
Calls share mutable state with their caller. The native backend currently
inlines stateful action calls and rejects recursive action-call graphs and early
action returns. [Integer functions](36-integer-functions.md) use separate call
frames and support recursion and early value returns.

## Core and STE

An action's ordered `control_step` edges point to `Control` nodes with
`control_kind` values `if`, `else`, `end_if`, `while`, `end_while`, `break`, or
`continue`. The edges carry explicit contiguous `step_index` values across
all writes, calls, repetitions and control statements in the action.

An `if` or `while` node has one `condition` edge to an `IntegerComparison`
node typed `Bool`. Its `comparison` attribute holds the operator;
`comparison_left` and `comparison_right` point to typed `IntegerExpression`
roots. Delimiters have no condition. The checker rejects malformed types,
operators, operands and misplaced control edges. Saved Core and rendered
specifications preserve executable block order. Imported field references
are qualified on both sides of comparisons.

Lowering generates labels and jumps using this additional STE template:

```text
If integer comparison left_data comparison_data right_data is true, go to label target_data.
```

All four operands name declared technical data. The two expressions use
validated postfix notation; the operator data holds one comparison symbol.
This instruction lowers to `BRANCH_INT`. The bytecode verifier checks both
expressions, the operator and the action-local jump target.

Each action invocation has an instruction budget of
`max(original_instruction_count * 1024, 1024)`. Instructions and labels
consume steps. Called actions receive separate budgets, and returning from
a call restores the caller's remaining budget. Native inlining preserves
these budgets. Exhaustion fails with an execution step-limit error; native
execution exits unsuccessfully without result stdout. This limits execution
and does not prove termination.

## Run and measure

```bash
cargo run -- ail-run bench/R007-sum-to-n/program.ail --action SumToN n.value=100
cargo run -- ail-ste bench/R007-sum-to-n/program.ail > /tmp/sum.ail-ste
cargo run -- ail-compile /tmp/sum.ail-ste --action SumToN \
  --target linux-x86_64-elf --out /tmp/ail-sum
/tmp/ail-sum n.value=100
```

The expected selected result is `out.total=5050` on both backends. A standalone
version with descriptive data names is [sum.ail-ste](../../examples/ste/sum.ail-ste).
The frozen
capability ladder passes **16/16** rungs, including maximum, absolute value
and sign, sum-to-n, GCD, Collatz, primality, recursive factorial and Fibonacci.
Collections, general
I/O, arbitrary-precision integers and broader Python compatibility remain
open; the ladder score does not establish Python replacement readiness.
