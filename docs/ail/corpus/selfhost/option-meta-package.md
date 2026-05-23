# Selfhost Fixture: Option Meta Package

status: accepted language-definition package
profile: AIL-Meta

Feature: `Option<T>`

Required generated components:

- parser rule for `Option<T>`
- AIL-Core node and edge mapping for `Some` and `None`
- checker rule requiring exhaustive `Match`
- renderer rule for canonical AIL-Spec
- diagnostic `AIL-CONTROL-002`
- AIL-Flow block rule for Option match
- valid example: `Option.map`
- invalid example: missing `None` branch
- trace fixture: `OptionMapEvaluated`
- migration note: no migration for initial release

Acceptance:

- bootstrap compiler consumes the package as AIL-Meta source
- generated checker or renderer component is included in the conformance report

Executable reference:

- `examples/compiler_pass.ail` is the current consumed AIL-authored compiler
  package for the related "action reads field" feature class
- `cargo test --test ail_toolchain compiler_pass` verifies parsing, checking,
  lowering, bytecode execution, pass execution over AIL-Core, diagnostics, and
  traces for that package
