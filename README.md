# AIL

AIL means Agentic Intent Language.

This repository contains the AIL specification, examples, compiler, bytecode
VM, and native Linux ELF toolchain. AIL starts from developer intent, captures
requirements through an AI-assisted programming agent, normalizes accepted
programs into AIL-Core IR, and compiles checked artifacts into executable
behavior.

The active documentation starts at [docs/README.md](docs/README.md).
The complete AIL specification suite is indexed at
[docs/ail/README.md](docs/ail/README.md), including execution semantics,
AIL-Core schema, prompt pack, package model, C interop, backend portability,
UI profile, diagnostics, safety, corpus, and traceability artifacts.

## Toolchain

The current compiler accepts deterministic AIL package artifacts, not free-form
conversation. The LLM-facing agent path is responsible for interviewing the
application developer, capturing sufficient requirements, converting those
requirements into AIL-Spec, elaborating AIL-Spec into AIL-Core, and invoking
the compiler pipeline.

The supported AIL commands are:

- `ail-check`
- `ail-core`
- `ail-flow`
- `ail-lower`
- `ail-compile`
- `ail-run`
- `ail-vm`
- `ail-conformance`
- `ail-requirements`
- `ail-spec`
- `ail-draft`
- `ail-build`
- `ail-pass`
- `ail-bootstrap`
- `ail-patch`

Use the commands through Cargo during development:

```bash
cargo run -- ail-check examples/support_ticket.ail
cargo run -- ail-core examples/support_ticket.ail
cargo run -- ail-flow examples/support_ticket.ail
cargo run -- ail-lower examples/support_ticket.ail
cargo run -- ail-run examples/support_ticket.ail --action CloseTicket ticket.status=Open
```

Checked AIL-Core can also render back to deterministic AIL-Spec for review or
agent handoff:

```bash
cargo run -- ail-core examples/support_ticket.ail > /tmp/support-ticket.ail-core.txt
cargo run -- ail-spec --core-file /tmp/support-ticket.ail-core.txt
```

AIL-Flow and agent graph patches can apply directly to a saved checked
AIL-Core artifact:

```bash
cargo run -- ail-patch --core-file /tmp/support-ticket.ail-core.txt /path/to/edit.ail-core.patch.json
```

## Machine Bytecode

For Linux, the machine-level bytecode target is a native ELF executable.

```bash
cargo run -- ail-compile examples/support_ticket.ail \
  --action CloseTicket \
  --target linux-x86_64-elf \
  --out /tmp/close-ticket
```

The compiler emits ELF64 x86_64 executable bytes directly for the supported
native subset. The saved AIL bytecode JSON artifact remains an auditable
intermediate for checker, agent, and VM workflows; it is not presented as the
Linux machine-level bytecode target.

Artifact bundle commands such as `ail-lower`, `ail-compile`, `ail-build`,
`ail-pass`, `ail-conformance`, and `ail-bootstrap` can write fingerprints,
manifests, native bytecode reports, dependency reports, checked AIL-Core, AIL
bytecode, native ELF outputs, and AIL-authored agent traces with
`--artifact-dir <dir>`.

## LLM Agent Path

The local LLM endpoint can be supplied with `--llm-endpoint`. The intended
development loop is:

```bash
cargo run -- ail-requirements examples/support_ticket.ail \
  --prompt "Build a support ticket application" \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions

cargo run -- ail-spec examples/support_ticket.ail \
  --requirements-file /path/to/requirements.ail-requirements.md \
  --prompt "Draft the support ticket specification" \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions

cargo run -- ail-build examples/support_ticket.ail \
  --prompt "Build a support ticket application" \
  --target linux-x86_64-elf \
  --action CloseTicket \
  --artifact-dir /tmp/ail-build \
  --llm-endpoint http://inteligentia-pro-1:8080/v1/chat/completions
```

The agent and base LLM are untrusted proposal mechanisms. The trusted boundary
is checked AIL-Core plus the compiler, verifier, manifests, reports, and
fingerprints generated from deterministic artifacts.

LLM responses may be raw deterministic artifacts or the prompt-pack JSON
envelope with `artifact_text`. If the envelope contains blocking `questions`
instead of an artifact, the command surfaces those questions and stops before
checker, repair, or compile stages. Malformed envelopes are rejected as
`AIL-PROMPT-001` prompt protocol errors. Envelope metadata must match the
requested artifact kind and package profile, and must keep checker handoff
enabled.

## Examples

AIL packages live under [examples](examples):

- `support_ticket.ail`
- `support_composed.ail`
- `support_shared.ail`
- `refund_tool.ail`
- `secret_access.ail`
- `runtime_generic.ail`
- `network_driver.ail`
- `compiler_pass.ail`
- `ail_toolchain_agent.ail`

Each package has an `ail-package.md` manifest and a `spec.ail-spec.md` entry
spec. Accepted and rejected conformance fixtures live under package-local
`examples/accepted` and `examples/rejected` directories.

## Verification

Use these checks before claiming a cleanup or language change is complete:

```bash
cargo fmt --check
cargo check
cargo test --test ail_toolchain
cargo test
cargo clippy --all-targets -- -D warnings
git diff --check
```
