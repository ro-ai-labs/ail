# AIL v0.2 Example Validation Catalog

This checked catalog stores prompt and response transcripts for the
`ail-examples` release verifier. Every counted example is replayed through the
prompt-to-artifact path and produces deterministic verification evidence.

## Use-Case Coverage Summary

Every entry below carries learning metadata:

- `use-case`: the practical scenario the example exists to teach or prove.
- `capability-level`: `low-level`, `mid-level`, or `high-level`.
- `capability-under-test`: the concrete AIL surface under pressure.
- `program-scale`: `utility`, `module`, or `multi-module-system`.
- `program-domain`: the practical domain being exercised: OS utility,
  C interop, compiler, runtime, package graph, application, agent tool,
  UI workflow, system driver, or diagnostic.
- `module-count`, `spec-count`, and `story-count`: declared interaction depth
  for the example; multi-module systems must use values of at least `2`.
- `interacts-with`: named modules, systems, or contracts crossed by the
  example, or `none` for standalone utilities.
- `user-story-id`: stable story family; it may repeat across prompt, target,
  and repair variants while `semantic-task` stays unique.
- `user-story`: one-line user-story view for the checked behavior.
- `acceptance-criteria`: observable story criteria tied to replay evidence.
- `story-evidence`: the strongest artifact that proves the story path.
- `story-journey`: story-to-spec, spec-to-story, amendment, or diagnostic
  preservation path.
- `story-roundtrip`: expected semantic preservation mode for regenerated
  stories.
- `semantic-anchors` in the referenced story file when present: the terms,
  actions, modules, targets, or diagnostics replay reports as semantic
  preservation evidence.
- `distinctness-claim`: why this entry is useful even when it shares a package
  with another prompt-surface example.
- `v0.3-signal`: what the example tells us to improve in the next language and
  toolchain version.

Current capability-level coverage is:

- `low-level`: C/host interop, compiler passes, network/system effects, and
  backend portability.
- `mid-level`: standard library packages, package imports, runtime generics,
  secret/permission checks, and deterministic state.
- `high-level`: application workflows, AgentTool safety, UI workflows,
  scheduled work, and diagnostic repair paths.

The catalog intentionally contains prompt-surface matrices over some packages.
Those entries are useful only when their `distinctness-claim` identifies the
prompt, target, diagnostic, checker assertion, or human-review path being
validated.

