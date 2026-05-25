# AIL v0.2 End-To-End Seed Corpus

This checked seed corpus stores deterministic prompt and response transcripts
for the `ail-e2e-corpus` release verifier.

## End-To-End Example: example-0
semantic-task: stdlib-collections-live-codex-interview-0
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-1
semantic-task: stdlib-collections-1
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-1.json
response-file: responses/example-1.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: vm
endpoint-label: local-endpoint-alt

## End-To-End Example: example-2
semantic-task: stdlib-collections-live-spec-input-2
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-3
semantic-task: stdlib-collections-live-codex-core-draft-3
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-4
semantic-task: stdlib-collections-live-codex-diagnostic-repair-4
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-5
semantic-task: stdlib-collections-live-codex-core-to-spec-5
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-6
semantic-task: stdlib-collections-live-codex-core-to-summary-6
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-7
semantic-task: stdlib-collections-live-codex-flow-patch-7
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-8
semantic-task: stdlib-collections-live-codex-trace-debug-8
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
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

## End-To-End Example: example-9
semantic-task: stdlib-collections-9
profile: Application
surface-tags: standard-library
package: examples/ail_std_collections.ail
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-9.json
response-file: responses/example-9.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: vm
endpoint-label: local-endpoint

## End-To-End Example: example-10
semantic-task: support-composed-live-codex-interview-10
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-11
semantic-task: support-composed-11
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-11.json
response-file: responses/example-11.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-12
semantic-task: support-composed-12
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-12.json
response-file: responses/example-12.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-13
semantic-task: support-composed-live-codex-core-draft-13
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-14
semantic-task: support-composed-live-codex-diagnostic-repair-14
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-15
semantic-task: support-composed-live-codex-core-to-spec-15
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-16
semantic-task: support-composed-live-codex-core-to-summary-16
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-17
semantic-task: support-composed-live-codex-flow-patch-17
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-18
semantic-task: support-composed-live-codex-trace-debug-18
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-19
semantic-task: support-composed-live-codex-interop-19
profile: Application
surface-tags: package-import
package: examples/support_composed.ail
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

## End-To-End Example: example-20
semantic-task: option-map-live-codex-interview-20
profile: Application
surface-tags: ui
package: examples/option_map.ail
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

## End-To-End Example: example-21
semantic-task: option-map-live-codex-requirements-21
profile: Application
surface-tags: ui
package: examples/option_map.ail
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

## End-To-End Example: example-22
semantic-task: option-map-live-codex-spec-draft-22
profile: Application
surface-tags: ui
package: examples/option_map.ail
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

## End-To-End Example: example-23
semantic-task: option-map-live-codex-core-draft-23
profile: Application
surface-tags: ui
package: examples/option_map.ail
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

## End-To-End Example: example-24
semantic-task: option-map-live-codex-diagnostic-repair-24
profile: Application
surface-tags: ui
package: examples/option_map.ail
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

## End-To-End Example: example-25
semantic-task: c-interop-live-codex-core-to-spec-25
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
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

## End-To-End Example: example-26
semantic-task: c-interop-live-codex-core-to-summary-26
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
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

## End-To-End Example: example-27
semantic-task: c-interop-live-codex-flow-patch-27
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
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

## End-To-End Example: example-28
semantic-task: c-interop-live-codex-trace-debug-28
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
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

## End-To-End Example: example-29
semantic-task: c-interop-live-codex-interop-29
profile: Application
surface-tags: c-host-interop
package: examples/c_interop.ail
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

## End-To-End Example: example-30
semantic-task: support-ticket-30
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-30.json
response-file: responses/example-30.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-31
semantic-task: support-ticket-31
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-31.json
response-file: responses/example-31.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-32
semantic-task: support-ticket-live-spec-input-32
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
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

## End-To-End Example: example-33
semantic-task: support-ticket-33
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-33.json
response-file: responses/example-33.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-34
semantic-task: support-ticket-34
profile: Application
surface-tags: backend-portability
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-34.json
response-file: responses/example-34.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-35
semantic-task: runtime-generic-live-codex-core-to-spec-35
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
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

## End-To-End Example: example-36
semantic-task: runtime-generic-live-codex-core-to-summary-36
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
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

## End-To-End Example: example-37
semantic-task: runtime-generic-live-codex-flow-patch-37
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
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

## End-To-End Example: example-38
semantic-task: runtime-generic-live-codex-trace-debug-38
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
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

## End-To-End Example: example-39
semantic-task: runtime-generic-39
profile: Application
surface-tags: core
package: examples/runtime_generic.ail
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-39.json
response-file: responses/example-39.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: PrioritizeTicket
runtime-state: ticket.id=T-1;ticket.priority=Low
endpoint-label: local-endpoint

