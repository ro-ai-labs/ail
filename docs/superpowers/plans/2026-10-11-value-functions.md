# General value functions implementation plan

**Spec:** ../specs/2026-10-11-value-functions-design.md
**Goal:** Reusable typed local computations and record transformations through
the existing STE-to-VM/native pipeline, preserving integer behavior.
**Architecture:** Extend the source AST, isolate general Core/protocol/VM/native
modules, and retain old integer lowering for pure integer call graphs.

## Ownership and interfaces

- Frontend agent: integer_function.rs and new value_function.rs. Expose
  classify(document), compile(function,document), action_call_instruction,
  elaborate/check/from_core_function/accesses. Extend IntegerStep::ValueOperation
  and existing rendering/parsing/namespacing. Delegate general action calls in
  the existing integer_call_step loop, and decode by call node kind in place.
- Root: new value_protocol.rs and value_vm.rs; ail.rs/action_order.rs/ste.rs;
  integer_vm.rs depth API/verifier interoperability, list input-validation helper,
  example/docs. Root also coordinates exact Core kind/edge names from frontend.
- Native agent: new native_value_function.rs and native_expr.rs plus necessary
  focused helper changes for trace suppression and helper dependencies.
- Test agent: tests/value_functions.rs and scripts/ail_value_function_differential.py.

Shared value_protocol API:

```
Signature { parameters:BTreeMap<String,String>, locals:BTreeMap<String,String>, result:String }
signature(action)->Result<Signature,String> // VALUE_FUNCTION only
signatures(program)->Result<BTreeMap<String,Signature>,String> // integer and general
is_function(action)->bool // VALUE_FUNCTION only
OperandKind::{Field(String), Expression}
operand_schema(instruction)->Result<BTreeMap<String,OperandKind>,String>
instruction_fields(instruction)->Result<BTreeMap<String,String>,String>
validate_body_instruction(instruction,fields,result,signatures)->Result<(),String>
```

Signature maps describe names without local. prefixes. Validation fields use
full local.name keys. Operation schema covers literal/text/list/JSON and EVAL_INT,
BRANCH_INT; call/return validation uses signatures and result metadata separately.
Schemas explicitly classify operand roles; metadata is never guessed from text.
Expose reusable signature map encoding/decoding and type support checks as needed.

Native prepare/emit APIs may choose internal names, using shared strict signature
and argument decoders. General calls pass fixed two-word typed packets. Int/Bool
returns use scalar registers; other results return one stable arena pointer/length.
Root VM calls old integer Machine::call_at_depth(name,args,depth,trace), where
depth is the number of already-active frames before entering that integer call.

## Tasks and acceptance

- [x] Write red source/typed protocol tests before implementation. Keep existing
  integer and global-only operation rejection tests intact.
- [x] Root protocol and borrowed-frame VM, typed boundary validation/return
  allocation and exact signature/body checking; depth-aware integer entry.
- [x] Frontend AST/classification/compilation/Core, strict ownership and source
  round trips, in-order general/integer action calls, imports and secret accesses.
- [x] Native general frame/call/return implementation, local operations and
  helper dependency emission; explicit traces only, no stale stack pointers.
- [x] Root integration for action/function compilation, direct entry/profile
  rejection, STE and native bundle/trace paths.
- [x] Independent integration/oracle tests and a runnable function-based JSON
  report package, exported STE, guide and updated current-scope documentation.
- [x] Independent cross-reviews; fix findings with regression evidence; full
  serial Rust suite, strict lint/format, existing differential checks and benchmark.
- [x] Record evidence and leave the broader goal active.

## Review focus

1. General classification must propagate through callers even for Int→Int
   signatures; original integer-only graphs must keep their old protocol.
2. Parameters validate in sorted order, including unused parameters, without
   arena charges. List parameter validation must allow canonical expansion until
   a result requests encoding; Json still checks canonical-size bounds.
3. Non-scalar returns copy exactly once before frame teardown, including borrowed
   arguments and literals. Copies survive caller overwrites and later calls.
4. One depth count crosses general-to-integer calls; each invocation retains its
   own fuel. No global or cross-function local access through forged Core/STE.
5. Core fields and bytecode metadata agree exactly. Scope, type, profile, secret
   reads/writes and all-actions/root-entry rejection cannot be bypassed.

Ruling: continue within the already-authorized shared worktree without approval
pauses or commits. Preserve all previous work; .git is read-only. Implementation
and test agents own disjoint domains, then review each other's work read-only.

## Implementation and review evidence

- Initial focused source/frontend/protocol/VM/native tests each failed for missing
  functionality before their implementation. Protocol red/green logs:
  `/tmp/ail-value-protocol-red.log`, `/tmp/ail-value-protocol-green.log`; VM:
  `/tmp/ail-value-vm-red.log`, `/tmp/ail-value-vm-green.log`.
- Native implementation passed 108 real-ELF tests, including 14 new value-function
  tests. Independent read-only native review found no concrete defect.
- Independent VM review found an internal constructor panic on malformed legacy
  integer dependencies. Observed regression in
  `/tmp/ail-value-constructor-red.log`; full protocol validation now precedes
  preparation. Five VM tests pass in `/tmp/ail-value-constructor-green.log`.
- Independent frontend review found that an unowned duplicate global Field could
  evade action-call secret validation. The regression first rejected the old
  implementation with `orphan value_call_argument field accepted`. Typed calls
  now require a unique owning Thing declaration and unique runtime field key.
  Tests cover Text arguments, destinations and Int expression leaves, including
  competing declared fields. The original reproduction now rejects checking and
  compilation: `/tmp/ail-value-review-fixed.log`.
- Public integration initially passed 13 tests, then gained two CLI tests for
  all-actions bundles/direct-entry rejection and callee-only Wasm trace contracts.
  The initial CLI test mistakenly omitted the existing required --action option;
  the test was corrected without changing CLI behavior.
- New example: `examples/value-functions`, exported STE:
  `examples/ste/value-functions.ail-ste`, guide: `docs/ail/44-value-functions.md`.
  Exact reports agree across source/Core/STE × VM/native in
  `/tmp/ail-value-final-example.log`.
- Earlier JSON/literal oracles all passed against this implementation:
  `/tmp/ail-value-regression-{literal,codec,recursive,mutation}.log`.
  Frozen ladder 16/16: `/tmp/ail-value-regression-bench.log`.
  Corpus 33 agree, 30 unsupported, zero divergent:
  `/tmp/ail-value-regression-corpus.log`. Existing benchmark inputs remain intact.

Final verification after the ownership fix:

- `cargo test --all-targets -- --test-threads=1`: **868 passed**, zero failures,
  across 15 targets (`/tmp/ail-value-final-tests.log`). This includes 270 library
  tests and 15 public value-function integration tests.
- `cargo clippy --all-targets -- -D warnings`: passed
  (`/tmp/ail-value-final-clippy.log`); `cargo fmt --all -- --check` and
  `git diff --check`: passed. Final development build passed.
- Independent Python value-function comparison rerun passed:
  360 six-type return cases, 100 local Text/List transformations, recursion64/65,
  unused argument/error ordering, raw/canonical list distinction, exact shared
  Text/List/Json return budgets, and 50 composed file reports
  (`/tmp/ail-value-final-oracle.log`).
- Final example rerun agrees exactly in all six source/Core/STE × VM/native
  routes (`/tmp/ail-value-final-example.log`). Local documentation links checked.
- No unresolved review findings for this milestone. Work remains uncommitted
  in the existing workspace. The broader Python replacement and agent-usable
  language goal remains **active**; general functions are verified progress,
  not completion of that broader objective.

