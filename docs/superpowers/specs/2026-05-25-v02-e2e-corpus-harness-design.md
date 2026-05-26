# AIL v0.2 Examples Harness Design

## Purpose

AIL v0.2 must prove that the language can survive real model variation, not
only deterministic compiler fixtures. The end-to-end corpus harness provides
that proof by replaying stored model or agent transcripts through the same
checker, lowering, compiler, runtime, and target-contract boundaries used by
normal AIL builds.

The harness is release evidence, not a training benchmark. It accepts only
artifacts that can be replayed without live model access and tied to
fingerprinted prompts, responses, checked Core, bytecode, traces, manifests,
and target outputs.

## Scope

The first implementation target is a new verifier command:

```bash
cargo run -- ail-examples examples --artifact-dir /tmp/ail-examples
```

The command reads corpus manifests and transcript files from
`examples`, replays each stored output, and writes a release bundle.
It does not call HTTP endpoints or Codex during replay.

Live capture is a separate mode or helper. Live capture may call configured
HTTP LLMs or Codex-style skill agents, but capture output is not trusted until
the offline replay command accepts it.

## Corpus Entry Model

Each counted example has one manifest entry with these required fields:

- `semantic-task`: stable id for the user intent and expected behavior
- `profile`: AIL profile name
- `package`: package path used as compilation context
- `prompt-file`: versioned system prompt path
- `prompt-version`: prompt metadata version
- `prompt-fingerprint`: deterministic fingerprint of the prompt file
- `executor-family`: `llm-http`, `ail-toolchain-agent`, or
  `codex-skill-agent`
- `executor-label`: stable model, agent, or skill label
- `endpoint-label`: required for `llm-http`, absent for offline agents
- `request-file`: stored request transcript path
- `response-file`: stored raw response transcript path
- `artifact-kind`: expected deterministic artifact kind
- `checker-result`: `accepted` or `rejected`
- `target`: `vm`, `linux-x86_64-elf`, `wasm32-unknown-sandbox-wasm`, or
  `aarch64-apple-darwin-libsystem-macho`
- `expected-diagnostic`: required for rejected examples
- `failure-taxonomy`: required for rejected examples

The verifier computes request, response, extracted artifact, Core, bytecode,
trace, target, and manifest fingerprints during replay. Corpus files may store
expected fingerprints, but stored fingerprints are advisory until recomputed.

## Replay Pipeline

For accepted examples:

```text
stored request/response
  -> prompt envelope extraction and validation
  -> checked AIL-Requirements or checked AIL-Spec
  -> checked AIL-Core
  -> AIL-Bytecode
  -> VM verification
  -> Linux native artifact or Wasm/Darwin target contract
  -> manifest and fingerprint bundle
```

For rejected examples:

```text
stored request/response
  -> prompt envelope extraction and validation
  -> deterministic checker rejection
  -> expected diagnostic and failure taxonomy match
  -> rejection manifest and fingerprint bundle
```

The replay verifier fails the whole corpus if any accepted example cannot
reach its requested target artifact, if any rejected example is accepted, or if
the counted example total is below the release threshold.

## Release Thresholds

AIL v0.2 requires at least 100 distinct end-to-end examples:

- at least 40 Application examples
- at least 15 AgentTool examples
- at least 10 Compiler examples
- at least 10 System examples
- at least 10 standard-library or package-import examples
- at least 5 UI examples
- at least 5 C/host interop examples
- at least 5 backend portability examples

The totals may overlap when one semantic example exercises multiple surfaces,
but the final counted corpus must still contain at least 100 distinct
semantic-task entries.

Prompt coverage must include every prompt in `docs/ail/prompts` at least once.
Executor coverage must include `llm-http` and `codex-skill-agent`. At least one
semantic task family must include two different HTTP model or endpoint labels
so portability evidence is not only a single-model replay.

## Output Bundle

The verifier writes:

- `examples-report.txt`
- `examples-report.fingerprint.txt`
- `manifest.ail-examples.txt`
- `manifest.fingerprint.txt`
- `examples/<semantic-task>/request.fingerprint.txt`
- `examples/<semantic-task>/response.fingerprint.txt`
- `examples/<semantic-task>/artifact.fingerprint.txt`
- `examples/<semantic-task>/checked.ail-core.txt` for accepted examples
- `examples/<semantic-task>/checked.ail-core.fingerprint.txt`
- `examples/<semantic-task>/artifact.ailbc.json`
- `examples/<semantic-task>/artifact.ailbc.fingerprint.txt`
- `examples/<semantic-task>/vm-trace.txt` when VM execution is applicable
- `examples/<semantic-task>/target-report.txt` for native or contract targets
- `examples/<semantic-task>/target-report.fingerprint.txt`
- `examples/<semantic-task>/diagnostics.txt` for rejected examples

The top-level report includes counts by profile, prompt, executor family,
target, checker result, and failure taxonomy. It also reports duplicate
fingerprint counts for requests, responses, extracted artifacts, checked Core,
bytecode, VM traces, native artifacts, target reports, and diagnostics so seed
fixtures cannot be mistaken for semantically broad release evidence.

## Codex Skill Agent Boundary

Codex-style skills and sub-agents are treated as untrusted executors. A Codex
executor can draft requirements, specs, repairs, or patches, but its output
must be normalized into the same prompt envelope and replayed through the same
deterministic checker and compiler gates.

Release evidence for a Codex executor records the skill or agent label, task
prompt, output transcript, extracted envelope, and replay artifacts. It must
not rely on conversational memory or an unstored agent decision as proof.

## Implementation Slices

1. Add parser and tests for the `examples` manifest format.
2. Add offline replay for accepted AIL-Spec outputs through Core and bytecode.
3. Add target generation for VM, Linux ELF, Wasm contract, and Darwin contract.
4. Add rejected-output replay with expected diagnostics and failure taxonomy.
5. Add corpus summary thresholds for 100 examples, prompt coverage, profile
   coverage, executor-family coverage, and target coverage.
6. Add live capture helpers for HTTP LLM endpoints and Codex skill agents.
7. Generate and commit the v0.2 release evidence bundle only after replay
   passes from a clean checkout.

## Current Gap

The existing prompt corpus has 14 stored outputs for one support-ticket task.
It validates accepted AIL-Spec outputs only as checked Core, and it does not
compile each accepted output to bytecode, native binary, or target contract.
That corpus remains useful as a prompt-portability fixture, but it is not
sufficient v0.2 release evidence.
