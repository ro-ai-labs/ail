# AIL Package

name: support-ticket
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things, actions, failures, guarantees, traces, secrets
conformance: first-slice
target-support:
  linux-x86_64-elf: supported
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
  aarch64-apple-darwin-libsystem-macho: planned-contract
