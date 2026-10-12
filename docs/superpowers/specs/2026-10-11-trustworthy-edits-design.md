# Trustworthy AIL edits

## Purpose and evidence

AIL must let an agent extend and deliberately change a program without silently
discarding existing behavior or protections. The current isolated checker accepts
a tiny replacement for the support-ticket package. The existing semantic diff
also reports equality when arithmetic changes an observable result from 9 to 5,
when calls are reordered, and when a call is duplicated. These are actual local
mock-model and executable reproductions, not hypothetical risks.

Audit evidence is in `/tmp/ail-edit-guard-probe.json`,
`/tmp/ail-edit-semantics-audit.md`, and `/tmp/ail-edit-workflows-audit.md`.
The initial repository snapshot is `/tmp/ail-edit-baseline.json` (1,810 files),
with copies under `/tmp/ail-edit-baseline` and executable
`/tmp/ail-edit-baseline-ail`.

This milestone supplies structural edit admission, not a proof that arbitrary
different programs are observationally equivalent. New entry points can expose
new behavior. The accepted claim is that an existing component's checked
structure is preserved, or that the candidate exactly matches a separately
specified checked target.

## Constraints and authority

- Keep the AIL-Spec → checked Core → STE → verified bytecode → VM/native pipeline.
- No new dependencies. Preserve all previous capability and runtime contracts.
- Do not modify frozen benchmark/oracle inputs or weaken their assertions.
- The capability ladder remains 19/19; this is an authoring correctness milestone.
- Work in the existing checkout; no git mutations, worktrees, resets, or commits.
- Preserve unrelated uncommitted work. Record changed files against the snapshot.
- Standing user authorization to continue the language and developer instructions
  authorize implementation and reversible checks. Skill approval/commit/worktree
  steps do not introduce additional permission gates for this work.

## One interpretation of checked Core order

Core edge ordinals already express semantic order. Canonical rendering uses them,
but document projection presently reads some vectors in storage order. A checked
legacy Set/Compute graph and its storage permutation have the same canonical
text/hash yet compute 9 and 7. Canonicalize the Core view once at the document
projection boundary, using the same ordered view for generic and specialized
decoders. An already ordered graph should avoid an unnecessary full clone.

Define an unambiguous representation: reject duplicate edge ordinals
with a specific checker diagnostic, including provenance relations. Some legacy
payloads are recovered from the first provenance attachment, so provenance order
cannot be assumed universally irrelevant. Graph mutation must append above the
maximum retained ordinal after removals, rather than using vector length.
Do not use opaque edge IDs to choose executable order. Gaps in ordinals are legal.
Explicit step indices remain authoritative for their existing operation families;
ordinal fallback and decoder operation indices must remain mutually consistent.
Storage permutations with unchanged ordinals must produce the same projected
document, STE, verified bytecode, and observable execution.

## Exact structural comparison

Keep the existing public comparison and report entry points, moving their
implementation into a focused module. Extend the diff with changed, invalid,
and preservation-violation categories; retain removed/added/retyped fields.

Both inputs must pass Core checking. Duplicate semantic `(kind, name)` labels,
duplicate node IDs, missing endpoints, and ambiguous ordering must fail closed.
An invalid graph must never compare equal to itself. This precondition also
applies to saved baseline Core loaded by the CLI.

The semantic projection includes exact case-sensitive kinds, names, optional
types, all node and edge attributes, edge occurrences, owner-local edge order,
and package metadata. Exclude evidence-only Provenance nodes and `has_provenance`
edges, while retaining any executable payload derived from them. Some legacy
reads, writes, and failure handling recover instructions from provenance names;
compare their effective payload and selection order before omitting evidence.
Attributes named `provenance` are not globally ignorable: legacy execution and
checking interpret them. Keep all attributes conservatively, including these.
Obtain the canonical projected document once per input and retain additional
owner-local semantic vectors for Action reads/writes, Tool reads/writes/calls,
CompilerPass reads/writes (including its existing declaration filtering), and
Failure handling. These are all consumers of executable provenance fallback.
Comparing their exact effective strings catches changes in node-level provenance
precedence, first attached evidence, and prefix stripping without duplicating the
compiler's fallback algorithm. Do not search for the first matching prefix when
the compiler chooses the first attachment and then strips that prefix.
Opaque graph IDs are remapped to exact semantic labels. Remap attribute values
only if the actual schema defines them as graph references; never normalize
arbitrary literals or user text. Keep artifact hashing unchanged: provenance is
still part of the existing patch base hash. Metadata is conservatively exact,
including authoring fields, until a separately justified exclusion is specified.