## Example: example-0
semantic-task: stdlib-collections-live-codex-interview-0
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-0.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-interview-0 exercises docs/ail/prompts/interview.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-0.json
response-file: responses/example-0.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-1
semantic-task: stdlib-collections-live-spec-input-1
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-1.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-spec-input-1 exercises docs/ail/prompts/requirements.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: unsloth-qwen3.6-35b-a3b-gguf-chat-requirements
capture-origin: live-llm
request-file: requests/example-1.json
response-file: responses/example-1.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
endpoint-label: inteligentia-pro-1-qwen3.6-35b-chat
## Example: example-2
semantic-task: stdlib-collections-live-spec-input-2
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-2.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-spec-input-2 exercises docs/ail/prompts/spec-draft.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: llm-http
executor-label: unsloth-qwen3.6-35b-a3b-gguf-chat
capture-origin: live-llm
request-file: requests/example-2.json
response-file: responses/example-2.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
endpoint-label: inteligentia-pro-1-qwen3.6-35b-chat
## Example: example-3
semantic-task: stdlib-collections-live-codex-core-draft-3
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-3.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-core-draft-3 exercises docs/ail/prompts/core-draft.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-3.json
response-file: responses/example-3.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-4
semantic-task: stdlib-collections-live-codex-diagnostic-repair-4
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-4.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-diagnostic-repair-4 exercises docs/ail/prompts/diagnostic-repair.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-4.json
response-file: responses/example-4.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-5
semantic-task: stdlib-collections-live-codex-core-to-spec-5
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-5.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-core-to-spec-5 exercises docs/ail/prompts/core-to-spec.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-5.json
response-file: responses/example-5.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-6
semantic-task: stdlib-collections-live-codex-core-to-summary-6
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-6.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-core-to-summary-6 exercises docs/ail/prompts/core-to-summary.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-6.json
response-file: responses/example-6.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-7
semantic-task: stdlib-collections-live-codex-flow-patch-7
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-7.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-flow-patch-7 exercises docs/ail/prompts/flow-patch.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-7.json
response-file: responses/example-7.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-8
semantic-task: stdlib-collections-live-codex-trace-debug-8
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-8.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-trace-debug-8 exercises docs/ail/prompts/trace-debug.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-8.json
response-file: responses/example-8.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-9
semantic-task: stdlib-collections-live-codex-interop-9
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
use-case: Standard library collection semantics with generic Option/List/Map behavior.
capability-level: mid-level
capability-under-test: stdlib-generics
program-scale: multi-module-system
program-domain: package-graph
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ail_std.option,ail_std.list,ail_std.map
user-story-id: stdlib-collections-story
user-story: As a reviewer I can inspect stdlib-collections behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-9.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stdlib-collections-live-codex-interop-9 exercises docs/ail/prompts/interop.system.md over stdlib-generics.
v0.3-signal: Generics need reusable conformance fixtures and teachable stdlib walkthroughs.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-9.json
response-file: responses/example-9.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-10
semantic-task: support-composed-live-codex-interview-10
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-10.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-interview-10 exercises docs/ail/prompts/interview.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-10.json
response-file: responses/example-10.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-11
semantic-task: support-composed-live-codex-requirements-11
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-11.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-requirements-11 exercises docs/ail/prompts/requirements.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-11.json
response-file: responses/example-11.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-12
semantic-task: support-composed-live-codex-spec-draft-12
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-12.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-spec-draft-12 exercises docs/ail/prompts/spec-draft.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-12.json
response-file: responses/example-12.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-13
semantic-task: support-composed-live-codex-core-draft-13
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-13.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-core-draft-13 exercises docs/ail/prompts/core-draft.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-13.json
response-file: responses/example-13.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-14
semantic-task: support-composed-live-codex-diagnostic-repair-14
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-14.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-diagnostic-repair-14 exercises docs/ail/prompts/diagnostic-repair.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-14.json
response-file: responses/example-14.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-15
semantic-task: support-composed-live-codex-core-to-spec-15
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-15.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-core-to-spec-15 exercises docs/ail/prompts/core-to-spec.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-15.json
response-file: responses/example-15.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-16
semantic-task: support-composed-live-codex-core-to-summary-16
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-16.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-core-to-summary-16 exercises docs/ail/prompts/core-to-summary.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-16.json
response-file: responses/example-16.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-17
semantic-task: support-composed-live-codex-flow-patch-17
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-17.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-flow-patch-17 exercises docs/ail/prompts/flow-patch.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-17.json
response-file: responses/example-17.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-18
semantic-task: support-composed-live-codex-trace-debug-18
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-18.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-trace-debug-18 exercises docs/ail/prompts/trace-debug.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-18.json
response-file: responses/example-18.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-19
semantic-task: support-composed-live-codex-interop-19
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
use-case: Package composition with explicit imports and capability grants.
capability-level: mid-level
capability-under-test: package-imports
program-scale: module
program-domain: package-graph
module-count: 2
spec-count: 2
story-count: 2
interacts-with: support_shared
user-story-id: support-composed-story
user-story: As a reviewer I can inspect support-composed behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-19.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: support-composed-live-codex-interop-19 exercises docs/ail/prompts/interop.system.md over package-imports.
v0.3-signal: Package graphs need clearer authoring guidance and dependency review views.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-19.json
response-file: responses/example-19.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-20
semantic-task: option-map-live-codex-interview-20
profile: Application
surface-tags: ui
package: examples/option_map.ail
use-case: Small transform used to exercise typed option mapping and UI-tagged prompt surfaces.
capability-level: high-level
capability-under-test: ui-surface-coverage
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.form,ui.route,ui.state
user-story-id: option-map-story
user-story: As a reviewer I can inspect option-map behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-20.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: option-map-live-codex-interview-20 exercises docs/ail/prompts/interview.system.md over ui-surface-coverage.
v0.3-signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-20.json
response-file: responses/example-20.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-21
semantic-task: option-map-live-codex-requirements-21
profile: Application
surface-tags: ui
package: examples/option_map.ail
use-case: Small transform used to exercise typed option mapping and UI-tagged prompt surfaces.
capability-level: high-level
capability-under-test: ui-surface-coverage
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.form,ui.route,ui.state
user-story-id: option-map-story
user-story: As a reviewer I can inspect option-map behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-21.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: option-map-live-codex-requirements-21 exercises docs/ail/prompts/requirements.system.md over ui-surface-coverage.
v0.3-signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-21.json
response-file: responses/example-21.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-22
semantic-task: option-map-live-codex-spec-draft-22
profile: Application
surface-tags: ui
package: examples/option_map.ail
use-case: Small transform used to exercise typed option mapping and UI-tagged prompt surfaces.
capability-level: high-level
capability-under-test: ui-surface-coverage
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.form,ui.route,ui.state
user-story-id: option-map-story
user-story: As a reviewer I can inspect option-map behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-22.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: option-map-live-codex-spec-draft-22 exercises docs/ail/prompts/spec-draft.system.md over ui-surface-coverage.
v0.3-signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-22.json
response-file: responses/example-22.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-23
semantic-task: option-map-live-codex-core-draft-23
profile: Application
surface-tags: ui
package: examples/option_map.ail
use-case: Small transform used to exercise typed option mapping and UI-tagged prompt surfaces.
capability-level: high-level
capability-under-test: ui-surface-coverage
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.form,ui.route,ui.state
user-story-id: option-map-story
user-story: As a reviewer I can inspect option-map behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-23.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: option-map-live-codex-core-draft-23 exercises docs/ail/prompts/core-draft.system.md over ui-surface-coverage.
v0.3-signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-23.json
response-file: responses/example-23.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-24
semantic-task: option-map-live-codex-diagnostic-repair-24
profile: Application
surface-tags: ui
package: examples/option_map.ail
use-case: Small transform used to exercise typed option mapping and UI-tagged prompt surfaces.
capability-level: high-level
capability-under-test: ui-surface-coverage
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.form,ui.route,ui.state
user-story-id: option-map-story
user-story: As a reviewer I can inspect option-map behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: checked-core
story-file: stories/example-24.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: option-map-live-codex-diagnostic-repair-24 exercises docs/ail/prompts/diagnostic-repair.system.md over ui-surface-coverage.
v0.3-signal: UI examples need richer package-local walkthroughs and stricter semantic tagging.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-24.json
response-file: responses/example-24.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
## Example: example-25
semantic-task: c-interop-live-codex-core-to-spec-25
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-25.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-core-to-spec-25 exercises docs/ail/prompts/core-to-spec.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-25.json
response-file: responses/example-25.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-26
semantic-task: c-interop-live-codex-core-to-summary-26
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-26.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-core-to-summary-26 exercises docs/ail/prompts/core-to-summary.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-26.json
response-file: responses/example-26.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-27
semantic-task: c-interop-live-codex-flow-patch-27
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-27.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-flow-patch-27 exercises docs/ail/prompts/flow-patch.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-27.json
response-file: responses/example-27.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-28
semantic-task: c-interop-live-codex-trace-debug-28
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-28.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-trace-debug-28 exercises docs/ail/prompts/trace-debug.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-28.json
response-file: responses/example-28.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-29
semantic-task: c-interop-live-codex-interop-29
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-29.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-interop-29 exercises docs/ail/prompts/interop.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-29.json
response-file: responses/example-29.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-30
semantic-task: support-ticket-live-codex-interview-30
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-30.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-interview-30 exercises docs/ail/prompts/interview.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-30.json
response-file: responses/example-30.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-31
semantic-task: support-ticket-live-codex-requirements-31
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-31.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-requirements-31 exercises docs/ail/prompts/requirements.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-31.json
response-file: responses/example-31.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-32
semantic-task: support-ticket-live-spec-input-32
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-32.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-spec-input-32 exercises docs/ail/prompts/spec-draft.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: llm-http
executor-label: unsloth-qwen3.6-35b-a3b-gguf-chat
capture-origin: live-llm
request-file: requests/example-32.json
response-file: responses/example-32.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: inteligentia-pro-1-qwen3.6-35b-chat
## Example: example-33
semantic-task: support-ticket-live-codex-core-draft-33
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-33.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-core-draft-33 exercises docs/ail/prompts/core-draft.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-33.json
response-file: responses/example-33.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-34
semantic-task: support-ticket-live-codex-diagnostic-repair-34
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-34.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-diagnostic-repair-34 exercises docs/ail/prompts/diagnostic-repair.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-34.json
response-file: responses/example-34.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-35
semantic-task: runtime-generic-live-codex-core-to-spec-35
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
use-case: Runtime generic value flow through typed actions and traceable outcomes.
capability-level: mid-level
capability-under-test: runtime-generics
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: runtime-generic-story
user-story: As a reviewer I can inspect runtime-generic behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-35.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: runtime-generic-live-codex-core-to-spec-35 exercises docs/ail/prompts/core-to-spec.system.md over runtime-generics.
v0.3-signal: Generic runtime behavior needs clearer type-inference explanations.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-35.json
response-file: responses/example-35.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: PrioritizeTicket
runtime-state: ticket.id=T-1;ticket.priority=Low
## Example: example-36
semantic-task: runtime-generic-live-codex-core-to-summary-36
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
use-case: Runtime generic value flow through typed actions and traceable outcomes.
capability-level: mid-level
capability-under-test: runtime-generics
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: runtime-generic-story
user-story: As a reviewer I can inspect runtime-generic behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-36.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: runtime-generic-live-codex-core-to-summary-36 exercises docs/ail/prompts/core-to-summary.system.md over runtime-generics.
v0.3-signal: Generic runtime behavior needs clearer type-inference explanations.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-36.json
response-file: responses/example-36.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: PrioritizeTicket
runtime-state: ticket.id=T-1;ticket.priority=Low
## Example: example-37
semantic-task: runtime-generic-live-codex-flow-patch-37
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
use-case: Runtime generic value flow through typed actions and traceable outcomes.
capability-level: mid-level
capability-under-test: runtime-generics
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: runtime-generic-story
user-story: As a reviewer I can inspect runtime-generic behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-37.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: runtime-generic-live-codex-flow-patch-37 exercises docs/ail/prompts/flow-patch.system.md over runtime-generics.
v0.3-signal: Generic runtime behavior needs clearer type-inference explanations.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-37.json
response-file: responses/example-37.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: PrioritizeTicket
runtime-state: ticket.id=T-1;ticket.priority=Low
## Example: example-38
semantic-task: runtime-generic-live-codex-trace-debug-38
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
use-case: Runtime generic value flow through typed actions and traceable outcomes.
capability-level: mid-level
capability-under-test: runtime-generics
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: runtime-generic-story
user-story: As a reviewer I can inspect runtime-generic behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-38.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: runtime-generic-live-codex-trace-debug-38 exercises docs/ail/prompts/trace-debug.system.md over runtime-generics.
v0.3-signal: Generic runtime behavior needs clearer type-inference explanations.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-38.json
response-file: responses/example-38.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: PrioritizeTicket
runtime-state: ticket.id=T-1;ticket.priority=Low
## Example: example-39
semantic-task: runtime-generic-live-codex-interop-39
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
use-case: Runtime generic value flow through typed actions and traceable outcomes.
capability-level: mid-level
capability-under-test: runtime-generics
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: runtime-generic-story
user-story: As a reviewer I can inspect runtime-generic behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-39.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: runtime-generic-live-codex-interop-39 exercises docs/ail/prompts/interop.system.md over runtime-generics.
v0.3-signal: Generic runtime behavior needs clearer type-inference explanations.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-39.json
response-file: responses/example-39.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: PrioritizeTicket
runtime-state: ticket.id=T-1;ticket.priority=Low
## Example: example-40
semantic-task: refund-tool-live-codex-interview-40
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-40.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-interview-40 exercises docs/ail/prompts/interview.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-40.json
response-file: responses/example-40.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-41
semantic-task: refund-tool-live-codex-requirements-41
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-41.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-requirements-41 exercises docs/ail/prompts/requirements.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-41.json
response-file: responses/example-41.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-42
semantic-task: refund-tool-live-codex-spec-draft-42
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-42.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-spec-draft-42 exercises docs/ail/prompts/spec-draft.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-42.json
response-file: responses/example-42.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-43
semantic-task: refund-tool-live-codex-core-draft-43
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-43.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-core-draft-43 exercises docs/ail/prompts/core-draft.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-43.json
response-file: responses/example-43.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-44
semantic-task: refund-tool-live-codex-diagnostic-repair-44
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-44.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-diagnostic-repair-44 exercises docs/ail/prompts/diagnostic-repair.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-44.json
response-file: responses/example-44.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-45
semantic-task: refund-tool-live-codex-core-to-spec-45
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-45.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-core-to-spec-45 exercises docs/ail/prompts/core-to-spec.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-45.json
response-file: responses/example-45.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-46
semantic-task: refund-tool-live-codex-core-to-summary-46
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-46.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-core-to-summary-46 exercises docs/ail/prompts/core-to-summary.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-46.json
response-file: responses/example-46.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-47
semantic-task: refund-tool-live-codex-flow-patch-47
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-47.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-flow-patch-47 exercises docs/ail/prompts/flow-patch.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-47.json
response-file: responses/example-47.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-48
semantic-task: refund-tool-live-codex-trace-debug-48
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-48.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-trace-debug-48 exercises docs/ail/prompts/trace-debug.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-48.json
response-file: responses/example-48.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-49
semantic-task: refund-tool-live-codex-interop-49
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-49.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-interop-49 exercises docs/ail/prompts/interop.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-49.json
response-file: responses/example-49.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-50
semantic-task: refund-tool-live-codex-interview-50
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-50.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-interview-50 exercises docs/ail/prompts/interview.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-50.json
response-file: responses/example-50.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-51
semantic-task: refund-tool-live-codex-requirements-51
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-51.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-requirements-51 exercises docs/ail/prompts/requirements.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-51.json
response-file: responses/example-51.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-52
semantic-task: refund-tool-live-spec-input-52
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-52.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-spec-input-52 exercises docs/ail/prompts/spec-draft.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: llm-http
executor-label: unsloth-qwen3.6-35b-a3b-gguf-chat
capture-origin: live-llm
request-file: requests/example-52.json
response-file: responses/example-52.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
endpoint-label: inteligentia-pro-1-qwen3.6-35b-chat
## Example: example-53
semantic-task: refund-tool-live-codex-core-draft-53
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-53.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-core-draft-53 exercises docs/ail/prompts/core-draft.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-53.json
response-file: responses/example-53.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-54
semantic-task: refund-tool-live-codex-diagnostic-repair-54
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
use-case: Agent tool for payment refund approval with permissions and capability checks.
capability-level: high-level
capability-under-test: agent-tool-safety
program-scale: multi-module-system
program-domain: agent-tool
module-count: 3
spec-count: 3
story-count: 3
interacts-with: payment.provider,policy.engine,audit.log
user-story-id: refund-tool-story
user-story: As a reviewer I can inspect refund-tool behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-54.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: refund-tool-live-codex-diagnostic-repair-54 exercises docs/ail/prompts/diagnostic-repair.system.md over agent-tool-safety.
v0.3-signal: AgentTool examples need multi-agent handoff and policy-review exercises.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-54.json
response-file: responses/example-54.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
## Example: example-55
semantic-task: compiler-pass-live-codex-core-to-spec-55
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-55.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-core-to-spec-55 exercises docs/ail/prompts/core-to-spec.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-55.json
response-file: responses/example-55.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-56
semantic-task: compiler-pass-live-codex-core-to-summary-56
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-56.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-core-to-summary-56 exercises docs/ail/prompts/core-to-summary.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-56.json
response-file: responses/example-56.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-57
semantic-task: compiler-pass-live-codex-flow-patch-57
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-57.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-flow-patch-57 exercises docs/ail/prompts/flow-patch.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-57.json
response-file: responses/example-57.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-58
semantic-task: compiler-pass-live-codex-trace-debug-58
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-58.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-trace-debug-58 exercises docs/ail/prompts/trace-debug.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-58.json
response-file: responses/example-58.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-59
semantic-task: compiler-pass-live-codex-interop-59
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-59.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-interop-59 exercises docs/ail/prompts/interop.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-59.json
response-file: responses/example-59.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-60
semantic-task: compiler-pass-live-codex-interview-60
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-60.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-interview-60 exercises docs/ail/prompts/interview.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-60.json
response-file: responses/example-60.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-61
semantic-task: compiler-pass-live-codex-requirements-61
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-61.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-requirements-61 exercises docs/ail/prompts/requirements.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-61.json
response-file: responses/example-61.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-62
semantic-task: compiler-pass-live-codex-spec-draft-62
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-62.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-spec-draft-62 exercises docs/ail/prompts/spec-draft.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-62.json
response-file: responses/example-62.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-63
semantic-task: compiler-pass-live-codex-core-draft-63
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-63.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-core-draft-63 exercises docs/ail/prompts/core-draft.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-63.json
response-file: responses/example-63.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-64
semantic-task: compiler-pass-live-codex-diagnostic-repair-64
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
use-case: Compiler pass semantics that transform AIL-Core with checked traces.
capability-level: low-level
capability-under-test: compiler-pass
program-scale: utility
program-domain: compiler
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: compiler-pass-story
user-story: As a reviewer I can inspect compiler-pass behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-64.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: compiler-pass-live-codex-diagnostic-repair-64 exercises docs/ail/prompts/diagnostic-repair.system.md over compiler-pass.
v0.3-signal: Self-hosting needs pass-composition examples and fixed-point checks.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-64.json
response-file: responses/example-64.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: InferReadPermissions
## Example: example-65
semantic-task: ui-workflow-live-codex-core-to-spec-65
profile: UI
surface-tags: ui
package: examples/ui_workflow.ail
use-case: Accessible route, form, dashboard, and workflow semantics for a user-facing app.
capability-level: high-level
capability-under-test: ui-workflow
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.route,ui.form,ui.dashboard
user-story-id: ui-workflow-story
user-story: As a reviewer I can inspect ui-workflow behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-65.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: ui-workflow-live-codex-core-to-spec-65 exercises docs/ail/prompts/core-to-spec.system.md over ui-workflow.
v0.3-signal: UI authoring needs stronger visual review artifacts and accessibility exercises.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-65.json
response-file: responses/example-65.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CreateTicketForm
runtime-state: ticket.title=Bug
## Example: example-66
semantic-task: network-driver-live-codex-core-to-summary-66
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-66.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-core-to-summary-66 exercises docs/ail/prompts/core-to-summary.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-66.json
response-file: responses/example-66.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-67
semantic-task: network-driver-live-codex-flow-patch-67
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-67.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-flow-patch-67 exercises docs/ail/prompts/flow-patch.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-67.json
response-file: responses/example-67.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-68
semantic-task: network-driver-live-codex-trace-debug-68
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-68.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-trace-debug-68 exercises docs/ail/prompts/trace-debug.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-68.json
response-file: responses/example-68.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-69
semantic-task: network-driver-live-codex-interop-69
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-69.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-interop-69 exercises docs/ail/prompts/interop.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-69.json
response-file: responses/example-69.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-70
semantic-task: network-driver-live-codex-interview-70
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-70.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-interview-70 exercises docs/ail/prompts/interview.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-70.json
response-file: responses/example-70.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-71
semantic-task: network-driver-live-codex-requirements-71
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-71.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-requirements-71 exercises docs/ail/prompts/requirements.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-71.json
response-file: responses/example-71.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-72
semantic-task: network-driver-live-codex-spec-draft-72
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-72.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-spec-draft-72 exercises docs/ail/prompts/spec-draft.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-72.json
response-file: responses/example-72.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-73
semantic-task: network-driver-live-codex-core-draft-73
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-73.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-core-draft-73 exercises docs/ail/prompts/core-draft.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-73.json
response-file: responses/example-73.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-74
semantic-task: network-driver-live-codex-diagnostic-repair-74
profile: System
surface-tags: core
package: examples/network_driver.ail
use-case: System-level network driver boundary with effects, capabilities, and packets.
capability-level: low-level
capability-under-test: system-driver
program-scale: utility
program-domain: system-driver
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: network-driver-story
user-story: As a reviewer I can inspect network-driver behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-74.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: network-driver-live-codex-diagnostic-repair-74 exercises docs/ail/prompts/diagnostic-repair.system.md over system-driver.
v0.3-signal: Systems profile needs hardware-facing contracts and scheduler/interrupt examples.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-74.json
response-file: responses/example-74.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
## Example: example-75
semantic-task: secret-access-live-codex-core-to-spec-75
profile: System
surface-tags: core
package: examples/secret_access.ail
use-case: Secret and permission semantics for guarded internal data access.
capability-level: mid-level
capability-under-test: security-permissions
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: secret-access-story
user-story: As a reviewer I can inspect secret-access behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-75.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: secret-access-live-codex-core-to-spec-75 exercises docs/ail/prompts/core-to-spec.system.md over security-permissions.
v0.3-signal: Security examples need threat-model annotations and audit trails.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-75.json
response-file: responses/example-75.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
## Example: example-76
semantic-task: secret-access-live-codex-core-to-summary-76
profile: System
surface-tags: core
package: examples/secret_access.ail
use-case: Secret and permission semantics for guarded internal data access.
capability-level: mid-level
capability-under-test: security-permissions
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: secret-access-story
user-story: As a reviewer I can inspect secret-access behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-76.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: secret-access-live-codex-core-to-summary-76 exercises docs/ail/prompts/core-to-summary.system.md over security-permissions.
v0.3-signal: Security examples need threat-model annotations and audit trails.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-76.json
response-file: responses/example-76.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
## Example: example-77
semantic-task: secret-access-live-codex-flow-patch-77
profile: System
surface-tags: core
package: examples/secret_access.ail
use-case: Secret and permission semantics for guarded internal data access.
capability-level: mid-level
capability-under-test: security-permissions
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: secret-access-story
user-story: As a reviewer I can inspect secret-access behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-77.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: secret-access-live-codex-flow-patch-77 exercises docs/ail/prompts/flow-patch.system.md over security-permissions.
v0.3-signal: Security examples need threat-model annotations and audit trails.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-77.json
response-file: responses/example-77.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
## Example: example-78
semantic-task: secret-access-live-codex-trace-debug-78
profile: System
surface-tags: core
package: examples/secret_access.ail
use-case: Secret and permission semantics for guarded internal data access.
capability-level: mid-level
capability-under-test: security-permissions
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: secret-access-story
user-story: As a reviewer I can inspect secret-access behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-78.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: secret-access-live-codex-trace-debug-78 exercises docs/ail/prompts/trace-debug.system.md over security-permissions.
v0.3-signal: Security examples need threat-model annotations and audit trails.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-78.json
response-file: responses/example-78.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
## Example: example-79
semantic-task: secret-access-live-codex-interop-79
profile: System
surface-tags: core
package: examples/secret_access.ail
use-case: Secret and permission semantics for guarded internal data access.
capability-level: mid-level
capability-under-test: security-permissions
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: secret-access-story
user-story: As a reviewer I can inspect secret-access behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-79.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: secret-access-live-codex-interop-79 exercises docs/ail/prompts/interop.system.md over security-permissions.
v0.3-signal: Security examples need threat-model annotations and audit trails.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-79.json
response-file: responses/example-79.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
## Example: example-80
semantic-task: repeated-task-live-codex-interview-80
profile: System
surface-tags: core
package: examples/repeated_task.ail
use-case: Scheduled repeated maintenance workflow with stateful trace evidence.
capability-level: high-level
capability-under-test: scheduled-workflow
program-scale: multi-module-system
program-domain: application
module-count: 3
spec-count: 3
story-count: 3
interacts-with: scheduler,task.store,audit.log
user-story-id: repeated-task-story
user-story: As a reviewer I can inspect repeated-task behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-80.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: repeated-task-live-codex-interview-80 exercises docs/ail/prompts/interview.system.md over scheduled-workflow.
v0.3-signal: Workflow examples need temporal policies and retry/backoff semantics.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-80.json
response-file: responses/example-80.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
## Example: example-81
semantic-task: repeated-task-live-codex-requirements-81
profile: System
surface-tags: core
package: examples/repeated_task.ail
use-case: Scheduled repeated maintenance workflow with stateful trace evidence.
capability-level: high-level
capability-under-test: scheduled-workflow
program-scale: multi-module-system
program-domain: application
module-count: 3
spec-count: 3
story-count: 3
interacts-with: scheduler,task.store,audit.log
user-story-id: repeated-task-story
user-story: As a reviewer I can inspect repeated-task behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-81.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: repeated-task-live-codex-requirements-81 exercises docs/ail/prompts/requirements.system.md over scheduled-workflow.
v0.3-signal: Workflow examples need temporal policies and retry/backoff semantics.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-81.json
response-file: responses/example-81.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
## Example: example-82
semantic-task: repeated-task-live-codex-spec-draft-82
profile: System
surface-tags: core
package: examples/repeated_task.ail
use-case: Scheduled repeated maintenance workflow with stateful trace evidence.
capability-level: high-level
capability-under-test: scheduled-workflow
program-scale: multi-module-system
program-domain: application
module-count: 3
spec-count: 3
story-count: 3
interacts-with: scheduler,task.store,audit.log
user-story-id: repeated-task-story
user-story: As a reviewer I can inspect repeated-task behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-82.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: repeated-task-live-codex-spec-draft-82 exercises docs/ail/prompts/spec-draft.system.md over scheduled-workflow.
v0.3-signal: Workflow examples need temporal policies and retry/backoff semantics.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-82.json
response-file: responses/example-82.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
## Example: example-83
semantic-task: repeated-task-live-codex-core-draft-83
profile: System
surface-tags: core
package: examples/repeated_task.ail
use-case: Scheduled repeated maintenance workflow with stateful trace evidence.
capability-level: high-level
capability-under-test: scheduled-workflow
program-scale: multi-module-system
program-domain: application
module-count: 3
spec-count: 3
story-count: 3
interacts-with: scheduler,task.store,audit.log
user-story-id: repeated-task-story
user-story: As a reviewer I can inspect repeated-task behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-83.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: repeated-task-live-codex-core-draft-83 exercises docs/ail/prompts/core-draft.system.md over scheduled-workflow.
v0.3-signal: Workflow examples need temporal policies and retry/backoff semantics.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-83.json
response-file: responses/example-83.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
## Example: example-84
semantic-task: repeated-task-live-codex-diagnostic-repair-84
profile: System
surface-tags: core
package: examples/repeated_task.ail
use-case: Scheduled repeated maintenance workflow with stateful trace evidence.
capability-level: high-level
capability-under-test: scheduled-workflow
program-scale: multi-module-system
program-domain: application
module-count: 3
spec-count: 3
story-count: 3
interacts-with: scheduler,task.store,audit.log
user-story-id: repeated-task-story
user-story: As a reviewer I can inspect repeated-task behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-84.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: repeated-task-live-codex-diagnostic-repair-84 exercises docs/ail/prompts/diagnostic-repair.system.md over scheduled-workflow.
v0.3-signal: Workflow examples need temporal policies and retry/backoff semantics.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-84.json
response-file: responses/example-84.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
## Example: example-85
semantic-task: c-interop-live-codex-core-to-spec-85
profile: System
surface-tags: core
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-85.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-core-to-spec-85 exercises docs/ail/prompts/core-to-spec.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-85.json
response-file: responses/example-85.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-86
semantic-task: c-interop-live-codex-core-to-summary-86
profile: System
surface-tags: core
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-86.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-core-to-summary-86 exercises docs/ail/prompts/core-to-summary.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-86.json
response-file: responses/example-86.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-87
semantic-task: c-interop-live-codex-flow-patch-87
profile: System
surface-tags: core
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-87.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-flow-patch-87 exercises docs/ail/prompts/flow-patch.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-87.json
response-file: responses/example-87.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-88
semantic-task: c-interop-live-codex-trace-debug-88
profile: System
surface-tags: core
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-88.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-trace-debug-88 exercises docs/ail/prompts/trace-debug.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-88.json
response-file: responses/example-88.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-89
semantic-task: c-interop-live-codex-interop-89
profile: System
surface-tags: core
package: examples/c_interop.ail
use-case: Checked C and host interop with ABI, ownership, status, and trace contracts.
capability-level: low-level
capability-under-test: c-host-interop
program-scale: utility
program-domain: c-interop
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: c-interop-story
user-story: As a reviewer I can inspect c-interop behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-89.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: c-interop-live-codex-interop-89 exercises docs/ail/prompts/interop.system.md over c-host-interop.
v0.3-signal: Interop needs deeper unsafe-boundary tutorials and more ABI fixture diversity.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-89.json
response-file: responses/example-89.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
## Example: example-90
semantic-task: support-ticket-live-codex-interview-90
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-90.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-interview-90 exercises docs/ail/prompts/interview.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-90.json
response-file: responses/example-90.json
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-91
semantic-task: support-ticket-live-codex-requirements-91
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-91.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-requirements-91 exercises docs/ail/prompts/requirements.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-91.json
response-file: responses/example-91.json
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-92
semantic-task: support-ticket-live-codex-spec-92
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-92.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-spec-92 exercises docs/ail/prompts/spec-draft.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-92.json
response-file: responses/example-92.json
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-93
semantic-task: support-ticket-live-codex-core-draft-93
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-93.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-core-draft-93 exercises docs/ail/prompts/core-draft.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-93.json
response-file: responses/example-93.json
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-94
semantic-task: support-ticket-live-codex-diagnostic-repair-94
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Application workflow for support-ticket actions, permissions, failures, and traces.
capability-level: high-level
capability-under-test: application-workflow
program-scale: multi-module-system
program-domain: os-utility
module-count: 3
spec-count: 3
story-count: 3
interacts-with: libsystem,elf-loader,wasm-sandbox
user-story-id: support-ticket-story
user-story: As a reviewer I can inspect support-ticket behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-94.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: support-ticket-live-codex-diagnostic-repair-94 exercises docs/ail/prompts/diagnostic-repair.system.md over application-workflow.
v0.3-signal: Application examples need user-story walkthroughs from intent to runtime trace.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-94.json
response-file: responses/example-94.json
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
## Example: example-95
semantic-task: stateful-counter-live-codex-core-to-spec-95
profile: System
surface-tags: core
package: examples/stateful_counter.ail
use-case: Minimal state mutation that proves deterministic VM/native behavior.
capability-level: mid-level
capability-under-test: stateful-runtime
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: stateful-counter-story
user-story: As a reviewer I can inspect stateful-counter behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-95.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: stateful-counter-live-codex-core-to-spec-95 exercises docs/ail/prompts/core-to-spec.system.md over stateful-runtime.
v0.3-signal: State examples need clearer persistence and concurrency boundaries.
prompt-file: docs/ail/prompts/core-to-spec.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:9f447e07620792b2
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-95.json
response-file: responses/example-95.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
## Example: example-96
semantic-task: stateful-counter-live-codex-core-to-summary-96
profile: System
surface-tags: core
package: examples/stateful_counter.ail
use-case: Minimal state mutation that proves deterministic VM/native behavior.
capability-level: mid-level
capability-under-test: stateful-runtime
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: stateful-counter-story
user-story: As a reviewer I can inspect stateful-counter behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-96.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: stateful-counter-live-codex-core-to-summary-96 exercises docs/ail/prompts/core-to-summary.system.md over stateful-runtime.
v0.3-signal: State examples need clearer persistence and concurrency boundaries.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-96.json
response-file: responses/example-96.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
## Example: example-97
semantic-task: stateful-counter-live-codex-flow-patch-97
profile: System
surface-tags: core
package: examples/stateful_counter.ail
use-case: Minimal state mutation that proves deterministic VM/native behavior.
capability-level: mid-level
capability-under-test: stateful-runtime
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: stateful-counter-story
user-story: As a reviewer I can inspect stateful-counter behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-97.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stateful-counter-live-codex-flow-patch-97 exercises docs/ail/prompts/flow-patch.system.md over stateful-runtime.
v0.3-signal: State examples need clearer persistence and concurrency boundaries.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-97.json
response-file: responses/example-97.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
## Example: example-98
semantic-task: stateful-counter-live-codex-trace-debug-98
profile: System
surface-tags: core
package: examples/stateful_counter.ail
use-case: Minimal state mutation that proves deterministic VM/native behavior.
capability-level: mid-level
capability-under-test: stateful-runtime
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: stateful-counter-story
user-story: As a reviewer I can inspect stateful-counter behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-98.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stateful-counter-live-codex-trace-debug-98 exercises docs/ail/prompts/trace-debug.system.md over stateful-runtime.
v0.3-signal: State examples need clearer persistence and concurrency boundaries.
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-98.json
response-file: responses/example-98.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
## Example: example-99
semantic-task: support-ticket-live-codex-rejected-99
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Rejected semantic-drift case used to verify diagnostic teaching coverage.
capability-level: high-level
capability-under-test: diagnostic-semantic-drift
program-scale: module
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: semantic-drift-story
user-story: As a reviewer I can inspect the semantic-drift diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-99.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: support-ticket-live-codex-rejected-99 exercises docs/ail/prompts/interop.system.md over diagnostic-semantic-drift.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-99.json
response-file: responses/example-99.json
artifact-kind: ail-spec
checker-result: rejected
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
expected-diagnostic: AIL001
failure-taxonomy: semantic-drift
## Example: example-100
semantic-task: stateful-counter-live-codex-accepted-100
profile: System
surface-tags: core
package: examples/stateful_counter.ail
use-case: Minimal state mutation that proves deterministic VM/native behavior.
capability-level: mid-level
capability-under-test: stateful-runtime
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: stateful-counter-story
user-story: As a reviewer I can inspect stateful-counter behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: vm-trace
story-file: stories/example-100.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stateful-counter-live-codex-accepted-100 exercises docs/ail/prompts/interop.system.md over stateful-runtime.
v0.3-signal: State examples need clearer persistence and concurrency boundaries.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-100.json
response-file: responses/example-100.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
## Example: example-101
semantic-task: support-ticket-profile-mismatch-rejected-101
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Rejected profile-mismatch case used to verify diagnostic teaching coverage.
capability-level: high-level
capability-under-test: diagnostic-profile-mismatch
program-scale: module
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: profile-mismatch-story
user-story: As a reviewer I can inspect the profile-mismatch diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-101.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: support-ticket-profile-mismatch-rejected-101 exercises docs/ail/prompts/spec-draft.system.md over diagnostic-profile-mismatch.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-101.json
response-file: responses/example-101.json
artifact-kind: prompt-envelope
checker-result: rejected
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
expected-diagnostic: AIL-PROMPT-001
failure-taxonomy: profile-mismatch
## Example: example-102
semantic-task: support-ticket-missing-trace-rejected-102
profile: System
surface-tags: core
package: examples/support_ticket.ail
use-case: Rejected missing-trace case used to verify diagnostic teaching coverage.
capability-level: high-level
capability-under-test: diagnostic-missing-trace
program-scale: module
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: missing-trace-story
user-story: As a reviewer I can inspect the missing-trace diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-102.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: support-ticket-missing-trace-rejected-102 exercises docs/ail/prompts/spec-draft.system.md over diagnostic-missing-trace.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-102.json
response-file: responses/example-102.json
artifact-kind: ail-spec
checker-result: rejected
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
expected-diagnostic: AIL-TRACE-001
failure-taxonomy: missing-trace
## Example: example-103
semantic-task: refund-tool-hallucinated-capability-rejected-103
profile: AgentTool
surface-tags: tool,capability
package: examples/refund_tool.ail
use-case: Rejected hallucinated-capability case used to verify diagnostic teaching coverage.
capability-level: high-level
capability-under-test: diagnostic-hallucinated-capability
program-scale: module
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: hallucinated-capability-story
user-story: As a reviewer I can inspect the hallucinated-capability diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-103.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: refund-tool-hallucinated-capability-rejected-103 exercises docs/ail/prompts/diagnostic-repair.system.md over diagnostic-hallucinated-capability.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-103.json
response-file: responses/example-103.json
artifact-kind: ail-spec
checker-result: rejected
target: vm
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;refund.amount=750
expected-diagnostic: AIL019
failure-taxonomy: hallucinated-capability
## Example: example-104
semantic-task: system-linux-syscall-darwin-unsupported-104
profile: System
surface-tags: system,backend
package: examples/darwin_linux_effect.ail
use-case: Rejected unsupported-target case used to verify diagnostic teaching coverage.
capability-level: low-level
capability-under-test: diagnostic-unsupported-target
program-scale: utility
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: unsupported-target-story
user-story: As a reviewer I can inspect the unsupported-target diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-104.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: system-linux-syscall-darwin-unsupported-104 exercises docs/ail/prompts/spec-draft.system.md over diagnostic-unsupported-target.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-104.json
response-file: responses/example-104.json
artifact-kind: ail-spec
checker-result: rejected
target: aarch64-apple-darwin-libsystem-macho
vm-action: LinuxExit
runtime-state: system.mode=test
expected-diagnostic: AIL-BACKEND-001
failure-taxonomy: unsupported-target
## Example: example-105
semantic-task: c-interop-nullable-nonnull-rejected-105
profile: System
surface-tags: c-interop,ffi
package: examples/c_interop.ail
use-case: Rejected invalid-interop case used to verify diagnostic teaching coverage.
capability-level: low-level
capability-under-test: diagnostic-invalid-interop
program-scale: utility
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: invalid-interop-story
user-story: As a reviewer I can inspect the invalid-interop diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-105.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: c-interop-nullable-nonnull-rejected-105 exercises docs/ail/prompts/interop.system.md over diagnostic-invalid-interop.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-105.json
response-file: responses/example-105.json
artifact-kind: ail-spec
checker-result: rejected
target: wasm32-unknown-sandbox-wasm
vm-action: strlen
runtime-state: text=null
expected-diagnostic: AIL-FFI-NULL-001
failure-taxonomy: invalid-interop
## Example: example-106
semantic-task: network-driver-effect-without-capability-rejected-106
profile: System
surface-tags: system,capability
package: examples/network_driver.ail
use-case: Rejected permission-capability case used to verify diagnostic teaching coverage.
capability-level: low-level
capability-under-test: diagnostic-permission-capability
program-scale: utility
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: permission-capability-story
user-story: As a reviewer I can inspect the permission-capability diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-106.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: network-driver-effect-without-capability-rejected-106 exercises docs/ail/prompts/spec-draft.system.md over diagnostic-permission-capability.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-106.json
response-file: responses/example-106.json
artifact-kind: ail-spec
checker-result: rejected
target: linux-x86_64-elf
vm-action: NetworkPacketReceiver
runtime-state: network.device=eth0;rx.buffer=empty
expected-diagnostic: AIL021
failure-taxonomy: permission-capability
## Example: example-107
semantic-task: package-registry-missing-import-rejected-107
profile: Application
surface-tags: package-import,registry
package: examples/missing_registry_import.ail
use-case: Rejected package-resolution case used to verify diagnostic teaching coverage.
capability-level: mid-level
capability-under-test: diagnostic-package-resolution
program-scale: module
program-domain: diagnostic
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: package-resolution-story
user-story: As a reviewer I can inspect the package-resolution diagnostic so that repair preserves the intended behavior.
acceptance-criteria: expected diagnostic exists; diagnostic artifact exists; repair target remains reviewable
story-evidence: diagnostics
story-file: stories/example-107.md
story-journey: diagnostic-story
story-roundtrip: diagnostic-preserving
distinctness-claim: package-registry-missing-import-rejected-107 exercises docs/ail/prompts/requirements.system.md over diagnostic-package-resolution.
v0.3-signal: Rejected examples need repair tutorials that convert diagnostics into corrected specs.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-107.json
response-file: responses/example-107.json
artifact-kind: ail-spec
checker-result: rejected
target: vm
vm-action: ResolveSharedImport
runtime-state: registry.index=missing-shared-lib
expected-diagnostic: AIL registry import shared-lib as Shared was not found in registry index
failure-taxonomy: package-resolution
## Example: example-108
semantic-task: ui-workflow-live-codex-spec-draft-108
profile: UI
surface-tags: ui
package: examples/ui_workflow.ail
use-case: Accessible route, form, dashboard, and workflow semantics for a user-facing app.
capability-level: high-level
capability-under-test: ui-workflow
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.route,ui.form,ui.dashboard
user-story-id: ui-workflow-story
user-story: As a reviewer I can inspect ui-workflow behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-108.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: ui-workflow-live-codex-spec-draft-108 exercises docs/ail/prompts/spec-draft.system.md over ui-workflow.
v0.3-signal: UI authoring needs stronger visual review artifacts and accessibility exercises.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-108.json
response-file: responses/example-108.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CreateTicketForm
runtime-state: ticket.title=Incident 108
## Example: example-109
semantic-task: ui-workflow-live-codex-requirements-109
profile: UI
surface-tags: ui
package: examples/ui_workflow.ail
use-case: Accessible route, form, dashboard, and workflow semantics for a user-facing app.
capability-level: high-level
capability-under-test: ui-workflow
program-scale: multi-module-system
program-domain: ui-workflow
module-count: 3
spec-count: 3
story-count: 3
interacts-with: ui.route,ui.form,ui.dashboard
user-story-id: ui-workflow-story
user-story: As a reviewer I can inspect ui-workflow behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-109.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: ui-workflow-live-codex-requirements-109 exercises docs/ail/prompts/requirements.system.md over ui-workflow.
v0.3-signal: UI authoring needs stronger visual review artifacts and accessibility exercises.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-109.json
response-file: responses/example-109.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CreateTicketForm
runtime-state: ticket.title=Incident 109
## Example: example-110
semantic-task: stateful-counter-live-codex-repair-110
profile: System
surface-tags: core,repair
package: examples/stateful_counter.ail
use-case: Minimal state mutation that proves deterministic VM/native behavior.
capability-level: mid-level
capability-under-test: stateful-runtime
program-scale: module
program-domain: runtime
module-count: 1
spec-count: 1
story-count: 1
interacts-with: none
user-story-id: stateful-counter-story
user-story: As a reviewer I can inspect stateful-counter behavior so that regenerated user stories remain semantically similar to the checked spec.
acceptance-criteria: checked spec exists; checked core exists; bytecode exists; runtime or target evidence exists
story-evidence: target-report
story-file: stories/example-110.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: stateful-counter-live-codex-repair-110 exercises docs/ail/prompts/repair.system.md over stateful-runtime.
v0.3-signal: State examples need clearer persistence and concurrency boundaries.
prompt-file: docs/ail/prompts/repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:6d171ea7c34f3e31
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-110.json
response-file: responses/example-110.json
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: IncrementCounter
runtime-state: counter.value=110
## Example: example-111
semantic-task: incident-response-live-codex-111
profile: Application
surface-tags: application-workflow,ui,package-import,multi-module
package: examples/incident_response.ail
use-case: Multi-module incident declaration with identity, policy, notification, workflow, and command-center surfaces.
capability-level: high-level
capability-under-test: multi-module-incident-workflow
program-scale: multi-module-system
program-domain: application
module-count: 4
spec-count: 4
story-count: 5
interacts-with: incident_identity,incident_policy,incident_notifications,incident_response
user-story-id: incident-response-declare-story
user-story: As an incident commander I can review incident declaration across identity, policy, and notification modules so that intake behavior remains semantically similar to the checked spec.
acceptance-criteria: checked incident response spec exists; checked core exists; bytecode exists; target or VM evidence exists; user-story metadata matches catalog
story-evidence: vm-trace
story-file: stories/example-111.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: incident-response-live-codex-111 validates incident-response multi-module behavior through docs/ail/prompts/interview.system.md and vm evidence. It covers multi-module-incident-workflow.
v0.3-signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-111.json
response-file: responses/example-111.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: DeclareIncident
runtime-state: incident.id=INC-1;incident.status=Declared;incident.severity=Sev1

