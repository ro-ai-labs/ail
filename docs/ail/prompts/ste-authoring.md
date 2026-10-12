# Agent guide: executable AIL-STE

Use [the STE reference](../33-ste-intermediate.md) and its exact templates.
STE is a deterministic low-level programming input. A successful grammar check
does not establish that the program satisfies a user's intent.

1. For an existing application, preserve the checked specification and use
   `ail-ste <package>` to export the executable intermediate.
2. For direct STE authoring, begin with `examples/ste/counter.ail-ste`. Keep
   package metadata, grants, failures, and target support explicit.
3. Declare technical data before procedures. Use declared identifiers in the
   instruction templates. Do not insert prose into instruction lines.
4. Run `ail-check <file.ail-ste>` and correct every diagnostic. Do not treat
   ignored-looking or trace-only instructions as computation.
5. Run `ail-run <file.ail-ste> --action <name> key=value ...` on representative
   inputs and compare final state with the intended result.
6. Compile using `ail-compile <file.ail-ste> --action <name> --target
   linux-x86_64-elf --out <binary> --artifact-dir <bundle>`. Run the binary on
   the same inputs. Keep actual outputs as evidence.

For changes to application contracts, revise the specification or Core through
the existing checked workflow. Editing a STE file does not update its source
specification or carry forward proof of Core-level checks. Broader types,
libraries and effects remain necessary for general Python replacement; the
current computation and regular-file operations are described below.

For numeric computation, use `computes` in the source specification and export
STE. The expression technical data uses postfix `i:<integer>`, `f:<field>`, and
`+ - * / %` tokens. Select result fields explicitly. Test signed values,
noncanonical input such as `007`, overflow, and zero divisors. Arithmetic uses
checked signed 64-bit integers with division toward zero, not Python integers.
See [the integer guide](../34-integer-expressions.md) for complete semantics.

For fractions, declare `Float` and use `computes floating` with explicit
integer/Float conversions. Arithmetic uses binary64, including signed zero,
infinity and NaN. JSON boundaries require finite values. Test rounding,
nonfinite results and conversions near integer bounds. See
[floating computation](../45-float-computation.md).

For conditional or repeated computation, use source `if`/`otherwise`/`end if`
and `while`/`end while` blocks, with `break` and `continue` for loop control.
Direct STE uses integer-comparison branches plus labels and jumps. Keep output
and trace declarations outside source blocks. Test zero iterations, both branch
outcomes, nested loops and termination limits. See [control flow](../35-control-flow.md).
For mixed scalar predicates, use `if boolean` / `while boolean` and typed
comparisons combined with `not`, `and` and `or`. Evaluation short-circuits;
static type and secret-access checks still cover skipped operands. See
[Boolean predicates](../48-boolean-predicates.md).

For recoverable bad input, surround executable operations with `try:`, one
`recover JSON or JSON-PATH into data.error:` clause, and `end try`. Declare the
error destination as Text; functions use local Text storage. `any` matches the
documented recoverable error families, including integer overflow and division by
zero, but excludes resource limits, execution limits, IO and capability errors.
Handlers receive a stable `AIL-RUNTIME-<CATEGORY>` string. Previous successful
writes and arena charges remain; there is no implicit rollback or retry. Test
normal paths, matching/nonmatching errors, nested calls and handler failures.
See [runtime recovery](../51-runtime-recovery.md) and
[Recovering JSON Lines](../../../examples/recovering-json-lines).

For value-returning computations, use explicit executable function declarations:
`The function needs:`, `The function produces:`, optional `The function stores:`,
and `The function executes:`. Reference parameters/locals as `local.name` and
call with named arguments into a declared result field. Legacy function trace
instructions do not compute values. Test multiple recursive calls in one frame,
missing locals, early return and depth exhaustion. See
[integer functions](../36-integer-functions.md) and
[value functions](../44-value-functions.md). Parameters, locals and the result
may be Int, Float, Bool, Text, List<Int>, List<Float>, List<Text> or Json. Non-Int arguments/returns
require bare typed fields. For local literal/text/list/JSON operations, use
`the function` instead of `the system`; all storage must be declared `local.name`.
Keep file access and global effects in actions. Non-scalar returns charge one
copy in the shared arena, so include return copies when testing allocation
limits. Validate even unused arguments and test alias/lifetime behavior across
repeated calls. See `examples/value-functions` for a reusable file report.
Use an action wrapper as the
entry point; `--all-actions` retains helper functions inside each executable.

