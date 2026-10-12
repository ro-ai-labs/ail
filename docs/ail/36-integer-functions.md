# Executable integer functions

This page describes the integer subset. [Value functions](44-value-functions.md)
extend the same source syntax to Boolean, Text, lists and JSON and local value
operations. Pure integer call graphs retain this protocol and implementation.

AIL supports named integer arguments, mutable locals, early value returns,
forward calls and mutual recursion through Core, STE, the VM and native Linux
x86-64 ELF. Each invocation owns its local storage. Multiple recursive calls
in one frame can retain separate results, as required by Fibonacci.

## Source

Use existing function input/output declarations followed by
`The function executes:`. Declare additional locals with `The function stores:`:

```text
The application Calculator manages integer results.

A N has:
- value: Int

An Out has:
- result: Int

Function: factorial.

The function needs:
- n: Int

The function produces:
- result: Int

The function stores:
- partial: Int

The function executes:
- if local.n <= 1:
- the function returns 1
- end if
- the function calls factorial with n = local.n - 1 into local.partial
- the function returns local.n * local.partial

Action: Factorial.

When Factorial happens:
- the system calls factorial with n = n.value into out.result
- the system returns out.result
- the system records a trace event named Calculated
```

Inputs, locals and the single output must have type `Int`. Input and local names
are distinct ASCII identifiers. Reference them as `local.name`. They are mutable,
but only parameters are initialized on entry. Reading an uninitialized local
fails. Each function allows at most 256 parameters and locals in total and
4,096 source body statements.

Bodies accept `the function computes local.name as <expression>`, value calls,
value returns, existing control-flow bullets and
`the function records a trace event named <event>`. Calls bind named arguments
with `name = expression`, separated by semicolons. For example,
`with b = local.n + 1; a = local.n` binds two parameters. Each required argument
must occur exactly once. Use `with none` for zero arguments. Expressions evaluate
in sorted parameter-name order within the caller before entering the callee.
Changing a callee parameter does not change caller storage.

A function return immediately exits the current invocation, including enclosing
loops. Falling off the body fails with `AIL-RUNTIME-RETURN`; there is no implicit
zero result. The declared output name does not automatically create a local.
Functions access only their parameters and locals and cannot call stateful
actions. An action can pass global expressions and assign the returned integer
to a declared global `Int` field, only after successful return. Action `returns`
still selects output fields and does not exit early.

The older `When <function> runs:` model remains descriptive. Its
`FUNCTION_CALL`/`FUNCTION_RETURN` instructions record traces and cannot be
executable integer-call targets. Mixing that body with an executable section or
local declarations is rejected, including unrecognized legacy prose.

## Execution and limits

[Checked i64 arithmetic](34-integer-expressions.md) applies to arguments, locals
and results. At most 64 executable function frames may be active, shared with
general value function callers and independently of the existing stateful
action-call depth. Excess depth fails with
`AIL-RUNTIME-DEPTH`. Each invocation receives
`max(bytecode_instruction_count * 1024, 1024)` steps; signature, label and return
instructions count. Callees receive fresh budgets and preserve caller fuel.
These are runtime limits, not a termination proof or a total budget for a
recursive call tree.

The VM uses indexed integer local slots. Native compilation uses real x86-64
machine calls and separate stack frames. Globals remain outside these frames.
Only explicit executed function trace statements add trace events; recursion
does not accumulate an automatic trace per instruction or invocation.

Both backends report checked overflow, zero divisors, missing locals, missing
returns, depth exhaustion and step exhaustion. Native failures use a nonzero
exit code, stderr diagnostics and no result stdout.

## Core and STE

An executable `Function` has `executable_integer=true`, ordered `function_step`
edges to `FunctionStep` nodes, and `function_parameter`/`function_local` edges
to owned `Field : Int` nodes. Fields carry `function_owner` and `local_name`.
Compute/return steps reference typed `IntegerExpression` trees; conditions use
`IntegerComparison : Bool`. Calls reference their target `Function`, destination
and named argument roots. An action's `integer_call_step` points to `ValueCall`
and shares its contiguous execution-step indices.

The checker rejects cross-function local references, invalid ownership/types,
wrong arguments, malformed ordering and duplicate operand edges. Source
profiles that lower executable functions are `Application` and `C interop`;
other profiles reject them. Saved Core preserves executable edges for checking
and rendering. Imports qualify function targets while retaining local names.

STE adds these templates, with declared technical-data identifiers as operands:

```text
Start integer function with parameters parameters_data.
Call integer function target_data with arguments arguments_data into field destination_data.
Return integer expression expression_data.
```

`INTEGER_FUNCTION` must begin its procedure. Its parameter data contains sorted
unique names separated by semicolons, or an empty string. `CALL_INT` arguments
encode named postfix expressions, such as `a=i:5;b=f:local.n i:1 +`. `RETURN_INT`
evaluates its postfix expression and returns the integer. The verifier checks
signatures, arity, body opcodes, return placement, local references and exact
instruction operands.

Functions require an action caller and cannot be root CLI actions.
`--all-actions` emits action entry points with their function dependencies;
bytecode and STE artifacts retain all procedures. Wasm contract trace reports
follow integer callees; executable Wasm remains unsupported.

## Run

```bash
cargo run -- ail-run bench/R005-factorial/program.ail --action Factorial n.value=20
cargo run -- ail-run bench/R006-fibonacci/program.ail --action Fibonacci n.value=30
cargo run -- ail-compile bench/R005-factorial/program.ail --all-actions \
  --target linux-x86_64-elf --artifact-dir /tmp/factorial-bundle
/tmp/factorial-bundle/target-Factorial.elf n.value=20
```

Factorial returns `out.result=2432902008176640000`; Fibonacci returns
`out.result=832040`. All **16/16** frozen computation rungs pass their declared
VM/native lanes with unchanged case/oracle fingerprints. This is a scalar
computation milestone. [Unicode text operations](37-text-operations.md) also
execute through both backends, as do [typed lists](38-typed-lists.md),
[file resources](40-file-resources.md) and [recursive Json](42-recursive-json.md).
[Typed value functions](44-value-functions.md) extend parameters, locals and
results to Text, Bool, Float, typed lists and Json, with
[runtime recovery](51-runtime-recovery.md) for data errors. Arbitrary precision
and broader Python compatibility remain open.