Do not use absolute global ordinal numbers in equality: adding a declaration can
shift those numbers. Compare occurrence-preserving ordered relations within each
owner/relation, plus existing explicit operation/step attributes. Do not collapse
duplicate calls into a set. Report deterministic, reviewable expected/actual
changes and the affected owner. Full structured equality, not a digest alone,
decides equivalence.

Known conservative limit: typed-expression Core→Spec rendering can insert
parentheses while retaining identical executable bytecode, but re-elaboration
changes the original writes.provenance policy text. Exact structural comparison
rejects that pair under this contract. Preserve effect phrasing or explicitly
patch the policy text; do not report general executable equivalence from this
structural predicate. Blank-line/layout formatting that leaves Core facts intact
remains admissible.

`is_equivalent()` means no semantic difference and no invalid input.
`preserves_baseline()` additionally allows only the additions described below.
The CLI `ail-equivalence` uses strict equality by default; explicit
`--preserve-baseline` requests the directional admission predicate. The existing
package syntax and advertised `--core-file` candidate syntax both work, and
conflicting candidate selectors are rejected.

## Permitted additions

Every existing semantic node, attribute, type, and complete outgoing relation
sequence is frozen. Existing actions cannot gain a write, requirement, return,
trace, call, or failure link implicitly. Existing records and variant sets cannot
gain fields or constructors implicitly. Existing package metadata is frozen.

The sole existing-root exception is `Application.contains`: it may acquire a
relation to a genuinely new permitted declaration. Existing children retain their
relative order. New declarations may own new bodies and reference old fields,
functions, types, and traces. Determine ownership using schema relation roles,
not name prefixes. Account for unanchored Type declarations explicitly. Reject
orphan executable fragments and new relations that alter existing owners.

Implicit new declaration roots are limited to ordinary Action, Function, and
Type, together with their owned helper nodes. CompilerPass, Failure, Thing, User,
Tool, UI, system, and foreign roots require an explicit target. In particular,
adding a Failure named PermissionDenied can change the failure selected by an
unchanged existing permission requirement. Do not classify that as safe merely
because its graph nodes are new. Reject new action/function runtime-name
collisions and new type-head collisions (including builtin reserved heads).

When the baseline lowers successfully, also require the candidate to lower and
preserve each existing bytecode action/function record, failure record, and
existing concrete type schema. Additional entries are permitted. A baseline that
cannot lower retains the narrower checked-graph/effective-payload contract; do
not pretend this supplies a runtime proof, and do not reject useful structural
comparison solely because a nonexecutable profile cannot lower. Avoid this extra
lowering work when there are no new declarations.

New declarations are additions, so they do not satisfy strict equality. The
policy must admit an independent new action, executable function, and nominal
type, while rejecting a step appended to an existing action.

## Explicit changes and immutable admission context

Use three explicit contexts: Greenfield, PreserveBaseline, ExpectedPatchTarget.
Source packages use one checked baseline captured from the loaded package and
its already resolved imports. Invalid nonempty source is an error. Greenfield is
available only through an explicit API/CLI decision for a metadata-only empty
source package; an arbitrary parse failure never chooses it.
The CLI decision is `--greenfield` on draft/spec/story/package-build. Require an
empty local source and reject combination with `--expected-patch` or `--baseline`.
Without that flag, an empty baseline fails with guidance to choose greenfield
explicitly. Existing imported packages are still captured once and merged into
every candidate; greenfield does not authorize changing imported metadata.

