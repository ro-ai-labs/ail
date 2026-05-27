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
repairer, prompt reviewer, story-promotion reviewer, repair-promotion reviewer,
AgentTool policy reviewer, and UI patch reviewer contracts. The prompt reviewer
contract must require prompt harness review, story harness review, examples
replay, and `cargo run -- ail-v03-roadmap examples`. The repair promotion
reviewer contract must require `repair-promotion-review.txt`,
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

The UI patch reviewer contract, `codex-ail-ui-patch-reviewer.md`, must require
`ui-review-patch.txt`, `ui-review-patch.fingerprint.txt`,
`ui-review-patch-fingerprint-observed-count`,
`ui-patch-capture-plan.json`, `ui-patch-import-demo-report.txt`, and
`ui-patch-runtime-state-check-report.txt`. The import and runtime reports must
include `source-preserved true`, `proposed-accepted true`,
`flow-edit-applied true`, `patched-core-replayed true`,
`visual-regression-fingerprint-preserved true`, and
`runtime-ui-state-anchor Ticket.reviewStatus` so reviewed UI/flow patches stay
proposal-only until the visual review, patched Core, and runtime UI-state
witness all agree.

For User Story mode promotion, the story-promotion reviewer contract,
`codex-ail-story-promotion-reviewer.md`, must require
`story-promotion-capture-plan.json`, `story-promotion-import-demo-report.txt`,
`story-promotion-import-demo-report.fingerprint.txt`,
`story-artifacts-preserved true`, `proposed-accepted true`,
`capture-plan story-promotion-capture-plan.json`,
`promotion-decision accepted-for-promotion`, `human-approval-required true`,
`promotion-source human-approved-story-promotion-batch`, `batch-plan-fingerprint`,
`default-max-tokens`, `max-tokens`, `token-budget-default`, and any
`token-budget-warning`. This keeps reviewed story-mode output, its visible
hosted generation budget, and the human-approved promotion batch as corpus-copy
evidence until a promotion imports the full story artifact bundle and proves
the proposed accepted entry replays.

The same gate also validates the repo-local Codex skills:

```text
examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md
examples/agents/skills/ail-story-promotion-reviewer/SKILL.md
examples/agents/skills/ail-system-prompt-harness-runner/SKILL.md
examples/agents/skills/ail-repair-promotion-reviewer/SKILL.md
examples/agents/skills/ail-agent-policy-reviewer/SKILL.md
examples/agents/skills/ail-ui-patch-reviewer/SKILL.md
```

Those skills are the reusable procedures for running and reviewing hosted
llama.cpp prompt artifacts, User Story mode artifacts, examples replay,
`v03-roadmap.txt`, story promotion, repair promotion, and AgentTool policy
evidence, including deterministic import demos, before generated content is
promoted into `./examples`. The UI patch reviewer skill adds the visual/flow patch
import gate for `ui-patch-import-demo-report.txt` and
`ui-patch-runtime-state-check-report.txt`.
Hosted llama.cpp prompt-pack evidence must not accept artifacts whose
model-check was skipped; `model-check skipped` is only valid for local
fake-server harness tests.

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
cargo test cli_ail_build_agent_verifies_bytecode_artifact_after_compile --test ail_toolchain
```

The expected evidence is:

```text
conformance-report.txt
manifest.ail-conformance.txt
agent.ailbc.json
artifact.ailbc.json
agent-trace.txt
action CompileApplication started
action VerifyBytecodeArtifact started
```

`agent-trace.txt` should show requirements capture, spec preparation, spec
acceptance, compile, bytecode verification, and manifest verification in order.
The bytecode-after-compile gate specifically rejects stale build-agent traces
where `VerifyBytecodeArtifact` appears before `CompileApplication`, or where
the trace claims verification without a compiled `artifact.ailbc.json`.
