# AIL

AIL means Agentic Intent Language.

This repository contains the AIL specification, examples, compiler, bytecode
VM, and native Linux ELF toolchain. AIL starts from developer intent, captures
requirements through an AI-assisted programming agent, normalizes accepted
programs into AIL-Core IR, and compiles checked artifacts into executable
behavior. The executable pipeline now includes controlled English:

```text
AIL-Spec -> checked AIL-Core -> AIL-STE -> verified bytecode -> VM / native ELF
```

AIL-STE is an executable programming profile based on ASD-STE100 Issue 9.
It uses fixed, short instruction templates and separately declared technical
data. It does not claim full ASD-STE100 conformity. See the
[STE reference](docs/ail/33-ste-intermediate.md) for the grammar and agent workflow.

AIL now computes checked 64-bit integer expressions in both the VM and native
ELF output, with comparisons, branches, loops and recursive functions.
Functions now accept and return integers, floating values, Booleans, Unicode text, typed lists
and recursive JSON, with isolated locals and reusable transformations.
Actions and functions also join, count, slice, search, split, replace and trim
Unicode text on both backends, including joining typed text lists.
Typed integer, floating and text lists support creation, length, indexing, append and
replacement, including map/reduce-style loops.
Lists also support slicing, concatenation, reversal and stable ascending or
descending sorting for numeric and text data.
Programs can create typed integer, floating, boolean, text and list constants directly
from JSON literals in source. Actions can also read and write bounded UTF-8
files through explicit caller-bound resources. Typed JSON parsing and formatting
connect those files to scalar and list computations.
Recursive Json values preserve nested records and lossless number tokens;
typed JSON Pointer lookup and collection counts make those records accessible.
Kind inspection and ordered key enumeration support traversal of unknown records.
Boolean predicates combine flags and typed comparisons with short-circuit logic.
JSON set and remove operations transform records and build reports from computed
values, including appending nested records to an array.
Binary64 Float computation supports fractions, scientific notation, explicit
integer conversions, typed function values and finite JSON number conversion.
The frozen capability ladder passes **23/23** rungs. It is not yet a Python
replacement. See [integer computation](docs/ail/34-integer-expressions.md) and
[control flow](docs/ail/35-control-flow.md), and
[integer functions](docs/ail/36-integer-functions.md), and
[text operations](docs/ail/37-text-operations.md), and
[typed lists](docs/ail/38-typed-lists.md), and
[typed literals](docs/ail/39-typed-literals.md), and
[file resources](docs/ail/40-file-resources.md), and
[JSON conversion](docs/ail/41-json-conversion.md), and
[recursive JSON](docs/ail/42-recursive-json.md), and
[JSON mutation](docs/ail/43-json-mutation.md), and
[value functions](docs/ail/44-value-functions.md), and
[floating-point computation](docs/ail/45-float-computation.md), and
[floating lists](docs/ail/46-floating-lists.md), and
[JSON inspection](docs/ail/47-json-inspection.md), and
[Boolean predicates](docs/ail/48-boolean-predicates.md), and
[text transformations](docs/ail/49-text-transformations.md), and
[list transformations](docs/ail/50-list-transformations.md) for syntax and current limits.
[Runtime recovery](docs/ail/51-runtime-recovery.md) adds explicit handlers for
data errors, with a reusable [JSON-lines batch example](examples/recovering-json-lines).
[Standard streams](docs/ail/52-standard-streams.md) add bounded stdin and exact
stdout/stderr for a [JSON-lines filter](examples/json-lines-filter) with `--stdio`.
[Process resources](docs/ail/53-process-resources.md) add caller-bound tools with
argument lists, captured output, exit status and recoverable timeouts, used by
the [AIL tool coordinator](examples/tool-coordinator).
[Directory resources](docs/ail/54-directory-resources.md) add sorted discovery and
computed relative reads/writes under caller roots, demonstrated by the
[discovered-file reporter](examples/directory-report).
The [expression calculator](examples/expression-calculator/README.md) implements
tokenization, parsing and checked evaluation in AIL itself, with an imported
library and JSON-lines interface. Its [reference](docs/ail/55-expression-calculator.md)
defines grammar, structured errors and bounds.

The active documentation starts at [docs/README.md](docs/README.md).
The complete AIL specification suite is indexed at
[docs/ail/README.md](docs/ail/README.md), including execution semantics,
AIL-Core schema, prompt pack, package model, C interop, backend portability,
UI profile, diagnostics, safety, corpus, and traceability artifacts.

[Bytes values and binary files](docs/ail/56-bytes.md) add arbitrary octets, indexed
updates, slices, concatenation, comparison and explicit UTF-8/hex conversion to
typed functions and both execution backends. The [binary file example](examples/binary-files)
inspects PNG header metadata and writes an exact copy with caller-granted paths.

