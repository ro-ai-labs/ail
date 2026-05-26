# stdlib-collections-live-spec-input-2 User Story

user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-journey: story-to-spec
story-roundtrip: semantic-similar
story-evidence: checked-core
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
semantic-anchors: Option<T>; Result<T,E>; Map<K,V>; Option.map; OptionMapEvaluated; spec-draft.system.md
