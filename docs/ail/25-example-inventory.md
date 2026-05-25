# AIL Example Inventory

## Purpose

This inventory makes referenced examples visible as part of the active
specification suite. Each example lists its source package, status, covered
artifacts, fixtures, and verification command.

## Version Surface

Unless a row states otherwise, examples in this inventory target:

- language reference: `ail-reference.draft`
- AIL-Core schema: `ail-core.schema.v0`
- prompt pack: draft prompt-pack with `AIL-PROMPT-001`
- bytecode: stage-0 VM JSON plus native Linux x86_64 ELF target where
  executable
- conformance suite: `first-slice` package fixtures and profile fixtures

When an example is updated to cover a newer language feature, its row or
section must name the changed version surface.

## Inventory

| Example | Profile | Version surface | Source | Status | Coverage |
| --- | --- | --- | --- | --- | --- |
| Support Ticket | Application | draft/default | `examples/support_ticket.ail/` | accepted first executable target | AIL-Spec, docs AIL-Core, AIL-Flow renderer, trace runtime, accepted/rejected fixtures, patch fixture |
| Refund Tool | AgentTool | draft/default | `examples/refund_tool.ail/` | accepted agent-tool target | AIL-Spec, docs AIL-Core, AIL-Flow renderer, trace runtime, accepted/rejected fixtures |
| Compiler Pass | Compiler | draft/default | `examples/compiler_pass.ail/` | accepted AIL-Meta compiler-pass target | AIL-Spec, docs AIL-Core, bytecode pass runtime, accepted/rejected fixtures |
| Network Driver | System | draft/default | `examples/network_driver.ail/` | accepted system target | AIL-Spec package, docs AIL-Core, AIL-Flow renderer, trace runtime, accepted/rejected fixtures |
| Toolchain Agent | Application | draft/default | `examples/ail_toolchain_agent.ail/` | accepted self-hosting path target | requirements capture, spec drafting, core lowering, pass execution, bytecode, native backend, prompt portability reports |
| Recursive Factorial | Application | draft/default | `examples/recursive_factorial.ail/` | accepted executable-semantics fixture | AIL-Spec, AIL-Core, recursive function bytecode, VM trace runtime |
| Option Map | Application | draft/default | `examples/option_map.ail/` | accepted executable-semantics fixture | AIL-Spec, AIL-Core, collection-transform bytecode, VM trace runtime |
| Stateful Counter | Application | draft/default | `examples/stateful_counter.ail/` | accepted executable-semantics fixture | AIL-Spec, AIL-Core, integer-state bytecode, VM trace runtime |

## Support Ticket

Artifacts:

- `examples/support_ticket.ail/ail-package.md`
- `examples/support_ticket.ail/spec.ail-spec.md`
- `docs/ail/examples/support-ticket.ail-spec.md`
- `docs/ail/examples/support-ticket.ail-core.md`
- `examples/support_ticket.ail/examples/accepted/close-ticket-minimal.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/action-without-trace.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/failure-without-handling.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/failure-without-trace.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/missing-failure-handler.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/missing-reference.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/secret-leak.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/secret-read-without-protection.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/unknown-field-type.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/unknown-field.ail-spec.md`
- `examples/support_ticket.ail/examples/rejected/unknown-requirement-field.ail-spec.md`
- `examples/support_ticket.ail/examples/patches/escalate-ticket.ail-patch.md`

Verification:

```bash
cargo test --test ail_toolchain support_ticket
```

## Refund Tool

Artifacts:

- `examples/refund_tool.ail/ail-package.md`
- `examples/refund_tool.ail/spec.ail-spec.md`
- `docs/ail/examples/refund-tool.ail-spec.md`
- `docs/ail/examples/refund-tool.ail-core.md`
- `examples/refund_tool.ail/examples/accepted/refund-minimal.ail-spec.md`
- `examples/refund_tool.ail/examples/rejected/approval-without-rule.ail-spec.md`
- `examples/refund_tool.ail/examples/rejected/permission-without-rule.ail-spec.md`
- `examples/refund_tool.ail/examples/rejected/secret-output.ail-spec.md`
- `examples/refund_tool.ail/examples/rejected/tool-without-trace.ail-spec.md`
- `examples/refund_tool.ail/examples/rejected/unknown-input-type.ail-spec.md`

Verification:

```bash
cargo test --test ail_toolchain refund_tool
```

## Compiler Pass

Artifacts:

- `examples/compiler_pass.ail/ail-package.md`
- `examples/compiler_pass.ail/spec.ail-spec.md`
- `docs/ail/examples/compiler-pass.ail-spec.md`
- `docs/ail/examples/compiler-pass.ail-core.md`
- `examples/compiler_pass.ail/examples/accepted/infer-read-permissions-minimal.ail-spec.md`
- `examples/compiler_pass.ail/examples/rejected/unknown-value-type.ail-spec.md`

Verification:

```bash
cargo test --test ail_toolchain compiler_pass
```

## Network Driver

Artifacts:

- `examples/network_driver.ail/ail-package.md`
- `examples/network_driver.ail/spec.ail-spec.md`
- `docs/ail/examples/network-driver.ail-core.md`
- `examples/network_driver.ail/examples/accepted/packet-receive-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/mutable-borrow-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/move-resource-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/layout-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/allocation-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/interrupt-context-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/interrupt-priority-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/interrupt-mask-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/scheduler-task-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/scheduler-task-priority-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/scheduler-task-timing-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/accepted/lock-guard-minimal.ail-spec.md`
- `examples/network_driver.ail/examples/rejected/*.ail-spec.md`

Verification:

```bash
cargo test --test ail_toolchain network_driver
```

## Recursive Factorial

Artifacts:

- `examples/recursive_factorial.ail/ail-package.md`
- `examples/recursive_factorial.ail/spec.ail-spec.md`

Verification:

```bash
cargo test --test ail_toolchain ail_spec_lowers_function_surface_into_runnable_bytecode
```

## Option Map

Artifacts:

- `examples/option_map.ail/ail-package.md`
- `examples/option_map.ail/spec.ail-spec.md`

Verification:

```bash
cargo test --test ail_toolchain ail_standard_library_option_map_executes_collection_transform_bytecode
```

## Stateful Counter

Artifacts:

- `examples/stateful_counter.ail/ail-package.md`
- `examples/stateful_counter.ail/spec.ail-spec.md`

Verification:

```bash
cargo test --test ail_toolchain ail_spec_lowers_stateful_counter_increment_to_integer_bytecode
```

## Last Conformance Result

The authoritative conformance result is the local test output from
`cargo test --test ail_toolchain` and `cargo test`. This inventory records the
commands and artifact boundaries; it does not replace fresh verification.
