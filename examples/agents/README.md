# AIL Codex Example Agents

This directory defines the Codex-style skill-agent executor labels that may be
used by `ail-examples` entries with `executor-family: codex-skill-agent`.
They are evidence contracts, not trusted compilers.

Each live Codex entry must store:

- the agent contract file name and version in the request JSON
- the exact user task, prompt-pack file, profile, package manifest, and input
  artifact given to the agent
- the raw Codex response JSON
- the extracted deterministic AIL artifact accepted or rejected by replay
- `capture-origin: live-codex`

The trusted boundary stays unchanged: Codex may draft or repair an artifact,
but only `ail-examples` replay proves spec -> Core -> bytecode -> VM and
target evidence.

## Deterministic Entrypoint Checks

Run the manual chapter checks without contacting a live model:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter agent-entrypoint --run-checks
```

The chapter checks the Codex-style agent contract files, validates them with
`cargo run -- ail-agent-contracts examples/agents`, checks
`examples/ail_toolchain_agent.ail`, verifies that the toolchain-agent package
lowers to bytecode, and verifies that `ail-build` writes `agent.ailbc.json` and
`agent-trace.txt` while the agent participates in the build entrypoint.

## Agent Contracts

| executor-label | Contract | Primary output | Replay requirement |
| --- | --- | --- | --- |
| `codex-ail-requirements-writer` | `codex-ail-requirements-writer.md` | AIL-Requirements or blocking questions | prompt envelope validates, then requirements feed a checked spec path |
| `codex-ail-spec-writer` | `codex-ail-spec-writer.md` | canonical AIL-Spec | parser, checker, Core lowering, bytecode, VM trace, and target evidence pass |
| `codex-ail-diagnostic-repairer` | `codex-ail-diagnostic-repairer.md` | repaired AIL-Spec or rejected diagnostic explanation | repaired artifact passes or the expected diagnostic is reproduced |
| `codex-ail-prompt-reviewer` | `codex-ail-prompt-reviewer.md` | Prompt and story harness review report | `scripts/run_v03_prompt_llm_harness.py --review-artifacts`, `scripts/run_v03_story_llm_harness.py --review-artifacts`, `scripts/run_v03_story_promotion_capture_plan.py --story-artifacts` writes `story-promotion-capture-plan.json`, then `scripts/run_v03_story_promotion_import_demo.py` writes `story-promotion-import-demo-report.txt` with `story-artifacts-preserved true` and `proposed-accepted true`; `ail-examples examples --artifact-dir ... --release-evidence` and `cargo run -- ail-v03-roadmap examples ...` write `v03-roadmap.txt` and pass before promotion |
| `codex-ail-repair-promotion-reviewer` | `codex-ail-repair-promotion-reviewer.md` | Repair promotion review report and capture plan | `ail-examples examples --artifact-dir ... --release-evidence` writes `repair-promotion-review.txt`, then `scripts/run_v03_repair_promotion_capture_plan.py` writes `repair-promotion-capture-plan.json`, and `scripts/run_v03_repair_promotion_import_demo.py` writes `repair-promotion-import-demo-report.txt` with `source-preserved true` and `proposed-accepted true` before any repaired artifact is proposed for human-approved corpus promotion |
| `codex-ail-agent-policy-reviewer` | `codex-ail-agent-policy-reviewer.md` | AgentTool policy handoff review report and capture plan | `ail-examples examples --artifact-dir ... --release-evidence` writes `agent-policy-review.txt`, then `scripts/run_v03_agent_policy_capture_plan.py` writes `agent-policy-capture-plan.json`, `scripts/run_v03_agent_policy_import_demo.py` writes `agent-policy-import-demo-report.txt` with `source-preserved true`, `proposed-accepted true`, `policy-handoff-imported true`, and `policy-handoff-replayed true`, and `scripts/run_v03_agent_policy_live_reviewer_harness.py --review-artifacts /tmp/ail-v03-agent-policy-live-review` reviews optional hosted reviewer evidence before any AgentTool policy handoff is proposed for human-approved corpus promotion |

## Codex Skills

The reusable prompt/system-interaction review skill is stored at
`examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md`. It mirrors
`codex-ail-prompt-reviewer.md` in Codex skill format, records the hosted
llama.cpp endpoint, and lists the deterministic commands and evidence required
before promoting generated prompt or User Story mode artifacts, including the
story-promotion import demo evidence.

The reusable repair-promotion review skill is stored at
`examples/agents/skills/ail-repair-promotion-reviewer/SKILL.md`. It mirrors
`codex-ail-repair-promotion-reviewer.md` and lists the deterministic
`repair-promotion` manual chapter evidence plus the plan-only capture artifact
and import-demo evidence required before a repaired rejected example can be
proposed as an accepted corpus entry.

The reusable AgentTool policy review skill is stored at
`examples/agents/skills/ail-agent-policy-reviewer/SKILL.md`. It mirrors
`codex-ail-agent-policy-reviewer.md` and lists the deterministic
`agent-policy-import` manual chapter evidence plus the plan-only capture
artifact, import-demo evidence, deterministic handoff witness, and optional
hosted live reviewer artifact review required before a policy handoff
amendment can be proposed as an accepted corpus entry. Hosted review evidence
must include `agent-policy-live-review-report.txt`,
`agent-policy-live-review-review.txt`, `reviewer-envelope-valid-count`, and
`reviewer-decision-accept-count` when it is claimed.

## Request JSON Shape

```json
{
  "agent_contract": "examples/agents/codex-ail-spec-writer.md",
  "agent_contract_version": "0.1.0",
  "executor_label": "codex-ail-spec-writer",
  "codex_model": "codex-model-name",
  "prompt_file": "docs/ail/prompts/spec-draft.system.md",
  "prompt_version": "ail-prompts.v0.2",
  "package": "examples/support_ticket.ail",
  "profile": "Application",
  "task": "Draft canonical AIL-Spec from checked requirements.",
  "input": {}
}
```

The response JSON must contain the exact text returned by Codex in `content` or
`artifact_text`, or an OpenAI-compatible `choices[0].message.content` field.
Replay extracts that text using the same stored-response rules as live LLM
captures.
