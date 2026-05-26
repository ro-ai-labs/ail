# Recursive Factorial Example

## Purpose

This package is the compact executable-semantics fixture for recursive
functions. It keeps recursion separate from larger application packages so
bytecode control-flow behavior can be inspected directly.

## Concepts Taught

- Function surfaces in AIL-Spec.
- Recursive self-calls with base and recursive cases.
- Integer arithmetic lowering and `FactorialCalled` trace evidence.

## Files To Inspect

- `ail-package.md`: function, recursion, and trace feature metadata.
- `spec.ail-spec.md`: `factorial` function input, output, recursive call, and
  trace event.
- `../../tests/ail_toolchain.rs`: tests named
  `ail_spec_lowers_function_surface_into_runnable_bytecode` and
  `ail_bytecode_vm_executes_action_call_control_flow` exercise the same
  lowering surface.

## Expected Replay Artifacts

Focused tests lower this package into checked Core and runnable bytecode for
the recursive function surface. Future catalog entries should add stored
request/response transcripts and per-entry VM trace evidence for factorial
inputs.

## Rejected Fixtures

This package has no package-local rejected fixtures. v0.3 should add rejected
function fixtures for missing base case, non-integer recursion input, and
unbounded recursion without a checker-visible guard.

## Next Example To Read

Read `stateful_counter.ail/README.md` next for state mutation, then
`compiler_pass.ail/README.md` for compiler-level transformations over checked
Core.

## v0.3 Learning Signal

Recursive examples need explicit termination and stack-safety diagnostics
before AIL can claim a stronger Turing-core teaching path.
