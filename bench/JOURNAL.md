# Capability Ladder Journal

Append-only record of what each loop iteration changed and what it bought.
One entry per committed change that moved, or attempted to move, the score.

Format:

```text
## <date> <commit> <score before> -> <score after>
rung:       the rung targeted
capability: the language capability added (not the artifact written)
result:     what happened, including attempts that failed
```

An entry with no score change is still recorded. Attempts that failed are the
most useful entries in this file, because they mark where the estimate was
wrong.

---

## 2026-07-28 da9cdba — -> 1/11

rung: R000-increment (calibration)
capability: none — this established the measurement, it did not add capability
result: Ladder built and scored for the first time. R000 passes on VM and
native across 4 cases; R001 through R010 are unimplemented, each blocked on a
named missing capability. Score 1/11.

Two checker rules landed alongside it, both closing paths by which the
toolchain accepted work it could not perform:

- `AIL-PARSE-001` — `parse_action_bullet` was a `strip_prefix` cascade with no
  terminal `else`. Unrecognized bullets were discarded and `ail-check` reported
  `diagnostics: none`. All 264 spec files in the corpus parse clean, so this was
  a latent hazard rather than active corruption.
- `AIL-EXPR-001` — `ADD_INT_FIELD` lowers only literal operands, so
  "increments the sum total by the sum a" degraded to an opaque `WRITE_FIELD`,
  checked clean, and computed nothing. Now rejected until the expression IR
  lands.

Anti-gaming verified against four attacks: weakening an oracle (exit 3), adding
a softer case (exit 3), deleting a failing rung (exit 3), and breaking
`ADD_INT_FIELD` while leaving the ladder untouched (exit 4, ratchet).

Note on the fourth: that same mutation fails only 3 of 432 unit tests and zero
of the audit's artifact, fingerprint, and manifest checks. The ladder catches it
immediately. That gap is the reason this file exists.

## 2026-07-28 — VM/native differential

capability: none — measurement again
result: `scripts/ail_differential.py` compares the mutation delta and
success/failure outcome of every corpus action across both backends. Baseline
33 agree, 0 diverged. Verified it detects a VM-only mutation (3 divergences,
exit 1).

Three iterations were needed to make it meaningful, and each failure is worth
recording:

1. Comparing raw output flagged 25 false divergences, because the VM prints
   `failure=NotFound` and native prints `failure NotFound`.
2. Running actions with empty state produced 33 agreements that were agreements
   about *failing* — an injected mutation went undetected. Fields are now seeded
   so actions reach their computation.
3. Comparing full state flagged 26 false divergences, because the VM echoes the
   whole final state while native prints only what it wrote. The comparison is
   now the mutation delta, last value per key.

A test that cannot fail proves nothing. Each version of this test was checked
against a deliberately injected divergence before being trusted.

## 2026-07-28 — LLM client hardening and the prompt-harness false green

capability: none — safety of unattended operation
result: Two defects that would have made an unattended loop actively harmful.

`src/llm.rs` had no timeouts of any kind, so a hung endpoint hung the process
forever. It also hardcoded `temperature: 0.0` with no seed; since the server is
bit-deterministic at temperature 0, a failed generation failed identically on
every retry, making retry a no-op. Both fixed, along with `\uXXXX` decoding,
response parsing, chunked transfer-encoding, and a loud failure when the model
returns empty content alongside populated reasoning. Zero dependencies, no
caller signatures changed, 29 new unit tests.

`scripts/run_v03_prompt_llm_harness.py` reported `review-result accepted` for
all 11 prompts while never calling `ail-check` on a single produced artifact. It
now checks every artifact and degrades to `needs-repair` with a non-zero exit
when any is rejected.

Worth recording: on the first live run through the fixed gate, the real model's
spec-draft artifact was rejected by `AIL-PARSE-001` — the rule added earlier the
same day. The diagnostic that closed a latent hole in the parser immediately
caught genuine model output that the old gate had been passing as accepted.

The test fixture for that harness returned the placeholder string
"deterministic prompt harness artifact", which is not a program. With the gate
now real, the fixture had to become a real checkable AIL-Spec. A fixture that
cannot pass a genuine check was evidence of nothing.

