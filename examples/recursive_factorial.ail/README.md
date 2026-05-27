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

This package includes `examples/accepted/recursive-with-stack-bound.ail-spec.md`
to prove that an explicit numeric recursion-depth bound is checker-visible
termination evidence. It also includes
`examples/rejected/recursive-without-base-case.ail-spec.md` and
`examples/rejected/recursive-without-decreasing-argument.ail-spec.md` to prove
that recursive functions without a checker-visible base-case branch or
decreasing argument fail with `AIL-CONTROL-003`.

## Next Example To Read

Read `stateful_counter.ail/README.md` next for state mutation, then
`compiler_pass.ail/README.md` for compiler-level transformations over checked
Core.

## v0.3 Learning Signal

Recursive examples now have checker-visible base-case, decreasing-argument,
and explicit stack-depth evidence. AIL still needs richer recursion proof
forms before it can claim a stronger Turing-core teaching path.
