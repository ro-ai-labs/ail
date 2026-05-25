# AIL Package

name: ail.std.collections
version: 0.2.0
profile: Application
entry: spec.ail-spec.md
features: stdlib, types, functions, collection-transform, traces
imports: ../ail_std_core.ail compatible ^0.2 as Core
conformance: v0.2
schema-version: ail-core.schema.v0
safety-level: standard
target-support:
  ail-core.schema.v0: supported
