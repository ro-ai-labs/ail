# AIL Manual: v0.3 Roadmap

## Purpose

The v0.3 roadmap chapter prints the examples-derived learning backlog without
requiring a reviewer or agent to mine `examples-report.txt`. The roadmap is the
machine-readable bridge from useful examples to the next language, prompt,
checker, runtime, target, and documentation work.

Run the chapter:

```sh
python3 scripts/run_ail_interactive_manual.py --chapter v03-roadmap --run-checks
```

## Roadmap Command

Print only the roadmap while still writing the fingerprinted examples artifact
bundle:

```sh
cargo run -- ail-v03-roadmap examples --artifact-dir /tmp/ail-manual-v03-roadmap --release-evidence
```

Expected output starts with:

```text
AIL-v0.3-Roadmap:
```

Expected files include:

```text
/tmp/ail-manual-v03-roadmap/v03-roadmap.txt
/tmp/ail-manual-v03-roadmap/v03-roadmap.fingerprint.txt
/tmp/ail-manual-v03-roadmap/manifest.ail-examples.txt
```

## Review Rule

Treat each `signal` line as a candidate v0.3 improvement. The grouped entry
ids, capability levels, program domains, prompt files, story journeys, and
checker results identify why the signal came from useful end-to-end evidence
instead of free-form planning prose.

