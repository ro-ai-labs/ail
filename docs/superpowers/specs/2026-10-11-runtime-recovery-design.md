# Structured runtime recovery through Core and STE

The active objective remains Specs → checked Core → ASD-STE100-based STE →
native programs, followed by a language that can replace Python and serve AI
agents. Current list transformations and their1,108-test final run establish
verified progress. A remaining practical gap is data-error recovery: one malformed
row, invalid conversion or missing JSON path currently aborts a batch program.
This milestone adds explicit recovery in actions and reusable functions, including
native unwinding and temporary-resource cleanup. It does not complete the full
Python replacement or establish complete ASD-STE100 conformance.

## Design and alternatives

Use structured recovery delimiters, a shared verified lexical layout and native
handler records. Returning special sentinels from individual operations would
require separate APIs and cannot handle failures from nested calls. Merely
jumping out of a native error label would leak scratch mappings and corrupt
frame/fuel/output state. Structured handlers with explicit cleanup are selected.

Like Python's [termination-style handlers](https://docs.python.org/3/reference/compound_stmts.html#the-try-statement),
recovery abandons the failed operation and the remaining protected body, then
executes the nearest matching handler. Previously completed work stays visible.
AIL uses fixed error categories rather than Python exception objects or classes.

## Source and behavior

```text
- try:
- the system parses JSON text data.row into data.record
- the system gets JSON value at path data.path from data.record into data.value
- recover JSON or JSON-PATH into data.error:
- the system increments data.rejected by 1
- end try
```

An executable action or value function may contain `try:`, one matching
`recover <categories> into <field>:`, and `end try`. Categories are uppercase
tokens separated by ` or `, or the single keyword `any`. The destination must
be a genuine declared Text field in the current scope. Functions use declared
`local.*` storage and existing `the function` operations. Source functions with
Int parameters/results and a Text recovery local become existing VALUE_FUNCTION
procedures. Raw INTEGER_FUNCTION procedures cannot install handlers; their
errors propagate normally to a caller's handler.

Exactly these categories are recoverable:
`INT`, `OVERFLOW`, `DIVZERO`, `FLOAT`, `BOOL`, `TEXT`, `LIST`, `INDEX`, `JSON`,
`JSON-PATH`. `any` matches this fixed set only. Duplicate categories, unknown
categories, mixed `any` and names, missing separators/destination, unmatched or
repeated delimiters and malformed recognized syntax are errors. Normalize source
terminal periods without claiming unrelated identifier prefixes. Recovery is
available only under Application and C interop.

The destination receives the static string `AIL-RUNTIME-<CATEGORY>` on recovery,
never a diagnostic message or operand contents. It remains unchanged when no
error occurs. This static value charges no Text/List allocation, like a literal.
Its value persists after the handler. No exception object, implicit retry, finally
block or user-defined raise is added by this milestone.

- On normal completion, skip the handler and continue after `end try`.
- On a matching error, remove the selected and any inner handlers before entering
  its recovery body. An error in that body can reach an outer protected scope.
- Nonmatching errors propagate to the next enclosing protected scope, including
  caller frames. Unhandled errors retain the existing fatal diagnostic/exit.
- Return expressions are evaluated/validated while current handlers are active.
  Only a successful return leaves their scopes. Break/continue/jumps discard all
  protected scopes they leave. Reentering via `try:` installs a fresh handler.
- Successful earlier field writes, trace events and completed file effects stay
  visible, including those in nested actions. Successful arena charges are not
  refunded. Failed atomic computations leave their destination unchanged; a
  failed function call does not replace its caller's destination.
- Limit/allocation failures, fuel/depth exhaustion, return-protocol failures,
  capability/file-grant/IO errors and unclassified errors remain fatal. Declared
  rule failures with `status=failed` retain their existing behavior.
- Existing family identities remain authoritative: for example malformed Bool
  arguments currently use JSON, while Boolean expressions use BOOL. Normalize
  missing/malformed ADD_INT_FIELD VM operands to INT to match native behavior;
  preserve its existing arithmetic and overflow semantics.

The VM records `recovered <CATEGORY> into <field>` in its trace when entering a
handler. Caught native errors print no diagnostic automatically. Explicit trace
instructions retain their existing backend behavior.

## Core and exact protocol

Extend AilControl with `Try`, `Recover { errors: String, key: String }`, `EndTry`.
Canonical errors are `any` or sorted unique semicolon-separated categories.
Add control kinds `try`, `recover`, `end_try`. An action's recover Control node
has exact attributes `control_kind` and `recovery_errors`; its sole semantic
operand is an attribute-free `recovery_destination` edge to the genuine Text
field. Functions keep their existing owned ValueFunctionStep control nodes and
use `value_field_operand(name=key)` for the local recovery destination. Try/end
nodes have only their control metadata. Existing
ordered control ownership/provenance rules continue to apply. Reject extra
metadata, shared/foreign owners, duplicate/mistyped/forged destination edges.
Imports qualify destination fields, and secret/write-access checking includes
the recovery destination independently of descriptive edges.

Lower into these exact opcodes:

| Opcode | Operands | Meaning |
| --- | --- | --- |
| TRY_BEGIN | errors,key | Install the matching lexical recovery scope |
| RECOVER | none | Normal path removes that handler and skips recovery body |
| TRY_END | none | End the recovery body's lexical region |

The frontend emitter patches TRY_BEGIN from the later Recover metadata. Its
checker must keep one output vector for the full action instead of passing a
fresh vector for each control. At most64 recovery scopes may be lexically nested;
the source's existing total control-depth and step limits remain in force.

