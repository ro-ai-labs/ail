# Trustworthy edits

Status: implemented authoring admission; `ail-core.patch.v0` authorization.
The capability ladder remains **19/19**. This milestone improves authoring
correctness; the larger goal of making AIL a usable Python replacement remains
active. The executable pipeline remains:

```text
AIL-Spec -> checked AIL-Core -> AIL-STE -> verified bytecode -> VM / native ELF
```

## Admission policy

Checking a candidate in isolation establishes validity. It does not establish
that replacing an existing program preserves its behavior or protections.
Package `ail-draft`, `ail-spec`, `ail-story`, model `ail-build`, and
`ail-build --spec-file` therefore capture a checked source baseline and resolved
imports before generation or acceptance. Three policies are available:

| Policy | Authority | Candidate must satisfy |
| --- | --- | --- |
| `PreserveBaseline` (default) | Captured checked source | Preserve existing structure; add only permitted independent declarations |
| `ExpectedPatchTarget` | Caller-supplied patch applied to that baseline | Exactly match the checked target, with no unrelated additions |
| `Greenfield` | Explicit choice for an empty local source | Check the new program; preserve package metadata and imported components |

Requirements prose, prompts, model responses and repair candidates cannot select
a policy or authorize a change. Repairs and retries reuse the same immutable
context. An invalid nonempty source is an error. Without explicit greenfield,
an empty source also fails; a parse failure never chooses greenfield.
Standalone requirements generation/repair still produces prose and retains its
empty-source compatibility. Prose is not executable edit authority.

Use `--expected-patch <path>` on package draft, Spec, story or build to authorize
an intentional change. A supplied package name in the `ail-core.patch.v0`
document must match the captured baseline's package. The mandatory `base_hash`
must also match that baseline; the patch must apply successfully and produce
checked Core. These checks occur before a model request. The final candidate
must match that target exactly, including retained policy attributes.

Use `--greenfield` on those package authoring commands only with empty local
source. It conflicts with `--expected-patch` and `--baseline`. Already resolved
imports remain protected. Local greenfield declarations can change the way an
imported action lowers even when its graph is unchanged, so admission also
checks existing compiled import records against the full candidate when the
import baseline can lower. If that baseline cannot lower, the narrower
structural/effective-payload contract applies.

## Equality and permitted additions

`compare_ail_core_semantics(baseline, candidate)` returns a deterministic
structured diff. `is_equivalent()` requires strict equality of checked semantic
facts; `preserves_baseline()` is directional and permits a limited extension.
Both inputs must check. Invalid graphs never compare equal, even to themselves.
Duplicate semantic labels, duplicate node IDs, missing endpoints and ambiguous
edge ordinals fail closed.

The comparison retains case-sensitive kinds, names and types, all node and edge
attributes, package metadata, and ordered relation occurrences for each owner.
It preserves call multiplicity and executable fallback payloads. Opaque graph
IDs are mapped to semantic labels; only schema-defined graph references are
remapped. Arbitrary text and literals remain exact. Global ordinal gaps or
shifts are harmless when each owner's relation order remains unchanged. Core
storage order does not select execution order. Duplicate edge ordinals reject,
including duplicates on provenance relations.

A preservation candidate can add an ordinary Action, Function or Type with its
owned helper nodes. New bodies can refer to old fields/functions/types/traces.
Existing Application containment can link genuinely new permitted declarations,
while keeping existing children in relative order. Unanchored Type declarations
are accounted for explicitly. New runtime action/function names and nominal type
heads must not collide with existing names or reserved builtin heads.

Every other existing owner remains frozen. Adding an operation, requirement,
trace, return, field, constructor or failure link to an old owner requires an
explicit target. Orphan executable fragments reject. New Failure, CompilerPass,
Thing, User, Tool, UI, system or foreign roots also require an explicit target.
For example, adding `PermissionDenied` can change which failure an unchanged
permission requirement selects, so a new Failure is not implicitly safe.

For a lowerable baseline with new declarations, admission also requires the
candidate to lower and preserve each existing bytecode action/function record,
failure record and concrete type schema. Additional entries are permitted.
The report distinguishes `existing-records-preserved`, `candidate-failed`,
`existing-records-changed`, `baseline-unavailable`, and `not-required` (no
supplemental lowering required). A checked descriptive or specialized profile
that cannot lower can still be structurally compared; its report explicitly
states `structural/effective-payload-only`.

## Proof limits and provenance

This is a checked structural contract, not a proof of observational equivalence
between arbitrary different programs. New entry points expose new behavior.
Even equal bytecode is insufficient to authorize a changed checked policy fact.

Evidence-only Provenance nodes and `has_provenance` edges are omitted from the
semantic facts only after executable payload selection has been accounted for.
Legacy reads, writes, tool calls, compiler ports and failure handling can obtain
payloads from provenance. The effective selected strings and their order remain
protected. Node or edge attributes named `provenance` are retained, because they
can encode executable instructions or policy. Artifact hashes still include
provenance; semantic equality does not imply an identical patch base hash.

Core-to-Spec rendering can insert parentheses into typed expressions. Rechecking
that Spec can retain identical bytecode while changing a `writes.provenance`
policy string, so strict equality and preservation reject the pair. Preserve the
effect phrasing or explicitly patch that policy text. Blank lines and layout
changes that leave checked facts intact are accepted. The historical broader
[round-trip goals](11-round-trip-equivalence.md) do not enlarge this implemented
admission predicate.