## Example: example-112
semantic-task: incident-response-live-codex-112
profile: Application
surface-tags: application-workflow,ui,package-import,multi-module
package: examples/incident_response.ail
use-case: Incident escalation across policy review, responder notification, Wasm target contract, and private-note protection.
capability-level: high-level
capability-under-test: multi-module-incident-workflow
program-scale: multi-module-system
program-domain: application
module-count: 4
spec-count: 4
story-count: 5
interacts-with: incident_identity,incident_policy,incident_notifications,incident_response
user-story-id: incident-response-escalation-story
user-story: As an incident commander I can review escalation workflows that notify responders without exposing private notes.
acceptance-criteria: checked incident response spec exists; checked core exists; bytecode exists; target or VM evidence exists; user-story metadata matches catalog
story-evidence: target-report
story-file: stories/example-112.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: incident-response-live-codex-112 validates incident-response multi-module behavior through docs/ail/prompts/spec-draft.system.md and wasm32-unknown-sandbox-wasm evidence. It covers multi-module-incident-workflow.
v0.3-signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-112.json
response-file: responses/example-112.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: EscalateIncident
runtime-state: incident.id=INC-1;incident.status=Declared;incident.severity=Sev1

## Example: example-113
semantic-task: incident-response-live-codex-113
profile: Application
surface-tags: application-workflow,ui,package-import,multi-module
package: examples/incident_response.ail
use-case: Spec-to-story regeneration for a multi-feature incident lifecycle with VM trace evidence.
capability-level: high-level
capability-under-test: multi-module-incident-workflow
program-scale: multi-module-system
program-domain: application
module-count: 4
spec-count: 4
story-count: 5
interacts-with: incident_identity,incident_policy,incident_notifications,incident_response
user-story-id: incident-response-story-regeneration
user-story: As a service owner I can regenerate a user-story view from the checked incident response spec and keep the workflow meaning intact.
acceptance-criteria: checked incident response spec exists; checked core exists; bytecode exists; target or VM evidence exists; user-story metadata matches catalog
story-evidence: vm-trace
story-file: stories/example-113.md
story-journey: spec-to-story
story-roundtrip: semantic-similar
distinctness-claim: incident-response-live-codex-113 validates incident-response multi-module behavior through docs/ail/prompts/core-to-summary.system.md and vm evidence. It covers multi-module-incident-workflow.
v0.3-signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-113.json
response-file: responses/example-113.json
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CompleteIncidentLifecycle
runtime-state: incident.id=INC-1;incident.status=Declared;incident.severity=Sev1

