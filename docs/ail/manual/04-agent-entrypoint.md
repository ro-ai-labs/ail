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
repairer, prompt reviewer, and repair-promotion reviewer contracts. The prompt
reviewer contract must require prompt harness review, story harness review,
examples replay, and `cargo run -- ail-v03-roadmap examples`. The repair
promotion reviewer contract must require `repair-promotion-review.txt`,
`repair-promotion-review.fingerprint.txt`, and
`repair-promotion-review-fingerprint-observed-count`.

The agent contract also requires the human-approved import demo evidence:
`repair-promotion-import-demo-report.txt`,
`repair-promotion-import-demo-report.fingerprint.txt`, `source-preserved true`,
and `proposed-accepted true`. This keeps the agent from treating a repaired
candidate as promotable unless the rejected source entry remains intact and the
proposed accepted entry replays in a corpus copy.

The AgentTool policy reviewer contract must require `agent-policy-review.txt`,
`agent-policy-review.fingerprint.txt`,
`agent-policy-review-fingerprint-observed-count`,
`agent-policy-capture-plan.json`, and
`agent-policy-import-demo-report.txt`. The import report must include
`source-preserved true`, `proposed-accepted true`,
`policy-handoff-imported true`, and `policy-handoff-replayed true` so a policy
handoff amendment remains proposal-only until the reviewed corpus copy replays.

For User Story mode promotion, the prompt reviewer contract must also require
`story-promotion-import-demo-report.txt`,
`story-promotion-import-demo-report.fingerprint.txt`,
`story-artifacts-preserved true`, and `proposed-accepted true`. This keeps
reviewed story-mode output as corpus-copy evidence until a human-approved
promotion imports the full story artifact bundle and proves the proposed
accepted entry replays.

The same gate also validates the repo-local Codex skills:

```text
examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md
examples/agents/skills/ail-repair-promotion-reviewer/SKILL.md
examples/agents/skills/ail-agent-policy-reviewer/SKILL.md
```

Those skills are the reusable procedures for reviewing hosted llama.cpp prompt
artifacts, User Story mode artifacts, examples replay, `v03-roadmap.txt`, and
repair promotion and AgentTool policy evidence, including deterministic import
demos, before generated content is promoted into `./examples`.

## Toolchain Agent Package

Check the AIL-authored toolchain agent package:

```sh
cargo run -- ail-check examples/ail_toolchain_agent.ail
cargo run -- ail-conformance examples/ail_toolchain_agent.ail --artifact-dir /tmp/ail-manual-agent-entrypoint-conformance
cargo test ail_toolchain_agent_package_lowers_to_verified_bytecode --test ail_toolchain
```

The conformance check must include local agent-boundary fixtures:

```text
conformance-report.txt
manifest.ail-conformance.txt
accepted: bytecode-verification-minimal.ail-spec.md
rejected: bytecode-verification-without-fingerprint.ail-spec.md AIL-AGENT-001
ail conformance: ok
```

Then verify the build entrypoint path:

```sh
cargo test cli_ail_build_runs_toolchain_agent_bytecode --test ail_toolchain
```

The expected evidence is:

```text
conformance-report.txt
manifest.ail-conformance.txt
agent.ailbc.json
agent-trace.txt
```

`agent-trace.txt` should show requirements capture, spec preparation, spec
acceptance, compile, bytecode verification, and manifest verification in order.