For text processing, declare `Text` fields and use the source forms `joins text`,
`counts characters in`, `slices text ... from ... to ...`, and `finds text ... in`.
Count/find destinations are `Int`; slice bounds are integer expressions.
Positions count Unicode scalar values, with negative/clamped slice bounds and
-1 for search misses. Test empty and missing text separately, multibyte text,
combining marks, destination aliases and allocation limits. Standard JSON
surrogate pairs are accepted when transporting non-BMP characters. See
[text operations](../37-text-operations.md) and `examples/unicode-text`.

For line/column processing, use `splits text ... by ...`, `joins list ... with ...`,
`replaces text ... in ... with ...` and `trims text`. Split produces List<Text>
and preserves empty parts; an empty separator fails. Replace with an empty needle
inserts at Unicode scalar boundaries. Trim uses a fixed whitespace set. Test
boundaries and escaping, not only ordinary ASCII. See
[text transformations](../49-text-transformations.md) and
[Tabular Report](../../../examples/tabular-report).

For collections, declare `List<Int>`, `List<Float>` or `List<Text>` fields. Create
an empty list, then append values; or supply a flat array as runtime input.
Use count/get inside a loop for map/reduce-style computations. Indices support
negative offsets and fail outside the list. Append/set produce new values, so
copied lists retain their previous contents. Lists remain bounded and each
operation parses its source; account for copy cost and the shared result budget.
Test malformed JSON, element types, empty lists, negative indices and aliased
destinations. See [typed lists](../38-typed-lists.md), `examples/typed-lists` and
`examples/ste/typed-lists.ail-ste` for executable syntax. Floating lists also
accept scalar Float runtime tokens, including nonfinite values; JSON conversions
require finite numbers. See [floating lists](../46-floating-lists.md).

For whole-list work, use `concatenates list`, `slices list`, `reverses list` and
`sorts list ... in ascending order` (or descending). Inputs/results keep one
element type. Slices clamp bounds and evaluate both expressions; sorts are stable
in both directions. Float NaNs are always last, and signed zeros compare equal.
These operations parse each input once and allocate one canonical list result.
Use them to avoid repeated append/set copying for these transformations. See
[list transformations](../50-list-transformations.md) and
[List Statistics](../../../examples/list-statistics).

To construct values in a program, use `the system assigns <JSON literal> to
<field>`. Declare the destination as `Int`, `Float`, `Bool`, `Text`, `List<Int>`,
`List<Float>`, `List<Text>` or `Json` and use matching JSON syntax. Float literals
must round to finite values. Quote and escape text; do not
split strings containing spaces into prose assignments. Values remain opaque
data through Core, STE and imports. Static assignments consume no runtime result
budget, while later text/list operations retain their charges. See
[typed literals](../39-typed-literals.md) and `examples/typed-constants`.

For text file I/O, use named resources. Declare the data field as Text, then use
`the system reads text from file resource input into data.body` or
`the system writes text data.body to file resource output`. Bind paths only at
execution with `--read-file=input=path` / `--write-file=output=path`; neither
package metadata nor ordinary state values grant these permissions. Native ELF
uses the same flags. Imported resources gain the alias prefix. Reads and writes
accept regular UTF-8 files up to 1 MiB. Completed writes survive later failures.
See [file resources](../40-file-resources.md) and `examples/text-files`.

For shell pipelines, use [standard streams](../52-standard-streams.md): read Text
from standard input and write Text to standard output/error inside actions. Pass
`--stdio` to `ail-run`, `ail-vm` or the emitted binary. It grants all three streams
and suppresses automatic reports; compilation never grants them. Write explicit
newlines where needed and keep application diagnostics on stderr. Input is bounded
to 1 MiB and read through EOF; do not assume interactive line reads. Test exact
stdout/stderr bytes, empty input, malformed UTF-8, absent grants, broken pipes and
late failures that retain an output prefix. See
[JSON-lines filter](../../../examples/json-lines-filter). Library integrations
can independently supply borrowed endpoints through `run_ail_bytecode_action_with_io`.

