# AIL Manual: Agent Entrypoint

## Purpose

The agent entrypoint chapter proves that the AIL development toolchain can use
Codex-style agent roles and an AIL-authored toolchain agent without treating the
agent as the trusted compiler. The compiler and verifier remain the authority;
the agent records participation and checks artifacts.

Run deterministic checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
```

## Agent Contracts

Validate the Codex role contracts:

```sh
cargo run -- ail-agent-contracts examples/agents
```

The report must include the requirements writer, spec writer, diagnostic
repairer, and prompt reviewer contracts. The prompt reviewer contract must
require prompt harness review, story harness review, examples replay, and
`cargo run -- ail-v03-roadmap examples`.

## Toolchain Agent Package

Check the AIL-authored toolchain agent package:

```sh
cargo run -- ail-check examples/ail_toolchain_agent.ail
cargo test ail_toolchain_agent_package_lowers_to_verified_bytecode --test ail_toolchain
```

Then verify the build entrypoint path:

```sh
cargo test cli_ail_build_runs_toolchain_agent_bytecode --test ail_toolchain
```

The expected evidence is:

```text
agent.ailbc.json
agent-trace.txt
```

`agent-trace.txt` should show requirements capture, spec preparation, spec
acceptance, compile, bytecode verification, and manifest verification in order.

