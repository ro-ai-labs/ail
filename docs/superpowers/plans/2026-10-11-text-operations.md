# Text operations implementation plan

**Design:** `docs/superpowers/specs/2026-10-11-text-operations-design.md`.
The user has authorized autonomous continuation of the full active goal.
Implement this milestone in the current shared worktree without discarding
prior work. Native and frontend modules have separate ownership; root owns
integration, the shared VM budget and end-to-end tests.

- [x] Establish failing source -> Core -> STE -> VM/native text tests.
- [x] Add `src/text_operations.rs`: public `AilTextOperation` representation;
  parse/render, Core elaborate/from_core/check, imports, bytecode conversion
  and validation, checked VM evaluation with `&mut usize` shared allocation
  count. Four opcodes and exact runtime semantics follow the design.
- [x] Extend `src/native_expr.rs` with direct native implementations of the
  four opcodes, UTF-8 validation/indexing, immutable bounded allocation and
  real-ELF tests. Retain existing integer and action-call semantics.
- [x] Wire `src/ail.rs`, `src/action_order.rs`,
  `src/ste.rs`: source ordering, typed Core/catalog/checking/saved edges,
  render/import, compilation, strict verification and shared VM allocation
  count through CALL_ACTION/REPEAT_ACTION. Reject unsupported profiles.
- [x] Verify real Unicode results, slice/find boundaries, aliases/mutation,
  nested calls/loops, imported and saved Core, malformed protocols and limits.
  Review integration, then full suite, Clippy, formatting, frozen ladder and
  corpus differential.
- [x] Document source, STE, errors and remaining goal gaps with verified evidence.

Review focus: UTF-8 byte offsets versus character positions; i64::MIN bounds;
operand evaluation before destination mutation; copied text lifetime; shared
allocation budget across nested action calls; duplicate/malformed saved edges.

## Verified outcome

- **631 tests passed** in `cargo test -- --test-threads=1`; existing localhost
  HTTP mock tests ran with the authorized sandbox escalation. Full log:
  `/tmp/ail-text-verified-tests.log`. This includes 46 native emitter tests,
  9 text module tests and 11 text integration tests.
- Clippy with `--all-targets -- -D warnings`, formatting and diff checks passed.
- The frozen capability ladder remains **16/16**, with no case/oracle changes.
- Existing corpus differential: **33 agree, 30 not natively compilable,
  0 diverged**.
- `python3 scripts/ail_text_differential.py`: **605 VM/native/Python cases**,
  9 invalid native UTF-8 forms and 5 saved-Core secret-access probes passed.
  The fixed-seed script preserves the independent review's oracle checks.
- Native adversarial repetitive search first exceeded its three-second test
  deadline; KMP passes the same 1 MiB found/absent cases. Scratch is bounded to
  4 MiB and released per search.
- Review found a secret-access bypass through text operand edges. A failing
  regression now passes after deriving reads/writes from semantic operands,
  including slice bounds, rather than trusting descriptive access edges.
- Standard JSON surrogate pairs now carry non-BMP text correctly; the transport
  regression first failed on a valid emoji escape and now passes while lone
  and mismatched surrogates remain errors.
- Source and direct STE CLI compilation both produce the documented Unicode
  outputs. The normal compile bundle's STE equals the checked-in exported
  example. Documentation: `docs/ail/37-text-operations.md`; examples:
  `examples/unicode-text` and `examples/ste/unicode-text.ail-ste`.

The active objective remains open. This milestone adds usable text computation;
collections, general I/O, richer value/function types, arbitrary precision,
broader Python compatibility and full ASD-STE100 conformity remain unproven.