## 2026-10-11 — 17/17 -> 18/18

rung: R027-run-length-codec
capability: explicit runtime raises in actions and executable functions, plus a
reusable pure AIL binary codec with encoding, decoding and size queries.
result: All 146 cases pass on VM and native ELF. The independent Python reference
and cases were frozen before implementation, fingerprint
`sha256:2084c99467f7ff315c452cd604436b08`. The previous 17 rung entries and all
275 prior benchmark/script inputs remain unchanged; no dependencies were added.

RLE32 uses a nonzero big-endian u32 count and one payload byte per packet.
The library validates format and result size before constructing output; file
actions preserve existing output when validation fails. The existing 1 MiB Bytes
bound and cumulative 64 MiB shared arena still apply. Source, Core, rendered
source and STE all execute through the VM and standalone native ELF.

Verification: 1,372 Rust tests across 42 targets, strict all-target Clippy,
21 focused differential suites, and the full 18/18 benchmark pass. The RLE
oracle performs 5,064 checks across 5,113 commands, including 12 execution routes,
large files, malformed inputs, exact error categories, mutation detection and
allocation exhaustion. The broad comparison reports 33 agreements, 49 entries
not natively compilable as root actions, and zero divergences. Nine additional
non-root entries are the new library functions, exercised by the dedicated suite.

Independent review caught missing-mode classification and an overly broad error
assertion; both were corrected with failing and passing evidence. The first full
Rust run also caught missing support-package registration and guide metadata.
Those integration fixes passed focused checks and the complete final rerun.
The broader Python-replacement goal remains active.

## 2026-10-11 — 18/18 -> 19/19

rung: R019-binary-tree
capability: executable recursive variants and concrete generic data, with typed
construction, tag tests, field extraction and custom function parameters/results.
result: All 45 frozen tree cases pass on VM and standalone native ELF. The pure
AIL library implements insertion, duplicate multiplicities, search, height and
in-order traversal without a sorting shortcut or host tree implementation.
The independent Python reference and cases were frozen before implementation,
fingerprint `sha256:2366607a2a2761edf25a49af0567ef69`. All previous 18 rung entries
and 426 prior benchmark/script inputs remain unchanged; no dependencies were added.

The same closed schemas and value transports pass through source, Core, rendered
source and STE. Nominal types remain distinct, including imported generic types;
full input validation covers unused recursive payloads and unused parameters.
Malformed values use catchable VALUE, while bounded-resource exhaustion remains
fatal. Constructors and returned custom values retain immutable/atomic result
semantics and the existing shared arena limits. See
[executable variants](../docs/ail/59-executable-variants.md) and the
[tree library](../examples/ail_std_tree.ail/README.md).

Verification: 1,405 Rust tests across 45 targets, strict all-target Clippy,
22 differential families, and the full 19/19 benchmark pass. The variant suite
runs eight complete sections with 4,270 checks across 2,502 commands, including
all 117 frozen/generated tree cases on 12 routes, builtin payloads, generics,
malformed artifacts, recovery, resource limits and mutation detection.
The broad comparison reports 33 agreements, 55 entries not natively compilable
as root actions, and zero divergences. Six additional non-root entries are the
new tree library functions, covered by the dedicated suite.

Independent reviews found and corrected imported-type loss, generic binder
shadowing, borrowed Json canonical-expansion handling, custom nominal slot
conflicts and compatibility of unused descriptive declarations. The final global
run also caught an invalid all-opcode STE fixture alias. A measured calculator
checker slowdown from approximately 1.1s to 6.1s exposed repeated schema resolution
for builtin fields. Exact builtin fast paths and once-per-function source schema
resolution restored checks to approximately 1.1s, with identical output and
independently verified whole-schema, ownership and provenance checks. All final
gates were rerun after that correction; no earlier result substituted for them.
The broader Python-replacement and AI-agent language goal remains active.


## 2026-10-12 — 19/19 -> 20/20