[Explicit errors](docs/ail/57-explicit-errors.md) let pure libraries reject domain data
through ordinary recovery. The [run-length codec](examples/run-length-codec/README.md)
implements binary encoding and decoding in imported AIL functions, with real file
entry points and an independent R027 benchmark.

[Executable variants](docs/ail/59-executable-variants.md) add recursive user-defined
values, concrete generic types, checked construction/access and typed function
boundaries. The [binary tree](examples/binary-tree/README.md) implements insertion,
search, height and in-order traversal in a reusable AIL library.

[Trustworthy edits](docs/ail/60-trustworthy-edits.md) protect existing checked
program structure during agent drafting, repair, story and build. Authoring preserves the
source by default; intentional changes require a caller-supplied expected patch.
Explicit greenfield authoring requires empty local source. This authoring
correctness milestone adds no ladder rung; the larger Python-replacement
goal remains active.

## Toolchain

The current compiler accepts deterministic AIL package artifacts, not free-form
conversation. The LLM-facing agent path is responsible for interviewing the
application developer, capturing sufficient requirements, converting those
requirements into AIL-Spec, elaborating AIL-Spec into AIL-Core, and invoking
the compiler pipeline.

The supported AIL commands are:

- `ail-check`
- `ail-ste`
- `ail-core`
- `ail-flow`
- `ail-flow-edit`
- `ail-lower`
- `ail-compile`
- `ail-run`
- `ail-vm`
- `ail-conformance`
- `ail-interview`
- `ail-requirements`
- `ail-spec`
- `ail-draft`
- `ail-build`
- `ail-pass`
- `ail-bootstrap`
- `ail-patch`
- `ail-equivalence`
- `ail-story`

Use the commands through Cargo during development:

```bash
cargo run -- ail-check examples/support_ticket.ail
cargo run -- ail-core examples/support_ticket.ail
cargo run -- ail-flow examples/support_ticket.ail
cargo run -- ail-interview examples/support_ticket.ail --prompt "Build a support ticket app"
cargo run -- ail-lower examples/support_ticket.ail
cargo run -- ail-run examples/support_ticket.ail --action CloseTicket ticket.status=Open
```

Export and execute the controlled-English intermediate, or author it directly:

```bash
cargo run -- ail-ste bench/R000-increment/program.ail > /tmp/counter.ail-ste
cargo run -- ail-check /tmp/counter.ail-ste
cargo run -- ail-run /tmp/counter.ail-ste --action IncrementCounter counter.value=41
cargo run -- ail-compile /tmp/counter.ail-ste --action IncrementCounter \
  --target linux-x86_64-elf --out /tmp/counter
/tmp/counter counter.value=41
```

[examples/ste/counter.ail-ste](examples/ste/counter.ail-ste) is a manually authored
example. STE export, lowering, build, and compile bundles include
`artifact.ail-ste` and a companion `manifest.ail-ste.txt` alongside bytecode.

Checked AIL-Core can also render back to deterministic AIL-Spec for review or
agent handoff:

```bash
cargo run -- ail-core examples/support_ticket.ail > /tmp/support-ticket.ail-core.txt
cargo run -- ail-spec --core-file /tmp/support-ticket.ail-core.txt
```

AIL-Flow and agent graph patches can apply directly to a saved checked
AIL-Core artifact:

```bash
cargo run -- ail-patch --core-file /tmp/support-ticket.ail-core.txt /path/to/edit.ail-core.patch.json
cargo run -- ail-flow-edit --core-file /tmp/support-ticket.ail-core.txt /path/to/edit.ail-flow.edit.json
```

## Machine Bytecode

For Linux, the machine-level bytecode target is a native ELF executable.

```bash
cargo run -- ail-compile examples/support_ticket.ail \
  --action CloseTicket \
  --target linux-x86_64-elf \
  --out /tmp/close-ticket
```

The compiler emits ELF64 x86_64 executable bytes directly for the supported
native subset. The saved AIL bytecode JSON artifact remains an auditable
intermediate for checker, agent, and VM workflows; it is not presented as the
Linux machine-level bytecode target.

Artifact bundle commands such as `ail-lower`, `ail-compile`, `ail-build`,
`ail-pass`, `ail-conformance`, and `ail-bootstrap` can write fingerprints,
manifests, native bytecode reports, dependency reports, checked AIL-Core, AIL
bytecode, native ELF outputs, and AIL-authored agent traces with
`--artifact-dir <dir>`.

## LLM Agent Path

The local LLM endpoint can be supplied with `--llm-endpoint`. The intended
development loop is:

```bash
cargo run -- ail-requirements examples/support_ticket.ail \
  --prompt "Build a support ticket application" \
  --llm-endpoint http://192.168.60.171:8091/v1/chat/completions

cargo run -- ail-spec examples/support_ticket.ail \
  --requirements-file /path/to/requirements.ail-requirements.md \
  --prompt "Draft the support ticket specification" \
  --llm-endpoint http://192.168.60.171:8091/v1/chat/completions

cargo run -- ail-build examples/support_ticket.ail \
  --prompt "Build a support ticket application" \
  --target linux-x86_64-elf \
  --action CloseTicket \
  --artifact-dir /tmp/ail-build \
  --llm-endpoint http://192.168.60.171:8091/v1/chat/completions
```