## Example: example-114
semantic-task: incident-response-live-codex-114
profile: Application
surface-tags: application-workflow,ui,package-import,multi-module
package: examples/incident_response.ail
use-case: Story amendment for incident lifecycle behavior across Darwin target-contract evidence.
capability-level: high-level
capability-under-test: multi-module-incident-workflow
program-scale: multi-module-system
program-domain: application
module-count: 4
spec-count: 4
story-count: 5
interacts-with: incident_identity,incident_policy,incident_notifications,incident_response
user-story-id: incident-response-amendment-story
user-story: As a reviewer I can amend the incident lifecycle story and verify the spec still preserves escalation, notification, and postmortem steps.
acceptance-criteria: checked incident response spec exists; checked core exists; bytecode exists; target or VM evidence exists; user-story metadata matches catalog
story-evidence: target-report
story-file: stories/example-114.md
story-journey: story-amendment
story-roundtrip: semantic-similar
distinctness-claim: incident-response-live-codex-114 validates incident-response multi-module behavior through docs/ail/prompts/flow-patch.system.md and aarch64-apple-darwin-libsystem-macho evidence. It covers multi-module-incident-workflow.
v0.3-signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-114.json
response-file: responses/example-114.json
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: ResolveIncident
runtime-state: incident.id=INC-1;incident.status=Mitigating;incident.severity=Sev1

