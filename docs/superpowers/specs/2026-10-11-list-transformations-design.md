# Typed list transformations through Core and STE

The continuing objective is an AI-agent-usable Python replacement through
Specs → checked Core → ASD-STE100-based STE → VM/native executables. The preceding
turn is verified progress: text transformations, 1,064 passing tests and independent
reports/oracles are recorded in its completed plan and current files. The next
gap is efficient collection processing: existing lists only create/count/get/
append/set. Repeated immutable updates require repeated full parsing/copying.
Add whole-list operations and demonstrate actual file-based statistics. This
increment does not redefine or complete the broader replacement objective.

## Design and alternatives

Extend the existing typed ListOperation and descriptor codec. This exposes
semantics to Core, STE, static accesses and both backends without external
dependencies. Implementing sorting entirely through existing append/set would
multiply parsing and result allocation; adding a new dynamic collection model
would change unrelated type/ownership contracts. Direct bounded list operations
are the selected extension. They retain immutable result ownership and give
programs efficient slicing and stable sorting at the existing maximum list size.

### Locked interfaces and source

All list operands and destinations are fields with exactly the same one of
List<Int>, List<Float>, List<Text>. Slice bounds are checked Int expressions.
Actions use `the system`; executable value functions use `the function` and
declared locals. Add these exact enum variants to AilListOperation:

```text
Concat { key:String, source:String, other:String }
Slice { key:String, source:String, start:IntExpr, end:IntExpr }
Reverse { key:String, source:String }
Sort { key:String, source:String, descending:bool }
```

```text
- the system concatenates list data.left and list data.right into data.result
- the system slices list data.values from data.start to data.end into data.result
- the system reverses list data.values into data.result
- the system sorts list data.values in ascending order into data.result
- the system sorts list data.values in descending order into data.result
```

Source requires an explicit sort direction, exactly `ascending` or `descending`.
No callback or implicit conversion. Existing operation syntax and behavior stay
compatible. New recognized malformed forms must fail under actions, executable
functions, descriptive function bodies and compiler-pass bodies; normalize
terminal punctuation and respect keyword boundaries/unrelated identifiers.

### Semantics

- Concat preserves all items from the first list, then all items from the second.
- Slice includes start and excludes end, with step 1. Negative bounds are
  relative to length; clamp either bound to [0,length], including i64 extremes.
  If end <= start, the result is empty. Evaluate both bounds once, start first,
  even if start alone would imply an empty result.
- Reverse produces the items in opposite order.
- Sort is stable in both directions. Int sorts numerically as signed i64.
  Text sorts lexicographically by Unicode scalar values (equivalent to byte
  order for valid UTF-8), with prefix before extension in ascending order.
  Preserve Unicode spelling, combining sequences, control characters and NUL.
  Float sorts numerically including infinities; +0 and -0 compare equal and
  retain their relative order. NaNs are always placed after all numbers in BOTH
  directions, remain stable among themselves, and are preserved as canonical nan.
  Descending reverses numeric/text comparison, not the final sorted array.

