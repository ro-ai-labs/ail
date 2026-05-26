# AIL Manual: Examples Release Replay

## Purpose

The examples release chapter proves that `./examples` is the coherent
end-to-end corpus, not a loose set of fixtures. It replays stored prompt,
response, story, package, Core, bytecode, VM, native, target, diagnostic, and
roadmap evidence through the deterministic verifier.

Run the chapter checks:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter examples-release --run-checks
```

## Release Replay

Generate the release evidence bundle:

```sh
cargo run -- ail-examples examples --artifact-dir /tmp/ail-manual-examples --release-evidence
```

The verifier requires at least 100 accepted prompt-to-artifact examples, useful
use-case metadata, capability-level coverage, prompt coverage, story metadata,
semantic anchors, executor/capture provenance, and v0.3 learning signals.

## Artifacts

Inspect these files after replay:

```text
/tmp/ail-manual-examples/examples-report.txt
/tmp/ail-manual-examples/v03-roadmap.txt
/tmp/ail-manual-examples/model-executor-manifest.txt
/tmp/ail-manual-examples/manifest.ail-examples.txt
```

Per-entry directories under `/tmp/ail-manual-examples/examples/` include the
request transcript, response transcript, extracted artifact, checked Core,
bytecode, VM trace, target report, diagnostics when rejected, and
`user-story.txt` when story metadata is present.

## Review Rule

Use `examples-report.txt` for corpus coverage and `manifest.ail-examples.txt`
for fingerprints. Use `model-executor-manifest.txt` to confirm hosted LLM and
Codex skill-agent provenance. Use `v03-roadmap.txt` to decide which language,
prompt, checker, runtime, target, or documentation work the corpus is asking
for next.

