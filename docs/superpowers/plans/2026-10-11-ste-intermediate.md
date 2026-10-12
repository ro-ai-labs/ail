# AIL STE Intermediate Implementation Plan

> Execute inline with test-driven development and verification before completion.

**Goal:** Make controlled English an executable intermediate in the normal AIL compiler and an agent input format.

**Architecture:** Core retains semantic checking. A fixed STE instruction grammar
serializes semantic lowering, then its strict parser produces the bytecode used
by the existing VM and native backend.

**Tech Stack:** Rust, existing dependency-free toolchain and integration tests.

**Spec:** `docs/superpowers/specs/2026-10-11-ste-intermediate-design.md`

## Global constraints

- Preserve package, capability, target, failure, and instruction data.
- Reject unrecognized executable input; do not claim complete ASD conformity.
- Preserve frozen benchmark cases and independent Python oracles.
- Continue the full Python replacement goal after this stage.

## Review focus

- Quoted/newline/Unicode technical data cannot inject code.
- Missing and duplicate identifiers cannot silently change semantics.
- Metadata cannot bypass the instruction parser with hidden actions.
- Trace-only existing operations must retain their actual semantics.
- Normal compilation and saved STE must produce the same executable behavior.

## Tasks

- [x] Add failing CLI tests for STE export, direct execution, native compilation,
  changed operands, invalid sentences, and artifact persistence.
- [x] Implement `src/ste.rs`: versioned grammar, data declarations, deterministic
  renderer, strict parser, and round-trip tests for checked example packages.
- [x] Change `compile_ail_core_bytecode` to use `compile_ail_core_ste` plus STE
  parsing; integrate commands and artifact output in `src/main.rs`.
- [x] Add an authored example, reference documentation, and agent prompt guidance.
- [x] Run required repository verification and capability benchmark; record
  evidence and remaining gaps without marking the overall goal complete.

## Execution record

The worktree was clean at `9edd2ee`. No STE implementation was present. The
existing computation lock records only R000-increment passing. This continuation
is authorized by the active user goal; implementation proceeds in the shared
worktree without a new approval or a commit to restricted Git metadata.


Implementation and verification on 2026-10-11:

- `src/ste.rs` supplies 62 instruction templates, data bindings, strict metadata
  parsing, and lossless rendering. The shared Core compiler consumes parsed STE.
- Added `ail-ste`, direct STE check/lower/run/VM/compile, a runnable authored
  counter, companion bundle artifacts, and reference/agent documentation.
- Initial CLI tests failed because the command and intermediate did not exist.
  Review tests then reproduced a reversed comparison, misleading trace wording,
  lost external-binding presence, an ignored specification override, and a JSON
  Unicode restriction. The fixes now pass execution and rejection tests.
- `cargo test`: 477 passed (31 library, 434 existing integration, 12 STE
  integration). The suite required local socket access for its loopback HTTP
  fixtures; the sandbox-only run failed at socket binding, not compiler behavior.
- `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`,
  and `git diff --check` passed. A Clippy nesting warning was corrected.
- `python3 scripts/ail_bench.py`: 1/16, with all four R000 cases agreeing between
  the Python oracle, VM, and native binary. Frozen benchmark inputs are unchanged.
- The read-only code review's four substantive findings and Unicode finding were
  reproduced and resolved. Existing VM limitations were retained and documented.

The overall goal remains active. Full ASD-STE100 conformance is unproven; this
is an executable programming profile, not a general STE checker. Python
replacement is contradicted by the 1/16 computation score. Next implement typed
variable expressions through Core, STE, VM and native code, starting with the
frozen R001-add cases and its sub-rungs. Then continue to real control flow,
functions, collections, I/O, and agent-driven application workloads. Do not use
this completed intermediate-stage plan as evidence that the broader goal is done.
