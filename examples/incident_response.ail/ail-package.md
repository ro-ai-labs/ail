# AIL Package

name: incident-response
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports, users, things, actions, routes, forms, dashboards, workflows, failures, guarantees, traces
imports: ../incident_identity.ail as Identity
imports: ../incident_policy.ail as Policy
imports: ../incident_notifications.ail as Notify
conformance: multi-module-system
schema-version: ail-core.schema.v0
safety-level: standard
target-support:
  ail-core.schema.v0: supported
  wasm32-unknown-sandbox-wasm: supported
  aarch64-apple-darwin-libsystem-macho: planned-contract
