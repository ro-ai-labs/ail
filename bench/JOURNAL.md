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