## End-To-End Example: example-40
semantic-task: refund-tool-live-codex-interview-40
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-41
semantic-task: refund-tool-live-codex-requirements-41
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-42
semantic-task: refund-tool-live-codex-spec-draft-42
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-43
semantic-task: refund-tool-live-codex-core-draft-43
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-44
semantic-task: refund-tool-live-codex-diagnostic-repair-44
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-45
semantic-task: refund-tool-live-codex-core-to-spec-45
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-46
semantic-task: refund-tool-live-codex-core-to-summary-46
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-47
semantic-task: refund-tool-live-codex-flow-patch-47
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-48
semantic-task: refund-tool-live-codex-trace-debug-48
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-49
semantic-task: refund-tool-live-codex-interop-49
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-50
semantic-task: refund-tool-50
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-50.json
response-file: responses/example-50.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
endpoint-label: local-endpoint

## End-To-End Example: example-51
semantic-task: refund-tool-51
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-51.json
response-file: responses/example-51.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
endpoint-label: local-endpoint

## End-To-End Example: example-52
semantic-task: refund-tool-live-spec-input-52
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
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

## End-To-End Example: example-53
semantic-task: refund-tool-53
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-53.json
response-file: responses/example-53.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
endpoint-label: local-endpoint

## End-To-End Example: example-54
semantic-task: refund-tool-54
profile: AgentTool
surface-tags: core
package: examples/refund_tool.ail
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-54.json
response-file: responses/example-54.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RefundCustomerPayment
runtime-state: order.id=O-1;payment.captured=true;refund.amount=100
endpoint-label: local-endpoint

## End-To-End Example: example-55
semantic-task: compiler-pass-live-codex-core-to-spec-55
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-56
semantic-task: compiler-pass-live-codex-core-to-summary-56
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-57
semantic-task: compiler-pass-live-codex-flow-patch-57
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-58
semantic-task: compiler-pass-live-codex-trace-debug-58
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-59
semantic-task: compiler-pass-live-codex-interop-59
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-60
semantic-task: compiler-pass-live-codex-interview-60
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-61
semantic-task: compiler-pass-live-codex-requirements-61
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-62
semantic-task: compiler-pass-live-codex-spec-draft-62
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-63
semantic-task: compiler-pass-live-codex-core-draft-63
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-64
semantic-task: compiler-pass-live-codex-diagnostic-repair-64
profile: Compiler
surface-tags: core
package: examples/compiler_pass.ail
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

## End-To-End Example: example-65
semantic-task: ui-workflow-live-codex-core-to-spec-65
profile: UI
surface-tags: ui
package: examples/ui_workflow.ail
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

## End-To-End Example: example-66
semantic-task: network-driver-live-codex-core-to-summary-66
profile: System
surface-tags: core
package: examples/network_driver.ail
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

## End-To-End Example: example-67
semantic-task: network-driver-67
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-67.json
response-file: responses/example-67.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-68
semantic-task: network-driver-68
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-68.json
response-file: responses/example-68.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-69
semantic-task: network-driver-69
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-69.json
response-file: responses/example-69.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-70
semantic-task: network-driver-70
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/interview.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:5ca61a4509169980
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-70.json
response-file: responses/example-70.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-71
semantic-task: network-driver-71
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-71.json
response-file: responses/example-71.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-72
semantic-task: network-driver-72
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-72.json
response-file: responses/example-72.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-73
semantic-task: network-driver-73
profile: System
surface-tags: core
package: examples/network_driver.ail
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-73.json
response-file: responses/example-73.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
endpoint-label: local-endpoint

## End-To-End Example: example-74
semantic-task: network-driver-live-codex-diagnostic-repair-74
profile: System
surface-tags: core
package: examples/network_driver.ail
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

## End-To-End Example: example-75
semantic-task: secret-access-live-codex-core-to-spec-75
profile: System
surface-tags: core
package: examples/secret_access.ail
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

## End-To-End Example: example-76
semantic-task: secret-access-76
profile: System
surface-tags: core
package: examples/secret_access.ail
prompt-file: docs/ail/prompts/core-to-summary.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:49f26ec41d722633
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-76.json
response-file: responses/example-76.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
endpoint-label: local-endpoint

## End-To-End Example: example-77
semantic-task: secret-access-77
profile: System
surface-tags: core
package: examples/secret_access.ail
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-77.json
response-file: responses/example-77.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
endpoint-label: local-endpoint

## End-To-End Example: example-78
semantic-task: secret-access-78
profile: System
surface-tags: core
package: examples/secret_access.ail
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-78.json
response-file: responses/example-78.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
endpoint-label: local-endpoint

## End-To-End Example: example-79
semantic-task: secret-access-79
profile: System
surface-tags: core
package: examples/secret_access.ail
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-79.json
response-file: responses/example-79.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: ViewInternalNotes
runtime-state: ticket.id=T-1;requester.role=SupportAgent
endpoint-label: local-endpoint

## End-To-End Example: example-80
semantic-task: repeated-task-live-codex-interview-80
profile: System
surface-tags: core
package: examples/repeated_task.ail
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