For structured file data, declare a Text field for raw input and a typed field
for its value. `the system parses JSON text data.raw into data.values` validates
JSON against the destination declaration. `the system formats data.values as
JSON text into output.text` serializes the value for a file write. Current
conversion types are Int, Float, Bool, Text, List<Int>, List<Float>, List<Text>
and Json, with 1 MiB
input/output bounds. Use the [JSON batch example](../../../examples/json-batch)
for a complete read/parse/function/loop/format/write workflow and see
[JSON conversion](../41-json-conversion.md) for exact error and budget semantics.

Use `Json` for a nested record or an API payload. Extract a value with
`the system gets JSON value at path data.path from data.document into output.value`;
the destination type selects decoding. Paths are Text containing JSON Pointer
strings such as `/records/0/name`. Count a Json array or object with
`the system counts entries in JSON data.document into output.count`. Test missing
and duplicate keys, escaped path tokens and typed extraction failures. Json
preserves decimal/exponent/large-integer tokens but does not add arithmetic for
them. See [recursive JSON](../42-recursive-json.md) and the
[record-processing example](../../../examples/json-records).

For records with unknown structure, use JSON kind inspection and ordered key
enumeration, then traverse with typed JSON Pointer lookups. See
[JSON inspection](../47-json-inspection.md) and
[its recursive example](../../../examples/json-inspection).

Build or update a Json report with
`the system sets JSON value at path data.path in data.document to data.value into output.document`.
Remove a member with
`the system removes JSON value at path data.path from data.document into output.document`.
Set converts the value's declared type; use `/-` to append to an existing array.
Parents must exist. Test duplicate-key ambiguity, missing parents and final
depth/count/size limits, including when replacing or removing large subtrees.
Only the final result consumes the list budget. See
[JSON mutation](../43-json-mutation.md) and the
[report transformation example](../../../examples/json-transform).

For external tools, use [process resources](../53-process-resources.md): declare
List<Text> arguments, Text input/captures, and Int timeout/status fields. A caller
passes `--run-process=name=/absolute/executable`; source/state never grant access.
Arguments are separate strings, with no implicit shell. Handle nonzero status as
ordinary output and use `recover PROCESS or TIMEOUT` for launch/invocation errors
and deadlines. The runtime cleans up children before recovery. Test early stdin
close, both large outputs, invalid output UTF-8 and timeouts, not just exit zero.
The [tool coordinator](../../../examples/tool-coordinator) runs a compiled AIL
worker and demonstrates a usable tool loop.

For discovered files, use [directory resources](../54-directory-resources.md).
Declare relative paths as Text and name lists as List<Text>. Request separate
caller `--read-directory` and `--write-directory` grants with absolute roots;
source and mutable state carry no authority. List names, loop over them, and use
computed Text paths for reads/writes. Listing includes directories and symlinks;
recover PATH/TEXT per entry when the workload should continue. Use safe bounded
output names, for example numeric indices, since suffixing a maximum-length
basename can fail. Test symlinks, traversal, Unicode/control names, invalid UTF-8,
missing roots, write failures and result budgets. The
[directory reporter](../../../examples/directory-report) demonstrates this flow.

Use the exact [Bytes templates](../56-bytes.md#core-and-ste) for byte operations
and binary files; preserve Bytes/Text distinctions and explicit UTF-8/hex
conversions. Indices and replacement values are checked Int expressions.
Binary file operations require the same caller grants as Text files.

For explicit domain rejection, emit `Raise error {category}.` with exactly one
`category` technical-data operand drawn from the existing catchable categories.
Never substitute `any`, a compound category, a variable, a message or a fatal
category. Existing recovery scopes catch this error. See
[explicit errors](../57-explicit-errors.md).

For [executable variants](../59-executable-variants.md), retain the closed
type_schemas metadata and use VALUE_CONSTRUCT, VALUE_TEST and VALUE_GET's exact
templates. Do not encode constructor arguments as unchecked prose in Core.
Preserve nominal type names, import qualification, sorted named arguments and
full-payload validation. Generic data declarations do not imply generic functions.
