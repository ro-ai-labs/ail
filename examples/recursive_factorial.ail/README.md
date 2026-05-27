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

Focused tests and catalog entries `example-117` through `example-121` lower
this package into checked Core, verified bytecode, and VM traces for recursive
factorial inputs. The stored prompt and response transcripts exercise the
interview, spec draft, Core draft, Core-to-spec, and trace-debug prompt
surfaces against the same compact recursion fixture.

## Rejected Fixtures

This package includes `examples/rejected/recursive-without-base-case.ail-spec.md`
to prove that recursive functions without a checker-visible base-case branch
fail with `AIL-CONTROL-003`. v0.3 should still add rejected function fixtures
for non-integer recursion input and unbounded recursion without a decreasing
argument.

## Next Example To Read

Read `stateful_counter.ail/README.md` next for state mutation, then
`compiler_pass.ail/README.md` for compiler-level transformations over checked
Core.

## v0.3 Learning Signal

Recursive examples now have a first checker-visible termination diagnostic.
AIL still needs explicit stack-depth policy and richer recursion proofs before
it can claim a stronger Turing-core teaching path.
