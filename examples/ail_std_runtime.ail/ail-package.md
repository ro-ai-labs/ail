# AIL Package

name: ail.std.runtime
version: 0.2.0
profile: Application
entry: spec.ail-spec.md
features: stdlib, actions, failures, guarantees, traces
imports: ../ail_std_core.ail compatible ^0.2 as Core
imports: ../ail_std_effects.ail compatible ^0.2 as Effects
capability-grants:
  - package: ail.std.effects
    capability: runtime network host effect
    effects: [network]
    approvals: []
conformance: v0.2
schema-version: ail-core.schema.v0
safety-level: standard
target-support:
  ail-core.schema.v0: supported