Slice/concat/reverse and sorting of ordered values follow the corresponding
[Python sequence operations](https://docs.python.org/3/library/stdtypes.html#common-sequence-operations)
and [stable sorting contract](https://docs.python.org/3/howto/sorting.html).
NaNs have no ordinary comparison order; AIL explicitly chooses stable NaN-last
ordering instead of claiming exact Python `sorted` behavior on interspersed NaNs.

### Bounds, validation and ownership

Existing limits: 1,048,576 raw bytes per input list; 65,536 items per input and
result; 1,048,576 canonical result bytes; shared 67,108,864-byte List/Json arena.
Each operation charges exactly its final canonical result length once, including
empty `[]` (2 bytes) and unchanged results. No Text arena charge. Scratch is
temporary and uncharged; successful function return adds the existing one-copy
charge. All input reads precede destination mutation; aliases and earlier copies
retain values. On failure, preserve destination and both arena counters.

Borrowed inputs need not fit the canonical result bound. Slice can shrink such
an input; reverse/sort/concat must reject a canonical result that exceeds it.
Concat validates both inputs completely before combined result count/size and
allocation, even if the first list already has the maximum count. Slice validates
the complete input before start expression, then end expression, then result
limits/allocation. Reverse/sort validate the complete source before producing.
Input errors use LIST/LIST-LIMIT; slice expression errors keep INT/DIVZERO/overflow
families and evaluation order. No INDEX error merely for clamped slice bounds.
Output limits and scratch/result allocation errors use LIST-LIMIT. Diagnostics
must not include operand contents, including invalid escape characters.

Sort parses each input once and uses stable O(n log n) comparisons with O(n)
descriptor scratch; no repeated full-list decode, no quadratic insertion sort.
Text comparison cost includes compared text bytes. Concat/slice/reverse are
linear in input/result size. Native uses existing list_parse/list_encode/
list_allocate, sorts descriptors and copies the final canonical result once.
All native helpers preserve frame/fuel registers and release scratch on success.
No interpreter, subprocess compiler, runtime file or dependency is added.

### Core, protocol and integration

Reuse untyped ListOperation with exact list_kind=concat|slice|reverse|sort,
element_type=Int|Float|Text. Sort alone requires sort_order=ascending|descending.
Existing list_source/list_destination apply. New list_other points to Concat's
second list; list_start/list_end point to typed checked IntegerExpression trees.
Operands are unique, attribute-free, genuine fields/expressions in the right
scope; unique ordered executable Action owner, exact metadata and no foreign
parents. Strengthen shared list expression decoding where necessary without
changing valid old Core. Preserve provenance.

All reads, including both lists and both bound expressions, participate in
secret-access checks even if descriptive access edges are absent. Namespace
field keys and expression fields, leaving enum strings intact. Functions reuse
ValueFunctionStep typed operand schema; preserve local ownership and argument/
return validation. Float helper discovery must include new Float-list operations
inside functions; Int/Text-only programs must not acquire Float runtime payloads.

Exact opcodes (type is element type):

| Opcode | Operands |
| --- | --- |
| LIST_CONCAT | key,source,other,type |
| LIST_SLICE | key,source,start,end,type |
| LIST_REVERSE | key,source,type |
| LIST_SORT | key,source,order,type |

Use AIL-LIST-001 and AILBC013. New operations only execute in Application/C interop;
retain old raw opcode behavior. Action trace stays `computed list <key>`; local
operations add no implicit traces or selected outputs.

Exact STE templates:

```text
Concatenate {type} list {source} and list {other} into field {key}.
Slice {type} list {source} from integer {start} to integer {end} into field {key}.
Reverse {type} list {source} into field {key}.
Sort {type} list {source} in {order} order into field {key}.
```

## Deliverables and verification

Source/Core/STE/VM/native implementation plus focused RED/GREEN, independent
Python-generated expectations for all three types and both sort directions,
signed-zero stability, infinities/NaNs, Unicode/NUL, aliases, errors, boundaries
and shared arenas/return copies. Adversarial 65,536-item sorts must complete under
a reasonable timeout; use shuffled, sorted, reverse, duplicate and common-prefix
text inputs. Test full-source validation for empty slices, both concat inputs,
borrowed canonical expansion, i64 min/max bounds, metadata/type/ownership/secret
forgeries, imports, local expressions, legacy/compiler guards and Float discovery.

Add examples/list-statistics and exported STE: read JSON batches, concatenate,
sort, slice/reverse a top window, compute median with reusable functions and write
a JSON report. Test real report bytes across source/saved Core/rendered source,
each directly and via STE, in VM/native. Add docs/ail/50-list-transformations.md
and indices/source/schema/STE/function references. Independent reviews, final
serial all-target tests, strict Clippy/fmt/diff, prior oracles and frozen16/16
complete this increment. Preserve benchmark inputs/answers/SCORE.lock and the
existing uncommitted worktree.
