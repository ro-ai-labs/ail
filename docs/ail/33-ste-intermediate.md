# AIL-STE executable intermediate

Status: implemented programming profile, version `AIL-STE 1`.

## Position in the compiler

```text
AIL-Spec -> checked AIL-Core -> AIL-STE -> verified AIL bytecode -> VM / ELF
```

`compile_ail_core_bytecode` constructs STE and parses it. Native compilation,
package execution, saved Core compilation, and agent builds call this path.
The STE file contains the instructions used to produce bytecode. It is also
accepted as a standalone low-level source format for agents and humans.

Core remains the contract-checking boundary for specifications. Direct STE input
receives grammar and bytecode checks, just like direct bytecode input; it does
not reconstruct a Core graph or prove the original specification's permissions,
types, or guarantees. Existing target restrictions are retained in metadata.

## Relationship to ASD-STE100

The profile is based on procedural writing in
[ASD-STE100 Issue 9](https://www.asd-ste100.org/assets/files/ASD-STE100_ISSUE9.pdf).
It uses fixed imperative templates with one operation per sentence and at most
20 words per template. Conditional jumps put the condition first. Technical
data are separate declarations, referenced by identifiers in instructions.

The parser enforces these templates, exact word order, and known operands.
It is not a general STE writing checker. The full dictionary, technical-name
approval process, and all writing rules have not been implemented or audited.
This profile must not be described as certified or fully conforming ASD-STE100.

## Commands

```bash
cargo run -- ail-ste bench/R000-increment/program.ail > /tmp/counter.ail-ste
cargo run -- ail-ste --core-file /tmp/checked.ail-core.txt
cargo run -- ail-check /tmp/counter.ail-ste
cargo run -- ail-lower /tmp/counter.ail-ste --artifact-dir /tmp/counter-bundle
cargo run -- ail-run /tmp/counter.ail-ste --action IncrementCounter counter.value=41
cargo run -- ail-vm /tmp/counter.ail-ste --action IncrementCounter counter.value=41
cargo run -- ail-compile /tmp/counter.ail-ste --action IncrementCounter \
  --target linux-x86_64-elf --out /tmp/counter --artifact-dir /tmp/counter-native
/tmp/counter counter.value=41
```

`ail-ste` accepts `--spec-file` for a package override and `--artifact-dir` for
STE and bytecode export. Re-exporting a STE file canonicalizes its data names
and declaration order. The parser requires no LLM or external service.

Standard lower/build/compile bundles save `artifact.ail-ste`, its fingerprint,
and `manifest.ail-ste.txt`. This companion manifest binds STE and bytecode
fingerprints and explicitly identifies the limited conformance claim. Existing
manifests remain present. A hash binds content; it does not prove correctness.

## File grammar

The first two lines are mandatory:

```text
AIL-STE 1
Metadata {"package":"counter","version":"0.1.0","profile":"Application","capability_grants":[],"target_support":{},"external_bindings":[],"failures":[]}
```

Metadata uses the corresponding bytecode fields and their schemas. `actions`
and `kind` are forbidden. Unknown or discarded fields and duplicate records are
rejected. Legacy metadata may omit `external_bindings`; that absence is preserved.

Technical data declarations precede all procedures:

```text
Data counter = "counter.value"
Data amount = "7"
Data description = "Add seven to the counter."
Data action = "IncrementCounter"
Procedure action.
Record the start of action action.
Add amount to field counter.
Attribute text = description
Return success.
End procedure.
```

Identifiers start with an ASCII letter or underscore, followed by ASCII letters,
digits, or underscores. Values are JSON strings; use JSON escapes for quotes,
backslashes, and control characters. Unicode text is permitted in values.
Each instruction references previously declared data. Instruction words are
case-sensitive. Leading/trailing whitespace and blank lines are accepted after
the metadata header; comments and unrecognized prose are rejected.

`Attribute operand = identifier` supplies a required operand omitted from the
instruction sentence. For `ADD_INT_FIELD`, `READ_FIELD`, `SET_FIELD`, and
`COPY_FIELD`, the `text` attribute preserves source descriptions. Other operands
are in their sentence templates. Duplicate or unknown attributes are errors.

Procedure names resolve through data declarations, must be nonempty and unique,
and require `End procedure.`. Procedures must have at least one instruction.
Bytecode verification checks operands, integer deltas, repeat counts, labels,
and action targets. Syntax errors use `AIL-STE-001 line N`; bytecode errors are
reported with that prefix at the final validation line.

## Execution limits

STE inherits the current VM and native backend semantics. A template beginning
with `Record` often represents a trace or descriptive contract, not enforcement
of the text it records. In particular, recording function branches, calls, and
results does not implement general functions. Native target support remains
limited to the existing native instruction subset.

The computation benchmark still determines language capability. Checked signed
64-bit integer expressions now execute through STE; see [integer computation](34-integer-expressions.md).
Source branches and loops also execute through STE; see [control flow](35-control-flow.md).
Recursive [integer functions](36-integer-functions.md) execute with isolated
call frames; [value functions](44-value-functions.md) extend them to Float, Bool, Text,
lists and Json with typed local operations. [Unicode text operations](37-text-operations.md) join, count, slice
and search scalar-indexed text. [Typed lists](38-typed-lists.md) support immutable
integer, [floating](46-floating-lists.md) and text collections. Named UTF-8 file reads/writes use explicit
caller bindings. [Standard streams](52-standard-streams.md) add bounded stdin and
exact application stdout/stderr under an explicit runtime grant. Broader value
types, libraries and Python compatibility remain on the active language-development objective.
See [floating-point computation](45-float-computation.md) for binary64 expressions,
comparisons, conversions and numeric JSON boundaries.
[JSON inspection](47-json-inspection.md) adds value kinds and ordered object keys
for recursive traversal of records with unknown fields.

## Instruction templates

Replace each `{operand}` with a declared technical-data identifier. The opcode
column explains the existing semantics; opcodes are not valid STE instructions.

| Bytecode operation | STE instruction |
| --- | --- |
| `STDIN_READ_TEXT` | `Read text from standard input into field {key}.` |
| `STDOUT_WRITE_TEXT` | `Write text from field {source} to standard output.` |
| `STDERR_WRITE_TEXT` | `Write text from field {source} to standard error.` |
| `TRY_BEGIN` | `Begin recovery for errors {errors} into field {key}.` |
| `RECOVER` | `Run the recovery handler.` |
| `TRY_END` | `End recovery.` |
| `RAISE` | `Raise error {category}.` |
| `ACTION_BEGIN` | `Record the start of action {action}.` |
| `LABEL` | `Mark label {name}.` |
| `BRANCH_FIELD_EQUALS` | `If field {key} equals {value}, go to label {label}.` |
| `BRANCH_INT` | `If integer comparison {left} {comparison} {right} is true, go to label {label}.` |
| `BRANCH_FLOAT` | `If floating comparison {left} {comparison} {right} is true, go to label {label}.` |
| `JUMP` | `Go to label {label}.` |
| `CALL_ACTION` | `Do action {target}.` |
| `REPEAT_ACTION` | `Do action {target} {count} times.` |
| `ADD_INT_FIELD` | `Add {delta} to field {key}.` |
| `EVAL_INT` | `Evaluate integer expression {expression} into field {key}.` |
| `EVAL_BOOL` | `Compute Boolean expression {expression} into field {key}.` |
| `BRANCH_BOOL` | `If Boolean expression {expression} is true, go to label {label}.` |
| `EVAL_FLOAT` | `Compute floating expression {expression} into field {key}.` |
| `INT_TO_FLOAT` | `Convert integer field {source} into floating field {key}.` |
| `FLOAT_TO_INT` | `Convert floating field {source} into integer field {key}.` |
| `OUTPUT_FIELD` | `Select output field {key}.` |
| `JSON_PARSE` | `Parse JSON text {source} as type {type} into field {key}.` |
| `JSON_FORMAT` | `Format {type} field {source} as JSON text into field {key}.` |
| `JSON_GET` | `Get {type} value at JSON path {path} from field {source} into field {key}.` |
| `JSON_SET` | `Set {type} value {value} at JSON path {path} in field {source} into field {key}.` |
| `JSON_REMOVE` | `Remove value at JSON path {path} from field {source} into field {key}.` |
| `JSON_COUNT` | `Count entries in JSON field {source} into field {key}.` |
| `JSON_KIND` | `Get JSON kind from field {source} into field {key}.` |
| `JSON_KEYS` | `Get JSON keys from field {source} into field {key}.` |
| `FILE_READ_TEXT` | `Read text from file resource {resource} into field {key}.` |
| `FILE_WRITE_TEXT` | `Write text from field {source} to file resource {resource}.` |
| `ASSIGN_LITERAL` | `Assign literal {value} of type {type} to field {key}.` |
| `TEXT_JOIN` | `Join text {left} and {right} into field {key}.` |
| `TEXT_LENGTH` | `Count characters in text {source} into field {key}.` |
| `TEXT_SLICE` | `Slice text {source} from integer {start} to integer {end} into field {key}.` |
| `TEXT_FIND` | `Find text {needle} in text {source} into field {key}.` |
| `TEXT_SPLIT` | `Split text in field {source} by field {separator} into field {key}.` |
| `TEXT_JOIN_LIST` | `Join text list in field {source} with field {separator} into field {key}.` |
| `TEXT_REPLACE` | `Replace text from field {needle} in field {source} with field {replacement} into field {key}.` |
| `TEXT_TRIM` | `Trim whitespace from field {source} into field {key}.` |
| `LIST_CONCAT` | `Concatenate {type} list {source} and list {other} into field {key}.` |
| `LIST_SLICE` | `Slice {type} list {source} from integer {start} to integer {end} into field {key}.` |
| `LIST_REVERSE` | `Reverse {type} list {source} into field {key}.` |
| `LIST_SORT` | `Sort {type} list {source} in {order} order into field {key}.` |
| `LIST_EMPTY` | `Create empty list of type {type} into field {key}.` |
| `LIST_LENGTH` | `Count items in {type} list {source} into field {key}.` |
| `LIST_GET` | `Get item {index} from {type} list {source} into field {key}.` |
| `LIST_APPEND` | `Append value {value} to {type} list {source} into field {key}.` |
| `LIST_SET` | `Set item {index} in {type} list {source} to value {value} into field {key}.` |
| `REQUIRE_EXISTS` | `Check that field {key} exists for rule {rule} with failure {failure}.` |
| `REQUIRE_FIELD_NOT_EQUALS` | `Check that field {key} differs from {value} for rule {rule} with failure {failure}.` |
| `REQUIRE_FIELD_IN` | `Check that field {key} is in list {values} for rule {rule} with failure {failure}.` |
| `REQUIRE_FIELD_AFTER` | `Check that field {source} is after field {key} for rule {rule} with failure {failure}.` |
| `OBSERVE_RULE` | `Record rule {rule}.` |
| `READ_FIELD` | `Record field read {key}.` |
| `READ_EFFECT` | `Record read effect {text}.` |
| `SET_FIELD` | `Set field {key} to {value}.` |
| `COPY_FIELD` | `Copy field {source} to field {key}.` |
| `WRITE_FIELD` | `Record field write {key} with description {text}.` |
| `EFFECT` | `Record effect {text}.` |
| `FUNCTION_BEGIN` | `Record the start of function {function} with description {label}.` |
| `FUNCTION_INPUT` | `Record function input {name} with type {type}.` |
| `FUNCTION_OUTPUT` | `Record function output {name} with type {type}.` |
| `FUNCTION_BRANCH` | `Record function condition {condition}.` |
| `FUNCTION_CALL` | `Record function call {target} with description {text}.` |
| `FUNCTION_RETURN` | `Record function result {value}.` |
| `INTEGER_FUNCTION` | `Start integer function with parameters {parameters}.` |
| `CALL_INT` | `Call integer function {target} with arguments {arguments} into field {key}.` |
| `RETURN_INT` | `Return integer expression {expression}.` |
| `VALUE_FUNCTION` | `Start value function with parameters {parameters}, locals {locals}, and result type {result}.` |
| `CALL_VALUE` | `Call value function {target} with arguments {arguments} into field {key}.` |
| `RETURN_VALUE` | `Return {type} value from expression {expression}.` |
| `OPTION_MAP` | `Map the option with the supplied result.` |
| `TOOL_BEGIN` | `Record the start of tool {tool} with description {label}.` |
| `TOOL_REQUIREMENT` | `Record tool requirement {text}.` |
| `TOOL_INPUT` | `Record tool input {name} with type {type} and secret status {secret}.` |
| `TOOL_OUTPUT` | `Record tool output {name} with type {type} and secret status {secret}.` |
| `TOOL_READ` | `Record tool read {text}.` |
| `TOOL_CALL` | `Record tool call {target}.` |
| `TOOL_WRITE` | `Record tool write {text}.` |
| `TOOL_PERMISSION` | `Record tool permission {text}.` |
| `TOOL_APPROVAL` | `Record tool approval {text}.` |
| `TOOL_SECRET_PROTECTION` | `Record tool secret protection {text}.` |
| `SYSTEM_BEGIN` | `Record the start of component {component} with description {label}.` |
| `SYSTEM_RESOURCE` | `Record resource {name} with type {type}.` |
| `SYSTEM_OWNS` | `Record ownership of resource {resource}.` |
| `SYSTEM_BORROWS` | `Record shared access to resource {resource}.` |
| `SYSTEM_MUTABLY_BORROWS` | `Record exclusive access to resource {resource}.` |
| `SYSTEM_REGION` | `Record region {region} for resource {resource}.` |
| `SYSTEM_LAYOUT` | `Record layout {layout} for resource {resource}.` |
| `SYSTEM_ALLOCATION` | `Record placement {placement} for resource {resource}.` |
| `SYSTEM_LOCK_GUARD` | `Record lock {lock} for resource {resource}.` |
| `SYSTEM_CONTEXT` | `Record context {name}.` |
| `SYSTEM_INTERRUPT_PRIORITY` | `Record interrupt priority {priority} for context {context}.` |
| `SYSTEM_INTERRUPT_MASK` | `Record interrupt mask {mask} for context {context}.` |
| `SYSTEM_SCHEDULER_TASK` | `Record task {task} in context {context}.` |
| `SYSTEM_TASK_PRIORITY` | `Record priority {priority} for task {task}.` |
| `SYSTEM_TASK_TIMING` | `Record deadline {deadline} and budget {budget} for task {task}.` |
| `SYSTEM_CAPABILITY` | `Record system capability {text}.` |
| `SYSTEM_EFFECT` | `Record system effect {text}.` |
| `PASS_BEGIN` | `Record the start of compiler pass {pass} with description {label} for purpose {purpose}.` |
| `PASS_INPUT` | `Record compiler input {name} with type {type}.` |
| `PASS_OUTPUT` | `Record compiler output {name} with type {type}.` |
| `PASS_READ` | `Record compiler read {text}.` |
| `PASS_STEP` | `Record compiler step {text}.` |
| `PASS_WRITE` | `Record compiler write {text}.` |
| `CORE_INFER_READ_PERMISSIONS` | `Infer {permission_kind} permissions through {edge} from {actor_kinds} to {target_kind} with secret policy {secret_policy}.` |
| `ASSERT_GUARANTEE` | `Record guarantee {text}.` |
| `EMIT_TRACE` | `Record trace event {event}.` |
| `RETURN_SUCCESS` | `Return success.` |

[Runtime recovery](51-runtime-recovery.md) supplies balanced data-error scopes.
`errors` is `any` or sorted unique semicolon-separated categories; `key` is the
Text error-code destination. Normal RECOVER execution skips its handler body.
The verifier rejects ordinary jumps into protected or handler regions before
either backend runs. Fatal limits, IO and capability errors remain outside `any`.

[Explicit RAISE](57-explicit-errors.md) takes one `category` technical-data
operand from the existing catchable set. It has no message or data operands,
and uses the same recovery and call-unwinding rules.

## Process execution

The PROCESS_RUN template and exact seven operands are specified in [process resources](53-process-resources.md#core-and-ste). Resource names and all six field operands use technical-data declarations; grants are supplied at execution time.

## Directory effects

[Directory resources](54-directory-resources.md#core-and-ste) specify the exact
DIRECTORY_LIST_NAMES, DIRECTORY_READ_TEXT and DIRECTORY_WRITE_TEXT templates.
Resource and field placeholders use technical data; caller roots are supplied
only at execution. PATH extends the recoverable categories while retaining their
existing indices.

## Executable variant values

The VALUE_CONSTRUCT, VALUE_TEST and VALUE_GET templates and closed type_schemas
metadata are specified in [executable variants](59-executable-variants.md#checked-artifacts-and-ste).
Types, variants, named arguments, payload names and fields use separately declared
technical data. The verifier checks every schema and all operations before either
execution backend runs, including unused variants and unreachable instructions.