## End-To-End Example: example-81
semantic-task: repeated-task-81
profile: System
surface-tags: core
package: examples/repeated_task.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-81.json
response-file: responses/example-81.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
endpoint-label: local-endpoint

## End-To-End Example: example-82
semantic-task: repeated-task-82
profile: System
surface-tags: core
package: examples/repeated_task.ail
prompt-file: docs/ail/prompts/spec-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:b23778093326102c
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-82.json
response-file: responses/example-82.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
endpoint-label: local-endpoint

## End-To-End Example: example-83
semantic-task: repeated-task-83
profile: System
surface-tags: core
package: examples/repeated_task.ail
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-83.json
response-file: responses/example-83.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
endpoint-label: local-endpoint

## End-To-End Example: example-84
semantic-task: repeated-task-84
profile: System
surface-tags: core
package: examples/repeated_task.ail
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-84.json
response-file: responses/example-84.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: linux-x86_64-elf
vm-action: RunMaintenanceCycle
runtime-state: counter.value=0
endpoint-label: local-endpoint

## End-To-End Example: example-85
semantic-task: c-interop-live-codex-core-to-spec-85
profile: System
surface-tags: core
package: examples/c_interop.ail
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

## End-To-End Example: example-86
semantic-task: c-interop-live-codex-core-to-summary-86
profile: System
surface-tags: core
package: examples/c_interop.ail
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

## End-To-End Example: example-87
semantic-task: c-interop-87
profile: System
surface-tags: core
package: examples/c_interop.ail
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-87.json
response-file: responses/example-87.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: wasm32-unknown-sandbox-wasm
vm-action: CompressPayload
endpoint-label: local-endpoint

## End-To-End Example: example-88
semantic-task: c-interop-live-codex-trace-debug-88
profile: System
surface-tags: core
package: examples/c_interop.ail
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

## End-To-End Example: example-89
semantic-task: c-interop-live-codex-interop-89
profile: System
surface-tags: core
package: examples/c_interop.ail
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

## End-To-End Example: example-90
semantic-task: support-ticket-live-codex-interview-90
profile: System
surface-tags: core
package: examples/support_ticket.ail
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

## End-To-End Example: example-91
semantic-task: support-ticket-91
profile: System
surface-tags: core
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/requirements.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:68e966969e0b1c12
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-91.json
response-file: responses/example-91.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-92
semantic-task: support-ticket-live-codex-spec-92
profile: System
surface-tags: core
package: examples/support_ticket.ail
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

## End-To-End Example: example-93
semantic-task: support-ticket-93
profile: System
surface-tags: core
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/core-draft.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:0175222e4a84bec4
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-93.json
response-file: responses/example-93.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-94
semantic-task: support-ticket-94
profile: System
surface-tags: core
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/diagnostic-repair.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:c9700f2c2e57e49e
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-94.json
response-file: responses/example-94.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: aarch64-apple-darwin-libsystem-macho
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
endpoint-label: local-endpoint

## End-To-End Example: example-95
semantic-task: stateful-counter-live-codex-core-to-spec-95
profile: System
surface-tags: core
package: examples/stateful_counter.ail
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

## End-To-End Example: example-96
semantic-task: stateful-counter-live-codex-core-to-summary-96
profile: System
surface-tags: core
package: examples/stateful_counter.ail
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

## End-To-End Example: example-97
semantic-task: stateful-counter-97
profile: System
surface-tags: core
package: examples/stateful_counter.ail
prompt-file: docs/ail/prompts/flow-patch.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:30136a21ab8d8eb6
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-97.json
response-file: responses/example-97.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
endpoint-label: local-endpoint

## End-To-End Example: example-98
semantic-task: stateful-counter-98
profile: System
surface-tags: core
package: examples/stateful_counter.ail
prompt-file: docs/ail/prompts/trace-debug.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:f5fffd069da83242
executor-family: llm-http
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-98.json
response-file: responses/example-98.ail-spec.md
artifact-kind: ail-spec
checker-result: accepted
target: vm
vm-action: IncrementCounter
runtime-state: counter.value=0
endpoint-label: local-endpoint

## End-To-End Example: example-99
semantic-task: support-ticket-rejected
profile: System
surface-tags: core
package: examples/support_ticket.ail
prompt-file: docs/ail/prompts/interop.system.md
prompt-version: ail-prompts.v0.2
prompt-fingerprint: fnv64:87f6dd1772d48729
executor-family: codex-skill-agent
executor-label: local-executor
capture-origin: deterministic-seed
request-file: requests/example-99.json
response-file: responses/example-99.ail-spec.md
artifact-kind: ail-spec
checker-result: rejected
target: vm
vm-action: CloseTicket
runtime-state: ticket.id=T-1;ticket.status=Open
expected-diagnostic: AIL001
failure-taxonomy: semantic-drift
