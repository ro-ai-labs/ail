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
`user-story.txt` when story metadata is present. Accepted UI workflow entries
and accepted entries tagged with the `ui` surface also include
`ui-review.txt`, a deterministic visual/accessibility/workflow review artifact
with upstream fingerprints. Repeated Task entries also include
`workflow-scheduler-review.txt`, a deterministic scheduler/retry/backoff
review artifact that links the repeated action, temporal policy, retry policy,
backoff policy, `AIL-WORKFLOW-*` diagnostics, package-local fixtures, and
runtime fingerprints. C interop entries include
`unsafe-boundary-review.txt`, a deterministic ABI/unsafe-boundary tutorial
that links zlib/libc host calls, pointer ownership, borrowed mutable buffers,
noescape callbacks, `repr(C)` layout, status maps, nullable contracts,
package-local FFI fixtures, diagnostics, and replay fingerprints.
Incident-response complex-system entries include `complex-story-graph.txt`, a
deterministic graph review artifact that links imported modules, UI surfaces,
workflow transitions, target contracts, regenerated story views, semantic
anchors, runtime evidence, and replay fingerprints.

## Review Rule

Use `examples-report.txt` for corpus coverage and `manifest.ail-examples.txt`
for fingerprints. Use `model-executor-manifest.txt` to confirm hosted LLM and
Codex skill-agent provenance. Use `v03-roadmap.txt` to decide which language,
prompt, checker, runtime, target, or documentation work the corpus is asking
for next. For UI workflow and UI-surface entries, check
`ui-review-fingerprint-*` report lines and the corresponding
`entry-artifact ... ui-review ...` manifest entries before claiming the
visual/accessibility review path is covered.
For scheduled-workflow entries, check
`workflow-scheduler-review-fingerprint-*` report lines and the corresponding
`entry-artifact ... workflow-scheduler-review ...` manifest entries before
claiming retry/backoff scheduler evidence is covered.
For C interop entries, check `unsafe-boundary-review-fingerprint-*` report
lines and the corresponding `entry-artifact ... unsafe-boundary-review ...`
manifest entries before claiming unsafe-boundary tutorial evidence is covered.
For incident-response complex-system entries, check
`complex-story-graph-fingerprint-*` report lines and the corresponding
`entry-artifact ... complex-story-graph ...` manifest entries before claiming
multi-surface story graph evidence is covered.