rung: R022-csv-aggregate
capability: reusable pure AIL CSV parsing and checked category aggregation, with
bounded JSON-lines requests and reports written to caller-granted files.
result: All 129 independently frozen cases pass on VM and standalone native ELF.
The Python csv.reader/declarative-grammar oracle was authored and reviewed before
implementation; fingerprint `sha256:421457ed6f6825d12c9d9bb1c268ff25`.
All previous 19 rung inputs/programs, compiler/runtime code and existing scripts
remain unchanged; no dependencies were added.

The imported library preserves quoted CR/LF, doubled quotes, Unicode/NUL and
empty-record distinctions. It validates complete syntax, headers and every row
width before numeric aggregation; checked per-group then global i64 addition
follows input order. Stream retains each original nonblank request and continues
after structured failures. WriteReport checks success before opening/writing the
report, retaining preexisting bytes on domain rejection. Arbitrary I/O failures
do not have a transactional rollback guarantee. See
[CSV aggregation](../docs/ail/61-csv-aggregation.md).

Verification: full unchanged benchmark 20/20; 2,688 independent acceptance checks
across 14 app and 14 parser execution routes, including source, saved Core,
rendered Spec, STE and separately executed/natively compiled saved bytecode.
This comprises 1,806 frozen-case comparisons, 602 direct parser comparisons and 280
file/stream/bound checks, with 154 report-preservation assertions within the latter.
The copied-library sort mutation is rejected by independent output on VM/native. The composed
acceptance records 2,737 commands, including two retained failed harness assertions;
it is not one clean full invocation. Exact ordinary native report assertions
replaced incorrect empty-stderr assumptions, with every changed branch executed.
An additional 14 original-line probes and 80 focused I/O checks pass separately.
Four affected package/docs/STE tests, 139 local links, exact checked-in STE export
and actual support-package closure probes pass. No compiler change justified
repeating the preceding milestone's completed broad compiler gates.

Plain 32,768-scalar fields and 1,024 direct-parser records pass; input content still
matters. A roughly 16 KiB field of 8,192 doubled quote pairs exhausts ordinary fuel,
and excessive JSON depth remains fatal. Existing value/arena/depth/fuel limits
apply, with no universal input-size, linear-time or full Python CSV claim.
The broader Python-replacement and AI-agent language goal remains active.


## 2026-10-12 — 20/20 -> 21/21

rung: R021-word-frequency
capability: reusable pure AIL tokenization and exact word frequencies, with a
bounded raw-stdin tool and deterministic top-five ranking.
result: All 75 independently frozen request cases pass on VM and standalone native
ELF. The explicit-whitespace regex/Counter/sort oracle was authored and reviewed
before implementation; fingerprint `sha256:468062b540458705e855e78c52757374`.
All previous 20 rung inputs/programs, compiler/runtime code and existing scripts
remain unchanged; no dependencies were added.

Words retain case, punctuation, Unicode spelling, NUL, zero-width space and BOM.
Only the pinned 29 whitespace scalars separate tokens. Numeric counts descend;
exact Unicode text breaks ties ascending, independently of input/map order.
Request validation rejects selected duplicate keys, malformed JSON/Unicode and
wrong types while retaining lossless ignored number tokens. Count reads one whole
raw UTF-8 stdin value and emits compact JSON plus LF, including for empty input.
Missing grants, invalid UTF-8 and fatal resource bounds emit no result stdout.
See [word frequency](../docs/ail/62-word-frequency.md).

Verification: full unchanged benchmark 21/21. One clean standalone acceptance run
performs 1,896 checks across 1,939 commands: 1,050 frozen request comparisons,
588 direct token comparisons, 182 raw-stdin successes, 70 failure/bound checks,
four exact ordinary-report smokes and two detected semantic mutations. All 14
app and 14 token routes cover source, saved Core, rendered Spec, STE and separately
executed/natively compiled saved bytecode. The copied-library ranking mutation
runs successfully but returns the wrong result, rejected by independent expected
bytes on VM/native. Nine semantic IR inspections cover purity and prohibited
operations. Earlier Task2 evidence adds 494 token/summary/request comparisons.

