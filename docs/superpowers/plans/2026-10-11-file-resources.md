# Named file resource implementation plan

Design: ../specs/2026-10-11-file-resources-design.md

- [x] Frontend/Core/bytecode/runtime module with strict validation and focused tests.
- [x] Direct Linux x86-64 file helpers and grant parsing, focused tests.
- [x] Root integration: model/order/parser/render/check/compile/verify/VM/calls/CLI/STE.
- [x] End-to-end tests, examples and independent Python differential verification.
- [x] Review, full test suite, fmt/clippy, frozen ladder and corpus compatibility.
- [x] Documentation and verification results, preserving the broader open goal.

## Review fixes

- Public resource maps originally allowed malformed unused bindings even though
  CLI/native flags rejected them. A regression demonstrated a write taking
  effect despite a malformed map. Entry validation now rejects every invalid
  resource name, empty path and NUL-containing path before execution, for both
  read and write maps.
- Direct bytecode could carry file operations under unsupported profiles.
  Verification now checks Application/C interop before accepting these opcodes;
  regression coverage includes direct STE, bytecode VM and native compilation.
- Native opens include O_CLOEXEC, O_NONBLOCK and O_NOCTTY, and native/VM file
  operation trace wording agrees. Short-write and EINTR loops preserve helper
  registers and caller permissions. Scratch mmap failure is a host IO error.
- The new timeout test helper initially waited without draining its pipes;
  a 1 MiB legacy SET_FIELD trace filled stderr and blocked the test child.
  Concurrent pipe readers now preserve both timeout coverage and bounded error
  reporting. All 16 file integration cases pass after the fix.
- Independent reviews checked the typed Core model, runtime permissions,
  namespace/ordering hooks, both recursive call paths, and native/VM semantics.
  No further reproduced defect remains.

## Verified artifacts

- `examples/text-files` reads a UTF-8 document, prefixes a heading, counts 34
  Unicode characters and saves through a called action. Source and direct STE
  run/compile on VM and Linux ELF with identical file bytes. Checked-in STE
  matches current export at `examples/ste/text-files.ail-ste`.
- `docs/ail/40-file-resources.md` records source, Core, STE, APIs, CLI grants,
  permissions, limits, errors, execution order and side-effect behavior.
- The independent `scripts/ail_file_differential.py` passes 200 fixed-seed
  Unicode/NUL/empty transforms with immutable copies, plus exact/overflow sizes,
  invalid UTF-8, denied/reversed/spoofed grants, malformed flags, duplicate
  bindings, paths with spaces and equals signs, default-state isolation,
  persistent writes after failure, regular-file checks, FIFOs and symlinks.
  Log: `/tmp/ail-files-python-differential.log`.
- Existing Python oracles pass: 605 text cases, 600 list pipelines, 1,600 list
  grammar mutations, 500 typed bytecode literals and 50 source literal cases,
  with their malformed-input and exact/overflow checks.
- Frozen numeric ladder remains 16/16; corpus differential remains 33 agree,
  30 not natively compilable, 0 diverged. No benchmark cases/oracles were edited.
  Logs: `/tmp/ail-files-bench.log`, `/tmp/ail-files-corpus-differential.log`.
- **720 tests passed** in `cargo test --all-targets -- --test-threads=1`, including
  12 file module tests, eight new native emitter tests and 16 integration tests.
  Existing HTTP mocks and a Unix socket fixture used authorized localhost/socket
  access. Final log: `/tmp/ail-files-verified-tests.log`.
- `cargo clippy --all-targets -- -D warnings`, formatting and diff checks pass.

The broad active goal remains open. This slice supplies regular UTF-8 files with
caller authority; streams, binary buffers, directory APIs, nested collections,
maps, richer functions/values, arbitrary precision, ecosystem compatibility and
full ASD-STE100 conformity remain unproven.
