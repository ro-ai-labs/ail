# Executable value functions

Functions accept and return `Int`, `Float`, `Bool`, `Text`, `Bytes`, `List<Int>`, `List<Float>`, `List<Text>` and
`Json` through source, checked Core, STE, the VM and direct Linux x86-64 ELF.
Use them for reusable text, list and record transformations with isolated locals.

[Executable variants](59-executable-variants.md) also support concrete custom
types as parameters, locals and results, including recursive generic data.
Their full schema is checked at function boundaries; arguments borrow input and
successful custom returns charge one copy to the shared composite arena.

```text
Function: label.
The function needs:
- name: Text
The function produces:
- result: Text
The function stores:
- prefix: Text
- result: Text
The function executes:
- the function assigns "processed: " to local.prefix
- the function joins text local.prefix and local.name into local.result
- the function returns local.result

Action: LabelName.
When LabelName happens:
- the system calls label with name = data.name into output.text
- the system returns output.text
- the system records a trace event named Labelled
```

The caller's `data.name` and `output.text` must be declared `Text` fields.
Inside a function, every field uses `local.name` and must be a parameter or an
explicit local declaration. The output declaration describes the result type;
it does not create storage. In the example, `result` is also declared as a local.

## Calls, operations and control

A function has named parameters, at most one set of distinct declared locals,
and exactly one result. Calls use `name = expression` pairs separated by
semicolons, or `with none` for no arguments. Forward calls and mutual recursion
are supported. Parameter names determine argument matching and evaluation order.

`Int` arguments and returns can use checked integer expressions. Other types
require a bare field of the exact declared type. The caller assigns its
destination only after a successful return, including when an input and the
destination use the same field.

Functions support integer and [floating computation](45-float-computation.md), branches, loops, value calls, early
returns and explicit traces. Local literal, text, list, JSON conversion, lookup,
count, set and remove statements use the same forms as actions, with
`the function` in place of `the system`. For example:

```text
- the function assigns "/amount" to local.path
- the function gets JSON value at path local.path from local.record into local.amount
- the function calls double with n = local.amount into local.amount
- the function sets JSON value at path local.path in local.record to local.amount into local.result
- the function returns local.result
```

Functions cannot access global fields, perform file I/O, invoke stateful actions
or tools, or select CLI output. Keep those operations in an action caller.
Only parameters are initialized on entry; reading an uninitialized local fails.
Falling through without a return fails, and a return immediately ends that
invocation. Only explicit trace statements emit function traces.

Pure integer functions retain their existing fast integer protocol. A function
uses the general protocol if it declares a non-Int value, uses a local value
operation, or can call another general function. This classification propagates
through callers, including functions whose own signature is entirely Int.

## Values, limits and errors

Arguments are evaluated and validated in sorted parameter-name order, including
unused parameters, before checking call depth. Int expressions evaluate once.
Other arguments borrow immutable input transport without an arena charge:

| Type | Boundary validation | Successful return |
| --- | --- | --- |
| Int | Checked i64 expression | Canonical integer; no arena charge |
| Float | Strict bounded binary64 transport | Canonical scientific notation; no arena charge |
| Bool | Exactly `true` or `false` | Canonical Boolean; no arena charge |
| Text | Valid UTF-8, at most 1 MiB | One copy in the text arena |
| Bytes | Canonical hex transport, at most 1 MiB raw bytes | One canonical transport copy in the shared list/Json arena |
| List<Int>, List<Float>, List<Text> | Flat-list runtime grammar and limits | Canonical list copy in the list arena |
| Json | Complete JSON, including structure and canonical-size limits | Canonical JSON copy in the list arena |

[Floating lists](46-floating-lists.md) retain signed zero, infinity and NaN in
parameters and returns; their JSON conversion requires finite numbers.
List parameters allow input whose canonical form would exceed 1 MiB, so a
function can inspect or shrink it. Returning that oversized canonical list fails.
Json parameters always check canonical size as well as raw size. JSON number
spellings, member order and unselected duplicate keys retain existing semantics.

Each successful non-scalar return charges exactly one copy, even when forwarding
a parameter, literal or previously allocated result. Returned values survive
callee teardown, caller overwrites and later calls. Body operations retain their
existing allocation rules; literals remain free. The entire run shares a
**64 MiB text arena** and **64 MiB list/Json arena**. Empty Text costs one byte;
empty Bytes costs four.
A failed call preserves its caller destination; earlier allocations and explicit
traces from its body are not rolled back.

Text/list/Json retain their existing 1 MiB value bounds, 65,536 list items /
JSON values, and 64 JSON container depth limits. Bytes allows 1 MiB raw bytes
and 2,097,156 bytes of canonical transport. Each function allows **256 combined parameters
and locals**, **4,096 source steps**, and **64 active call frames**, shared with
integer callees. Each invocation has its own instruction fuel limit of
`max(instruction_count * 1024, 1024)`. Direct general bytecode permits 16,384
body instructions plus its signature marker.

