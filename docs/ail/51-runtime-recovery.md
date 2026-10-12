# Recover from runtime errors

Actions and value functions can recover from malformed data and failed
computations through checked Core, STE, the VM and native Linux x86-64 ELF.
Actions can also recover from process invocation failures, timeouts and
directory path errors.
Recovery requires Application or C interop.

```text
- try:
- the system parses JSON text data.row into data.record
- the system gets JSON value at path data.path from data.record into data.value
- recover JSON or JSON-PATH into data.error:
- the system computes data.rejected as data.rejected + 1
- end try
```

Declare the recovery destination as Text. Exactly one `recover` clause belongs
to each `try`, followed by `end try`. Functions use declared `local.*` fields and
`the function` operations. Int-returning functions that declare Text recovery
storage use the existing typed value-function backend.

## Matching and control flow

List one or more distinct uppercase categories separated by ` or `, or use `any`
to match all categories in this table:

| Category | Typical cause |
| --- | --- |
| INT | Missing or malformed integer input |
| OVERFLOW | Checked integer arithmetic overflow |
| DIVZERO | Integer or floating division/remainder by zero |
| FLOAT | Invalid floating input or integer conversion |
| BOOL | Missing or malformed Boolean expression input |
| TEXT | Invalid text input or empty split separator |
| LIST | Missing or malformed typed list |
| INDEX | List or byte index outside its bounds |
| JSON | Malformed JSON, wrong typed value or invalid conversion |
| JSON-PATH | Invalid, missing or ambiguous JSON Pointer path |
| PROCESS | Invalid process invocation or child setup/exec failure |
| TIMEOUT | Process communication deadline expired |
| PATH | Invalid relative directory path, rejected file kind or path access failure |
| BYTES | Malformed Bytes transport/hex or replacement byte outside 0..255 |
| VALUE | Missing or malformed custom variant data, bad payloads or wrong-variant extraction |

Existing operation-specific categories still apply. For example, a malformed
Bool function argument reports JSON, while a malformed Boolean expression operand
reports BOOL. Floating division or modulo by either signed zero reports DIVZERO,
as specified by the existing Float operations.

On success, execution skips the handler and leaves its destination unchanged.
On error, it abandons the rest of the protected body and enters the nearest
matching handler. The destination receives a stable string such as
`AIL-RUNTIME-DIVZERO`, without diagnostic text or operand contents. The value
remains available after `end try` and requires no arena allocation.

Handlers nest and errors propagate through calls. An error raised while executing
a handler can reach an outer protected scope. A return expression is evaluated
before leaving its function's handlers, so its errors can still be recovered
locally. Successful returns, break, continue and jumps remove handlers for the
scopes they leave. Entering a fresh `try` installs a fresh handler.

This follows the general termination model of
[Python's try statement](https://docs.python.org/3/reference/compound_stmts.html#the-try-statement):
the handler continues outside the failed operation. AIL currently uses fixed
categories and a single handler clause; it has no exception objects, implicit
retry, user-defined error categories or payloads, or finally block.

## State, effects and limits

Completed field writes, trace events and file effects remain visible. This
includes completed work in an action whose later error reaches a caller's
handler. Recovery does not roll back a block. Atomic failed operations preserve
their destination, and a failed function call does not replace its result field.

Previously successful Text and List/Json allocations remain charged. Catching an
error does not reset instruction fuel or function-depth accounting. Temporary
native mappings created by the failed operation are released before recovery;
surviving values remain in their existing result arenas.

`any` catches the fifteen listed categories. PATH applies to
[directory resources](54-directory-resources.md). PROCESS and TIMEOUT apply to
[process resources](53-process-resources.md), after child cleanup. Allocation and
value-size limits remain fatal. The directory relative-path length bound is an
explicit exception: it reports recoverable PATH.
Execution/depth exhaustion, capability/file-grant/IO failures, missing returns
and other unclassified errors remain fatal. Declared rule failures with a failed
action status keep their existing behavior. Catching JSON does not catch
JSON-LIMIT. Unhandled errors retain their existing diagnostic and exit behavior.

The VM adds `recovered <CATEGORY> into <field>` to its trace. Caught native errors
do not print terminal error diagnostics; ordinary native traces remain available.

## Core and STE

Action Control nodes add `control_kind=try|recover|end_try`. A recover node stores
canonical `recovery_errors` and one attribute-free `recovery_destination` edge to
a genuine Text field. Function controls retain owned ValueFunctionStep nodes and
typed local field operands. Exact attributes, field ownership, ordered scopes and
semantic write permissions are checked even without descriptive access edges.
Imports qualify the destination field.

| Opcode | Operands | STE template |
| --- | --- | --- |
| TRY_BEGIN | errors,key | `Begin recovery for errors {errors} into field {key}.` |
| RECOVER | none | `Run the recovery handler.` |
| TRY_END | none | `End recovery.` |

The `errors` technical data is `any` or a sorted, unique, semicolon-separated
list such as `DIVZERO;INT`. Normal execution of RECOVER skips its handler body;
error dispatch enters immediately after that delimiter. Empty bodies are allowed.
Each original action or function is limited to 64 nested recovery scopes, within
the existing source control limits. Calls retain their callers' active handlers;
native inlining does not impose an additional limit on the combined chain.

The shared verifier tracks scope identity and whether an instruction belongs to
a protected or recovery body. Ordinary jumps may leave regions but cannot enter
their interiors or cross between sibling regions. Jumps to TRY_BEGIN are allowed
because that instruction installs the handler. Raw INTEGER_FUNCTION procedures
cannot install handlers; caller recovery still covers their propagated errors.

Protocol errors use AILBC019. Source/Core errors use AIL-CONTROL-001 and existing
typed-function diagnostics. Direct STE receives protocol verification; it does
not reconstruct the source specification's Core permissions or guarantees.

## Recovering JSON-lines example

[Recovering JSON Lines](../../examples/recovering-json-lines) reads numeric rows,
calls a reusable parse/divide function, counts rejected rows by category, sorts
accepted values and writes one actual JSON report. Blank lines are skipped.

```bash
cargo run -- ail-run examples/recovering-json-lines --action Analyze \
  --read-file=input=examples/recovering-json-lines/input.jsonl \
  --write-file=report=/tmp/recovered-report.json
```

See its [exported STE](../../examples/ste/recovering-json-lines.ail-ste) and
[implementation record](../superpowers/plans/2026-10-11-runtime-recovery.md).
After `cargo build`, `python3 scripts/ail_recovery_differential.py` exercises
independent recovery expectations and actual report files.

[Bytes](56-bytes.md#limits-and-errors) appends catchable BYTES at index 13.
That addition expanded the `any` mask to 16383 while retaining earlier indices.
BYTES-LIMIT remains fatal even inside `recover any`.

[Executable variants](59-executable-variants.md) append VALUE at index 14;
`any` now has mask 32767. VALUE-LIMIT and VALUE-IO remain fatal, including inside
`recover any`. Existing category indices retain their values.

Actions and executable functions can now [explicitly raise](57-explicit-errors.md)
one catchable category with `the system raises error CATEGORY` or
`the function raises error CATEGORY`. `RAISE` has the single operand `category`
and STE template `Raise error {category}.`. The existing handler-selection and
unwind rules apply; no new category or mask is introduced.
