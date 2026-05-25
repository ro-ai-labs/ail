# AIL Package

name: ail.std.runtime.missing-grant-fixture
version: 0.2.0
profile: Application
entry: spec.ail-spec.md
features: stdlib, imports
imports: ../../../../ail_std_effects.ail compatible ^0.2 as Effects
conformance: v0.2
schema-version: ail-core.schema.v0
safety-level: standard
target-support:
  ail-core.schema.v0: supported
