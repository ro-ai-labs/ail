# bench/conv — the conversation / negotiation benchmark

`bench/` measures whether AIL can compute. `bench/conv/` measures the other half
of the product: whether the agent a non-programmer talks to can **negotiate** a
specification — ask about what it does not know, refuse to invent what nobody
told it, and converge on an AIL-Spec that matches what the person actually
wanted.

Run it with `python3 scripts/ail_conv_bench.py`. The rationale for every design
decision, including why no LLM judges anything here, is in that script's header.

## A rung

```text
bench/conv/C001-booking/
  persona.md       hidden intent + how this person talks. NEVER shown to the agent.
  must-elicit.md   facts the agent must ask about, with accept/reject criteria
  forbidden.md     things it must not invent
  rubric.md        how the produced spec is read against the hidden intent
```

Plus, shared by every rung:

```text
bench/conv/grammar-sheet.md   the AIL-Spec grammar handed to the agent under test
bench/conv/CONV.lock          frozen rung fingerprints, best score, passing set
```

## Score

Per conversation:

| Field | Meaning |
|---|---|
| `elicited` | how many must-elicit facts the agent actually asked about |
| `invented` | how many forbidden items reached the final spec |
| `converged` | did an AIL-Spec appear at all, inside the turn cap |
| `checked` | does it pass `ail-check` with zero diagnostics |
| `turns` | agent turns to converge |
| `rubric` | diagnostic only: how well the spec matches the hidden intent |

A conversation passes only when `invented == 0`, `converged`, `checked`, and
`elicited == 100%`. `rubric` is reported but cannot move pass/fail — a rubric
that could raise the pass rate would be a rubric worth softening.

## Rule grammar

All three rule files share one line grammar, so there is one thing to learn and
one thing to audit:

```text
match: alt|alt|alt      an alternation group; repeatable
accept: ...             synonym of match; reads better in must-elicit.md
reject: alt|alt         disqualifies a candidate that matches it
why: free text          human note, ignored by the scorer
```

**Every match/accept line must match; any one alternative inside a line is
enough.** Groups are ANDed, alternatives are ORed. A trailing `*` is a prefix
match: `cancel*` covers cancelled and cancellation. Matching is case-insensitive
and word-boundary. There is no regex escape hatch, deliberately: a frozen
measuring stick that only regex authors can read is not auditable.

`accept` rules in must-elicit.md are matched against the agent's **questions**.
`match` rules in forbidden.md and rubric.md are matched against the **final
spec**.

## Why forbidden items are invented by construction

Every entry in a `forbidden.md` names something the persona does not want and
will refuse in plain words if asked — see the `refuse` block in each
`persona.md`. The simulated user therefore cannot truthfully confirm one. Any
appearance in the produced spec is an invention, with no intent detection and no
judgement call anywhere in the path.

Asking about a forbidden item is not penalised. Asking is the correct behaviour.
Writing it down uninvited is the failure.

## First measurement

`C001-booking`, `ornith-1.0-35b` at `192.168.60.171:8091`, 2026-07-28, 12 turns:

```text
   FAIL  C001-booking   elicited 5/6  invented 0  converged no  checked no
                        turns 12/12  rubric 0/7
         never asked: who-books
```

The agent asked good questions and never invented anything. It also never
stopped asking. By turn 11 it was asking whether booking times should be shown
in 12-hour or 24-hour format. It produced no spec at all, so `converged`,
`checked` and every rubric rule are false by consequence rather than on their
own merits.

`invented 0` on a run with no spec is not a result. It is the absence of one,
and the pass rule is written so that it cannot be mistaken for a good score.

Three things were measured on the way to that number and are recorded here
because they change how the harness has to be configured:

1. **With its reasoning trace enabled, this model does not deliver the
   artifact.** On the turn where it should have emitted the finished AIL-Spec it
   produced 6905 characters of reasoning and an empty `content`, at a 900-token
   budget and again at 1800. `ail_conv_bench.py` therefore runs both roles with
   `chat_template_kwargs: {"enable_thinking": false}` by default;
   `--agent-thinking` re-enables it. Anything in this repo that calls the
   endpoint with a fixed `max_tokens` and reads `content` is exposed to the same
   failure.
2. **A turn the model cannot produce is scored, not raised.** An agent that
   returns nothing has failed the conversation; aborting the benchmark would
   hide that behind a stack trace. Only a simulated-user failure voids a run,
   because then the fixture broke rather than the agent.
3. **The simulated user elaborates around the question it was asked.** Asked how
   long a session takes, Dana also gave her opening hours. This cannot inflate
   `elicited`, which counts only the agent's own questions, and it can only
   reduce `invented`. It makes the benchmark slightly easier and never
   dishonest, so it is left alone: real customers do this constantly.

Three elicitation rules were tightened between the run and the freeze, after
reading which question had been credited for which fact — `duration` was firing
on "pick a specific time slot", `hours` on "see real-time availability", and
`who-uses` on "who can take a shift". Each is noted in place in the rung file.
That is the only window in which a rule may be loosened or tightened: before the
rung is frozen. Afterwards the fingerprint in `CONV.lock` makes it a deliberate,
visible act.

## Adding a rung

1. Write the four files. The persona must be a person with a job, not a
   requirements document with a name.
2. Run it and confirm it fails, or at least confirm which parts fail and why. A
   rung written to match what already works measures nothing.
3. `python3 scripts/ail_conv_bench.py --update-lock` to freeze it.

Once a rung is in `CONV.lock` the harness aborts on any change to its four
files. Revising one is a deliberate human act (`--bless-rungs`), which is the
whole reason the lock exists.