Errors use the INT, FLOAT, TEXT, BYTES, LIST and JSON families. Bool boundary errors
use JSON/JSON-LIMIT. Depth and missing-return errors are
`AIL-RUNTIME-DEPTH` and `AIL-RUNTIME-RETURN`. Diagnostics omit operand contents.
Native failure produces a nonzero exit and no result stdout.

## Core, STE and direct entry

Core uses `Function(executable_value=true)`, ordered `ValueFunctionStep` nodes,
and typed owned local fields. `ValueFunctionCall` represents an action's general
call. Explicit operand edges carry roles and types; executable prose is not
stored as an unchecked attribute. Checks reject duplicate, missing, foreign or
orphan operands, mismatched metadata and noncanonical executable markers.
See the [Core schema](18-ail-core-schema.md).

```text
Start value function with parameters parameters_data, locals locals_data, and result type result_data.
Call value function target_data with arguments arguments_data into field destination_data.
Return type_data value from expression expression_data.
```

These templates lower to `VALUE_FUNCTION`, `CALL_VALUE` and `RETURN_VALUE`.
Typed signature maps contain sorted `name:Type` pairs separated by semicolons.
Calls retain named postfix arguments; non-Int arguments and returns are single
field references. The function result type is checked against its declaration.

Application and C interop profiles support these functions. Functions require
an action caller and are excluded from `--all-actions`; direct CLI, VM and
native function entry is rejected. Source and Core errors use
`AIL-FUNCTION-001`; bytecode and STE protocol errors use `AILBC011`.
Imports qualify function targets and caller globals, preserving local names.
Actual call operands imply secret reads and destination writes.

## Runnable file report

The [value-functions package](../../examples/value-functions) reads nested JSON,
calls a Json-to-Json record transform and an integer helper, then calls Text and
List<Text> helpers to build labels. It removes private fields, doubles amounts,
adds a Boolean flag and writes a report with count `3` and total `80`.
Other number tokens keep their exact spelling.

```bash
cargo run -- ail-run examples/value-functions --action Transform \
  --read-file=input=examples/value-functions/input.json \
  --write-file=report=/tmp/value-report.json

cargo run -- ail-compile examples/value-functions --action Transform \
  --target linux-x86_64-elf --out /tmp/value-report
/tmp/value-report \
  --read-file=input=examples/value-functions/input.json \
  --write-file=report=/tmp/value-report-native.json
```

The [exported STE](../../examples/ste/value-functions.ail-ste) runs through the
same commands. Source, saved Core and STE produce identical reports in the VM
and native executable.

After `cargo build`, run `python3 scripts/ail_value_function_differential.py` for
independent Python comparisons: 360 six-type cases, 100 local transformations,
recursion and exact allocation boundaries, and 50 composed file reports.
Float support is documented in the [floating-point guide](45-float-computation.md).
[JSON inspection](47-json-inspection.md) adds kind/keys operations on locals and
a recursive function that traverses objects with previously unknown field names.
Broader libraries and full Python replacement remain on the active objective.

The 2026-10-11 verification passed 868 Rust tests, strict Clippy and formatting,
the independent comparisons above and prior JSON/literal regression oracles.
The frozen capability ladder remains 16/16; the broader corpus has 33 VM/native
agreements, 30 unsupported native cases and zero divergences. The
[implementation record](../superpowers/plans/2026-10-11-value-functions.md)
includes review findings, fixes and verification evidence.

[Boolean predicates](48-boolean-predicates.md) add local Bool computations and
lazy Boolean controls. Declare a Bool local, compute the predicate into it,
then return or pass that field. All function arguments are still validated on
entry, even when a predicate would skip reading one. Boolean evaluation borrows
local values and charges neither result arena.

[Text transformations](49-text-transformations.md) add split, list join, replace
and trim on declared locals. Split produces `List<Text>`; list join consumes it.
Each operation charges only its result arena, and returning that result retains
the existing one-copy charge. [Tabular Report](../../examples/tabular-report)
uses reusable functions for CRLF normalization and column cleanup.

[List transformations](50-list-transformations.md) add typed local concat, slice,
reverse and stable sort. Bounds can reference local Int fields; all list operands
have one matching element type. [List Statistics](../../examples/list-statistics)
composes batch merging, median calculation and a highest-value window.

[Runtime recovery](51-runtime-recovery.md) adds local `try`/`recover`/`end try`
blocks with a declared Text error destination. Data errors propagate through
calls; return expressions remain protected until validation succeeds. A caught
error preserves earlier local values and successful arena charges. Int-signature
source functions with Text recovery locals use VALUE_FUNCTION; raw integer
functions propagate errors to caller handlers.

## Bytes parameters, locals and results

[Bytes](56-bytes.md) is also a supported primitive. All ten pure byte operations
work in recursive typed functions. Arguments borrow validated canonical
transport; successful returns copy once into the shared List/Json arena. There
is no implicit Text/Json conversion, and file effects remain action-only.

Functions can [raise explicit domain errors](57-explicit-errors.md), including
from a function whose reachable paths all raise. A raise-containing function
must return or raise on every reachable exit; a failed call leaves its result
destination unchanged. The [RLE library](58-run-length-codec.md) uses this to
validate binary formats before constructing output.
