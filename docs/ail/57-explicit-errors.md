# Explicit runtime errors

An action or executable function can deliberately reject invalid domain data:

```text
- the system raises error BYTES
- the function raises error JSON
```

The category is one literal from the existing catchable set: `INT`, `OVERFLOW`,
`DIVZERO`, `FLOAT`, `BOOL`, `TEXT`, `LIST`, `INDEX`, `JSON`, `JSON-PATH`, `PROCESS`,
`TIMEOUT`, `PATH`, `BYTES`, or `VALUE`. Dynamic categories, messages, compound categories,
`any` and fatal categories such as `BYTES-LIMIT` are not accepted.

The statement produces the corresponding `AIL-RUNTIME-*` error. The nearest
matching protected scope handles it under the existing [recovery rules](51-runtime-recovery.md).
An explicit raise inside a recovery body can be handled by an outer scope. An
unmatched error propagates through action/function calls and terminates execution.
Later statements do not run; previous state changes and external effects remain.
A failed function does not overwrite its caller's result destination.

For example, a binary format parser can reject a zero count without manufacturing
an unrelated arithmetic failure:

```text
- if local.count == 0:
- the function raises error BYTES
- end if
```

Raise-only functions are allowed. A function containing raise statements must
return or raise on every reachable exit path; a branch that falls through still
rejects. This structural check explores both branch outcomes and possible recovery paths;
it does not prove conditions symbolically. Existing functions without raise
retain their previous behavior.

Core action nodes use the existing `Control` kind, `control_kind=raise` and one
`error_category` attribute, with ordered `control_step` ownership and no operand
edges. Function control steps retain their scoped ownership schema. Categories
are preserved by imports and source/Core roundtrips. Invalid categories or
structural metadata reject even when unreachable.

Bytecode is `RAISE` with exactly the `category` operand. AIL-STE uses
`Raise error {category}.`, with the category in technical data. Application and
C interop support execution; unsupported profiles and descriptive functions
reject reserved raise forms. Invalid bytecode uses `AILBC019`; source and Core
retain the established control/function diagnostic families.

The category indices, recovery masks, native unwind rules and runtime limits are
unchanged. The [run-length codec](58-run-length-codec.md) uses explicit errors in
an imported pure AIL library and exercises them through both VM and standalone ELF.
