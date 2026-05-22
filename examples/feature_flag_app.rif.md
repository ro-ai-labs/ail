app FeatureFlags

things:
  thing Feature
    field id: Id<Feature>
    field enabled: Bool

operations:
  operation Audit.record(enabled: Bool) -> Unit

intent EnableFeature

subject:
  feature: Feature

steps:
  1. Enable feature
     set: feature.enabled = true
     changes: feature.enabled

  2. Audit previous state
     call: Audit.record(false)

guarantees:
  if this intent succeeds:
    feature.enabled == true