Shared `src/recovery.rs` owns category parsing, exact error-prefix classification
and `Layout::analyze(&AilBytecodeAction) -> Result<Layout, String>`. Layout exposes
handlers (begin, recover, end, mask, key), a protected-handler sequence for each
instruction, and `exited(from,to)` for ordinary branches/jumps. Each lexical
region has an identity and Protected/Recover phase. A jump target's region
sequence must be a prefix of its source's sequence: jumps can exit scopes but
cannot enter a body or cross same-depth sibling scopes. TRY_BEGIN may be targeted
from outside because executing it installs the handler. RECOVER performs its
own special normal transfer; only error dispatch enters a handler from outside.
Returns may leave all scopes. Reject malformed raw bytecode and direct STE
before either backend runs. Protocol diagnostics use AILBC019; source/Core use
AIL-CONTROL-001 and existing typed-function diagnostics as appropriate.

STE templates (operands remain separately declared technical data):

```text
Begin recovery for errors {errors} into field {key}.
Run the recovery handler.
End recovery.
```

## Runtime architecture

VM actions and value functions consult the verified lexical layout at each
failing instruction. Nested action execution must retain mutable state and trace
on error so an outer handler observes completed callee work. Do not clone all
state per instruction or protected block. Function locals remain isolated and
discarded on propagation. Existing per-scope fuel accounting remains cumulative
across recovery; handler execution consumes normal instruction fuel.

Native handler records preserve the recovery PC, mask, destination slot, prior
handler, stack/frame/depth context, owner live-fuel-cell pointer and output-scope
checkpoint. Helpers reuse fuel registers as scratch, so do not restore fuel from
throw-time registers or the original TRY entry value. Maintain the owner's live
fuel in memory at instruction boundaries. Unwinding restores caller output
selection metadata without reverting field values. Successful return removes
owned handlers only after result validation/copy has succeeded.

Temporary native mmap storage must be reclaimable after errors. Use intrusive
doubly-linked headers in an extra page before temporary mapping payloads, with
one registry rooted in the execution context. Normal frees can remove mappings
in any order; recovery cleans all live temporary mappings before resetting the
machine stack. Persistent64MiB Text and List/Json arenas stay outside this
registry and retain successful allocation counters. Current statement/call-entry
boundaries have no live scratch mappings. Cover list/JSON scratch, formatters,
text matcher/search scratch and file-read temporary buffers. File descriptors
must close before a recoverable UTF-8 validation error. The embedded Float
runtime is allocation-free and returns statuses; do not unwind its Rust stack.
No new dependencies, runtime compiler/interpreter/subprocess or runtime files.

### Shared implementation interface

`recovery.rs` exposes these crate-internal interfaces to the frontend and native
workers (all other implementation details remain private):

```rust
pub(super) const CATEGORIES: [&str; 10]; // order from the catchable list above
pub(super) fn source_errors(text: &str) -> Result<String, String>;
pub(super) fn mask(errors: &str) -> Result<u16, String>;
pub(super) fn category(message: &str) -> Option<&'static str>;
pub(super) fn code(category: &str) -> Option<&'static str>;
pub(super) struct Handler {
    pub begin: usize, pub recover: usize, pub end: usize,
    pub mask: u16, pub key: String,
}
pub(super) struct Layout { pub handlers: Vec<Handler>, /* private indexes */ }
impl Layout {
    pub fn analyze(action: &AilBytecodeAction) -> Result<Self, String>;
    pub fn protected(&self, pc: usize) -> &[usize]; // handler indexes, outer first
    pub fn beginning(&self, pc: usize) -> Option<&Handler>;
    pub fn recovering(&self, pc: usize) -> Option<&Handler>;
    pub fn exited(&self, from: usize, to: usize) -> Result<usize, String>;
    pub fn matching(&self, pc: usize, category: &str) -> Option<&Handler>;
}
```

The destination `pc == instruction_count` denotes the empty outer scope.
`source_errors` accepts source ` or ` separators and normalizes order; `mask`
requires canonical sorted/unique semicolon protocol tokens or `any`. Errors from
this module carry AILBC019. Consumers may wrap them in source/Core diagnostics.

## Deliverables and verification

Implement source/Core/STE/bytecode/VM/native support; tests must observe missing
support before implementation. Cover every recoverable family, specific and any
filters, normal fallthrough, nested/nonmatching/handler failures, all forms of
scope exit/reentry, return-expression errors, recursive calls and deep repeated
unwind. Check direct/saved/rendered Core, STE, imported/local ownership, secrets,
profile restrictions and forged bytecode jumps/metadata.

Independent expectations must check values, prior state/effects, handler codes,
silent caught native errors, exact arena accounting and unchanged atomic
destinations. Fatal families must remain fatal inside `any`. Repeated malformed
list/JSON/text/file inputs under bounded native address space and file-descriptor
limits must demonstrate scratch cleanup. A repeated caught-error loop must still
exhaust fuel. Exercise caller output-selection restoration and stack/depth reuse.

Add `examples/recovering-json-lines` and exported STE: read a newline-separated
JSON file, process each row through a reusable function, accumulate valid numeric
values, count rejected rows by stable error family and write a real JSON report.
Test actual report bytes independently on all six source/Core/STE routes in VM
and native, including malformed rows, wrong kinds and missing fields. Document
recovery in chapter51, source/Core/STE/function/failure references and the agent
guide. Independent reviews, final serial full Rust suite, strict Clippy/fmt/diff,
new/prior differential runners and frozen16/16 benchmark complete this milestone.