Optional caller-supplied `--expected-patch <path>` is an `ail-core.patch.v0`
document applied to that checked baseline before any model request. Its existing
package and base_hash preconditions must pass, and its result must check clean.
The final candidate must match the checked expected target exactly, including
the absence of unrelated additions. A generated patch, requirements prose,
model envelope, or prompt text cannot change the admission policy.

This mode authorizes changes expressible by the existing Core patch schema.
It does not invent package-metadata replacement or occurrence-addressed patch
operations. Unsupported intentional changes require later explicit editing
contracts; there is no unchecked semantic-change override.

Keep existing public drafting APIs as preserve-by-default wrappers. Add context
variants for draft, requirements-to-Spec, recorded/max-token generation, and
repair. Reuse the same immutable context throughout retries. Prompts provide the
source baseline or expected target so models can satisfy the contract; prompt
instructions never substitute for checking.
This admission default applies to APIs returning executable Spec candidates.
Standalone requirements generation/repair produces prose, so retain its existing
ability to work with an empty source. Context-aware requirements variants may
carry the selected target into guarded build/story workflows; they do not make
requirements prose an authorization artifact.

Use the same import snapshot for source and candidate elaboration. Detect source
snapshot mismatch rather than publishing evidence from a separately reread
package as if it were the admitted baseline.

## Admission and publication boundaries

Guard direct `ail-draft`, `ail-spec`, `ail-story`, model `ail-build`, and package
`ail-build --spec-file` before any AcceptSpec event, compiler execution, native
output, or accepted artifact publication. A rejected candidate can be returned
as clearly labeled diagnostic evidence. It cannot be published as accepted.

Ordinary isolated check/compile/run and deterministic patch commands remain
usable. Standalone `ail-build --core-file` has no source-edit claim unless an
explicit `--baseline` is supplied; `--expected-patch` without a usable baseline
is an error. Do not accept flags that are silently ignored. Preserve standalone
Core compilation compatibility.

Successful authoring artifacts include a deterministic admission report with
policy, baseline artifact hash, optional expected-target hash, structural diff,
and admitted status. Bind the report into the build artifact manifest. Replace
story's unconditional executable acceptance claim with the real admission
result; story anchor coverage remains separate evidence.

Explicit compiler passes run after authoring admission. Record them as separate
checked transformations; the authoring proof covers the admitted pre-pass Core
and must not claim unchanged behavior for arbitrary pass output. An admission
or source-snapshot rejection does not overwrite accepted artifacts from an
earlier successful run. All such rejection checks precede the first accepted
publication write. This supplies pre-publication consistency, not atomic
filesystem isolation or rollback from arbitrary publication I/O failures. Fresh
artifact directories remain absent of accepted files on rejection; report the
current run's failure unambiguously.

## Diagnostics and validation

Use `AIL-EDIT-*` diagnostics for invalid baselines/targets and rejected semantic
changes. Include a bounded useful summary and concrete component changes.
Existing draft JSON must expose the diagnostics. Admission reports distinguish
checker validity, baseline preservation, and exact expected-target matching.

Permanent tests first reproduce arithmetic/literal/function/variant changes,
order and call multiplicity, omitted protections, case-sensitive types, metadata,
invalid baselines, and candidate saved-Core dispatch. Test provenance and ID
changes, storage permutation, harmless formatting, permitted new declarations,
and explicit expected changes positively. Rejected edits must leave the source
and accepted/native publication untouched. A stale patch must fail before network.

Independent end-to-end cases execute the original and deliberately changed
program through STE, VM, and native output to prove that the guard detects real
observable changes. Also prove an intended checked change can be admitted and
executed. Full Rust, existing differential, benchmark, and broad VM/native gates
are required because the shared Core projection boundary changes. Record frozen
input hashes, commands, counts, failures, scope, and limitations. The larger
Python-replacement goal remains active after this milestone.