Four affected package/docs/STE tests, exact checked-in STE/sample commands and
actual support-loader closure probes pass. The guide test initially exposed missing
mandatory library README sections; a docs-only correction and focused recheck
followed completed behavioral acceptance, with before/after identities retained.
The historical Task2 report's universal timeout wording is qualified by review:
three completed scratch scripts lacked timeout arguments. The permanent acceptance
uses finite deadlines. No compiler change justified repeating the preceding
milestone's completed broad compiler gates.

Single 32,768-scalar ASCII and Unicode words, 1,024 repeated words, 128 distinct
words with ties/repeats and 32,768 separators pass. Exactly 1 MiB of ASCII spaces
succeeds with zero words. A 174,763-byte NUL-only word instead exceeds the escaped
List representation and fails LIST-LIMIT on both backends with no stdout. Existing
value/arena/depth/fuel limits apply; accepted size depends on content. No arbitrary
word cap, linear-time, general NLP or full Python compatibility claim is made.
The broader Python-replacement and AI-agent language goal remains active.


## 2026-10-12 — 21/21 -> 22/22

rung: R023-json-pretty
capability: reusable pure AIL JSON pretty printing with deterministic two-space
layout and a bounded raw-stdin formatter.
result: All 162 independently frozen request cases pass on VM and standalone
native ELF. The recursive tree-render oracle was authored and reviewed before
AIL implementation; fingerprint `sha256:611860d22426e9cba9d97d4ba0dcde8d`.
All previous 21 rung inputs/programs, compiler/runtime code, tests and existing
scripts remain unchanged; no dependencies were added.

The library preserves ordered duplicate pairs, exact number lexemes and decoded
strings while normalizing JSON whitespace and escape spellings. It uses ordinary
AIL text/control operations around the existing compact Json formatter. Invalid
wrappers return REQUEST; malformed selected JSON documents return JSON. Fatal
limits propagate. Analyze encodes its structured Json result as UTF-8 Bytes so
the unchanged benchmark observes every byte, including Unicode line separators.
Format reads one raw JSON document and emits pretty JSON plus one LF. Missing
grants, invalid UTF-8/JSON and fatal limits produce no result stdout.
See [JSON pretty printing](../docs/ail/63-json-pretty.md).

Verification: full unchanged benchmark 22/22. The final complete standalone
acceptance invocation performs 5,044 checks across 5,088
commands: 2,268 frozen request comparisons,
1,246 direct render comparisons, 1,310
raw-stdin successes, 208 failure/bound checks, ten exact
ordinary/raw smokes and two detected semantic mutations. Counts include the
two-backend failure preflight; full matrices use all 14 app and 14 direct-render
routes through source, saved Core, rendered Spec, STE and separately executed/
natively compiled saved bytecode. The copied-library indentation mutation runs
successfully but returns wrong bytes, rejected independently on VM/native.
Actual semantic IR inspections verify purity and permitted action I/O.

Four affected package/docs/STE tests, exact checked-in export/sample commands,
local links and real support-registration positive/negative/restored probes pass.
Preparatory harness history is retained: a corrected recovery-report assumption,
an interrupted partial run ahead of a known fuel-diagnostic assertion mismatch,
and focused corrected preflight before the accepted complete run. The unexecuted
assertion is not reported as an observed failure. Earlier Task2 evidence also
retains an early evaluator launched before report smokes and the distinct accepted
post-smoke run. After the complete behavioral run, permanent dependency capture
was extended to include runtime sources and build.rs. A separate executed identity
gate covers that narrow harness change, retaining predecessor/final script hashes
and unchanged compiler/product inputs. Behavioral and identity evidence are
reported as separate checks. All accepted subprocess checks use finite deadlines.

Separate 32,768-scalar ASCII and Unicode strings/keys, a 5,000-digit integer,
128 scalar values, 128 mixed duplicate pairs and 64 nested arrays pass. Exactly
1 MiB of total stdin containing null plus JSON whitespace succeeds. Depth 65
fails JSON-LIMIT; a 4,000-element scalar array reaches instruction fuel; a
524,673-byte array of 128 long strings reaches the cumulative Text allocation
limit. Input length alone does not determine acceptance. Existing value/arena/
depth/fuel bounds apply; no unbounded formatting, original-byte preservation,
linear-time or full Python compatibility claim is made.
The broader Python-replacement and AI-agent language goal remains active.