For llama.cpp-server, the hosted development model is exercised through
`--llm-endpoint http://192.168.60.171:8091/v1/chat/completions`. Root URLs
such as `http://192.168.60.171:8091/` are accepted as shorthand for
`/completion`, and explicit `/completion` paths are also supported, but the
chat-completions path gives the current hosted model the strongest
artifact-format behavior. Chat completion requests disable thinking through
`chat_template_kwargs`.

The agent and base LLM are untrusted proposal mechanisms. The trusted boundary
is checked AIL-Core, immutable edit admission, the compiler, verifier, manifests,
reports and fingerprints generated from deterministic artifacts. Isolated
checker validity does not authorize replacing an existing program. Package
authoring captures checked source and imports before generation and checks every
repair against the same preservation or exact expected-target policy.

LLM responses may be raw deterministic artifacts or the prompt-pack JSON
envelope with `artifact_text`. If the envelope contains blocking `questions`
instead of an artifact, the command surfaces those questions and stops before
checker, repair, or compile stages. Malformed envelopes are rejected as
`AIL-PROMPT-001` prompt protocol errors. Envelope metadata must match the
requested artifact kind and package profile, and must keep checker handoff
enabled.

## Examples

AIL packages live under [examples](examples). The current tree contains 33 package directories
with the `.ail` suffix, plus the newer computation examples and the 128-entry replay catalog in
[`examples/examples.md`](examples/examples.md). Current release replay evidence
reports `checker-result-count accepted 119` and `checker-result-count rejected 9`.
Each counted catalog entry is end-to-end: stored request/response or package
source, checked artifact, checked AIL-Core, bytecode, VM trace, and binary or
target-contract evidence.

Application and workflow packages:

- `ail_std_collections.ail`
- `ail_std_core.ail`
- `ail_std_csv.ail`
- `ail_std_effects.ail`
- `ail_std_expression.ail`
- `ail_std_json_pretty.ail`
- `ail_std_lru.ail`
- `ail_std_rle.ail`
- `ail_std_runtime.ail`
- `ail_std_security.ail`
- `ail_std_tree.ail`
- `ail_std_words.ail`
- `darwin_linux_effect.ail`
- `incident_identity.ail`
- `incident_policy.ail`
- `incident_response.ail`
- `missing_registry_import.ail`
- `option_map.ail`
- `recursive_factorial.ail`
- `repeated_task.ail`
- `runtime_generic.ail`
- `secret_access.ail`
- `stateful_counter.ail`
- `support_composed.ail`
- `support_shared.ail`
- `support_ticket.ail`

Specialized profile packages:

- `ail_toolchain_agent.ail`
- `c_interop.ail`
- `compiler_pass.ail`
- `incident_notifications.ail`
- `network_driver.ail`
- `refund_tool.ail`
- `ui_workflow.ail`

Each package has an `ail-package.md` manifest and a `spec.ail-spec.md` entry
spec. Accepted and rejected conformance fixtures live under package-local
`examples/accepted` and `examples/rejected` directories where present. The
active package inventory is [docs/ail/25-example-inventory.md](docs/ail/25-example-inventory.md);
the replay and learning guide entry point is [examples/README.md](examples/README.md).

## Verification

Use these checks before claiming a cleanup or language change is complete:

```bash
cargo fmt --check
cargo check
cargo test --test ail_toolchain
cargo test
cargo clippy --all-targets -- -D warnings
git diff --check
python3 scripts/ail_edit_differential.py
```

Before calling AIL v0.3 complete, run the bundled release audit:

```bash
python3 scripts/run_v03_release_audit.py --bundle-root /tmp/ail-v03-release-evidence
```

The [CSV aggregation application](examples/csv-aggregate/README.md) parses quoted
CSV and aggregates checked integer totals in a pure AIL library, with JSON-lines
and caller-granted file reports. See its [reference](docs/ail/61-csv-aggregation.md).

The [word-frequency application](examples/word-frequency/README.md) counts exact
Unicode whitespace words through a pure AIL library and bounded raw stdin.
See its [reference](docs/ail/62-word-frequency.md) for ranking, request errors and
measured VM/native input limits.

The [JSON pretty-printer](examples/json-pretty/README.md) preserves duplicate member
order, numeric tokens and Unicode through two-space layout and bounded raw stdin.
See its [reference](docs/ail/63-json-pretty.md) for the pure API and Bytes transport.

The [LRU cache](examples/lru-cache/README.md) exposes reusable immutable state and
one-request stdin simulation; see the [API reference](docs/ail/64-lru-cache.md).
