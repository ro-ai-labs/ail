# AIL Package

name: c-interop
version: 0.2.0
profile: C interop
entry: spec.ail-spec.md
features: c-interop, external-bindings, abi-layout, callbacks, ownership, failures, traces
conformance: v0.2
schema-version: ail-core.schema.v0
safety-level: expert
target-support:
  ail-core.schema.v0: supported
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