## 2026-10-12 — 22/22 -> 23/23

rung: R024-lru-cache
capability: reusable pure AIL LRU state API and bounded raw-stdin request tool.
result: All 78 independently frozen requests pass on VM and standalone native ELF.
The OrderedDict oracle and independent list/manual checks were reviewed before AIL
implementation; fingerprint `sha256:3a45be77224ac7548a556597dbcc0a5c`.
All previous 22 rung inputs/programs, compiler/runtime, tests and existing scripts
remain unchanged; no dependencies were added.

The library exposes create, get, put, run and runRequest. It validates supplied
states, retains caller values, promotes hits and replacements, evicts the prior
least-recent entry, and distinguishes cached null from a miss. Ordered duplicate
payload members, number lexemes and decoded Unicode survive. Direct domain errors
use existing VALUE; the request wrapper returns REQUEST, CAPACITY or OPERATIONS.
Its VALUE recovery covers operation validation only; transition/result failures
and fatal bounds propagate. Analyze uses complete UTF-8 Bytes transport. Run reads
one request through EOF and constructs compact JSON plus LF before its only write.
Structured domain responses exit zero; grant, UTF-8 and fatal errors emit no result
stdout. See [LRU cache](../docs/ail/64-lru-cache.md).

Verification: the unchanged full benchmark passed 23/23 in 1214.63 seconds
with 1305 pinned dependencies. The main standalone acceptance
invocation recorded 3,096 complete selected-result comparisons,
1,132 exact stdin successes and 118 fatal checks
across 4,390 bounded commands. The two semantic mutation comparisons
are included among selected-result checks; they are not added again. All 78 frozen
requests run through Analyze and stdin on all 14 app routes; 142 direct vectors
run on all 14 probe routes. These include source, saved Core, rendered Spec, STE,
and actually executed or separately native-compiled saved bytecode. Exact ordinary
reports, IR purity and permitted I/O are checked separately. A copied-library hit-
promotion mutation returns a valid success report but wrong intermediate events,
which independent expectations reject on VM/native even if final states reconverge.

Focused supplements cover complete direct reports, all-route direct fatal errors,
private-marker leakage, 512 puts at zero capacity and output-depth construction
with independently accepted input controls. Their completed portions and a bounded
continuation compose with the main run; they are not claimed as one invocation.
The main probe labeled result-depth65 actually has input depth66 and proves input
rejection. The focused 61-array put/evict request has input depth64 but requires
result depth65; its 60-array control succeeds at input63/result64. Final dependency
and checked-in STE identity checks bind the final harness/export to the executed
artifacts. Four exact affected Rust tests, real production support-loader positive,
missing-declaration negative and restored probes, samples and local links pass.
There are 33 suffix packages and 128 unchanged catalog/story/request/response items.

Required useful sizes remain intact: capacity128 with259 operations; capacity16
with528 operations; separate8192-scalar ASCII and Unicode keys/values;5000-digit
integer and exponent tokens; nested duplicate pairs;32 nested arrays; and maximum
i64 capacity. Exactly1MiB padded small input succeeds, while1MiB+1 fails. Actual
depth, cumulative allocation and instruction-fuel failures are retained alongside
successful controls; input size alone does not establish acceptance. Existing
value/arena/depth/fuel bounds apply. No O(1), generic Map, unlimited history,
reclamation, arbitrary-precision arithmetic or full Python compatibility is claimed.

Retained history includes the pre-source correction from unsupported custom error
categories to VALUE, source/probe syntax corrections, and harness assertion/name
corrections. The oracle amendment changed only reference/intent, preserving all78
case bytes and request outputs. Completed Task2 leaf subprocesses were bounded;
their historical top-level drivers lacked separate outer deadlines. This disclosed
process exception was accepted without repeating completed suites. Subsequent full
benchmark, acceptance and continuation drivers have finite outer deadlines and
descendant cleanup. Independent review also found that a rejected reused artifact
directory could lose its outer logs. A scoped supervisor repair and preservation
controls address that defect; the original review and evidence remain retained.
The broader Python-replacement and AI-agent goal stays active.