The current patch schema cannot replace package metadata or address an individual
occurrence of repeated identical edge selectors. Some intentional edits therefore
need a future editing contract. There is no unchecked semantic-change override.

## CLI and API

Compare a candidate against a package or saved Core:

```bash
cargo run -- ail-core bench/R001-add/program.ail > /tmp/add.ail-core.txt
cargo run -- ail-equivalence --core-file /tmp/add.ail-core.txt \
  --baseline bench/R001-add/program.ail
cargo run -- ail-equivalence bench/R001-add/program.ail \
  --baseline /tmp/add.ail-core.txt --preserve-baseline
```

Strict equality is the CLI default. `--preserve-baseline` requests the directional
policy; a permitted addition passes only that mode. Package and `--core-file`
candidate selectors conflict. A rejected comparison exits unsuccessfully and
prints a reviewable diff.

Standalone saved Core compilation stays available without a source-edit claim:

```bash
cargo run -- ail-build --core-file /tmp/add.ail-core.txt \
  --action AddNumbers --target linux-x86_64-elf --out /tmp/add-standalone
```

To enforce preservation for that build, add
`--baseline <package-or-saved-Core>`. To authorize a specific target, also add
`--expected-patch <caller-supplied-patch>`. A standalone expected patch requires
a baseline. A positional source package combined with `--core-file` rejects;
use the pathless form above. Package build obtains its baseline from the package
and rejects a separate `--baseline`. Unsupported/conflicting authoring flags
reject rather than being silently ignored. Ordinary isolated check, compile,
run, deterministic Core-to-Spec and patch commands retain their own contracts.

Public APIs in `ail::ail` expose `AilEditContext::preserve_package`,
`preserve_core`, `expected_patch`, `expected_core_patch`, and `greenfield`.
`check_candidate` returns admission diagnostics and `render_admission` renders
deterministic evidence. Existing executable drafting APIs preserve by default;
`*_with_edit_context` variants support the explicit immutable context across
drafting, recorded/max-token generation, requirements guidance and repair.
The package context captures resolved objects and validates candidate elaboration
against that snapshot; API callers remain responsible for filesystem publication.
The CLI adds filesystem source-consistency barriers.

## Diagnostics and publication

`AIL-EDIT-BASELINE` identifies invalid, ambiguous or improperly empty authority.
`AIL-EDIT-TARGET` identifies a patch/precondition/target failure.
`AIL-EDIT-REJECTED` identifies a candidate that violates the selected policy;
`AIL-EDIT-SNAPSHOT` identifies source/import/registry inconsistency. Summaries are
bounded and name concrete component differences. Draft JSON includes diagnostics.
Reports distinguish checker validity, preservation and exact target matching.

A rejected CLI build can emit only the bounded diagnostics; it need not write an
admission artifact. To inspect the complete diff, run `ail-equivalence` with the
candidate and captured baseline, or with the checked expected-target Core as
`--baseline`. The API can render full evidence through
`AilEditContext::render_admission(candidate)`. Successful builds bind the report
as described below; diagnostic rejection is not successful publication.

Admission precedes AcceptSpec, compiler passes, native output and accepted artifact
publication. Successful authoring builds write `admission.ail-edit.txt` with policy,
baseline and optional target artifact hashes, candidate hash, structural diff and
admitted status; the build manifest binds its fingerprint. Story evidence uses
the checked admission result. Story-text anchor coverage remains separate evidence
and does not authorize an executable amendment.

Explicit compiler passes run after admission. Evidence records each checked
transformation separately, its input/output Core hashes, and the authoring proof's
pre-pass scope. An arbitrary pass is not claimed to preserve program behavior.

The CLI captures local and recursive imported Spec/manifest objects and raw bytes,
plus registry bytes. Every admission or source-snapshot rejection occurs before
the first accepted write. A final shared source-consistency barrier immediately
precedes build publication. Rejection preserves previous accepted artifacts and
native output; a fresh rejected run publishes no accepted artifacts. This supplies
pre-publication consistency. It does not provide atomic filesystem isolation,
prevent another writer changing files after the barrier, or roll back arbitrary
later output I/O failures.

## Independent verification

```bash
python3 scripts/ail_edit_differential.py --artifact-dir /tmp/ail-edit-oracle-evidence
```

The destination must be new or empty. The script builds the current library and
CLI, selects their exact Cargo-reported artifacts, and compiles a temporary Rust
probe against that library. It generates its own source pairs and expected
results without importing compiler/runtime code or transient fixtures. Source,
saved Core and STE execute through VM and Linux x86_64 native ELF. Arithmetic,
literals, function bodies, variant constructors, call order/multiplicity,
permissions, traces, type case and metadata test rejection; exact caller-authored
patch targets test intentional change. Neutral IDs, evidence-only provenance,
storage/ordinal transformations and formatting, plus independent declarations,
test the positive boundary. The probe also tests invalid self-comparison and
executable provenance fallback. A real CLI build checks authorized output and
preservation of earlier accepted files after rejection. `report.json` records
commands, counts, runtime expectations and binary/library/script hashes.

Existing differential families and the frozen capability benchmark remain
separate gates; this oracle does not replace or modify them. Full Rust, existing
oracles, the 19-rung benchmark and broad VM/native verification remain required
for milestone completion. The new guard adds no capability rung.