## Example: example-115
semantic-task: incident-response-live-codex-115
profile: Application
surface-tags: application-workflow,ui,package-import,multi-module
package: examples/incident_response.ail
use-case: Service-owner dashboard and command-center workflow checked against Wasm target-contract evidence.
capability-level: high-level
capability-under-test: multi-module-incident-workflow
program-scale: multi-module-system
program-domain: application
module-count: 4
spec-count: 4
story-count: 5
interacts-with: incident_identity,incident_policy,incident_notifications,incident_response
user-story-id: incident-response-dashboard-story
user-story: As a service owner I can inspect dashboard and command-center stories that coordinate multiple incident modules.
acceptance-criteria: checked incident response spec exists; checked core exists; bytecode exists; target or VM evidence exists; user-story metadata matches catalog
story-evidence: target-report
story-file: stories/example-115.md
story-journey: story-to-spec
story-roundtrip: semantic-similar
distinctness-claim: incident-response-live-codex-115 validates incident-response multi-module behavior through docs/ail/prompts/requirements.system.md and wasm32-unknown-sandbox-wasm evidence. It covers multi-module-incident-workflow.
v0.3-signal: Complex systems need richer story graphs that span imported modules, UI surfaces, workflows, target contracts, and regenerated story views.
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: codex-skill-agent
executor-label: codex-ail-spec-writer
capture-origin: live-codex
request-file: requests/example-115.json
response-file: responses/example-115.json
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: StartPostmortem
runtime-state: incident.id=INC-1;incident.status=Declared;incident.severity=Sev1
