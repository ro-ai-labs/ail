# AIL Package

name: missing-registry-import
version: 0.2.0
profile: Application
entry: spec.ail-spec.md
features: imports, registry
registry: registry/ail-registry.md
imports: shared-lib@0.1.0 as Shared
conformance: v0.2
schema-version: ail-core.schema.v0
safety-level: standard
target-support:
  vm: supported
