rung: C002-shift-swap
facts: 6

# What the agent must ask about

Rule grammar is documented in C001-booking/must-elicit.md and is identical here.
A fact counts only when one of the agent's own questions matches it.

`approval` is the rung. Marius's opening says "swap between themselves", which
reads as needing nobody's permission. The requirement he actually has is the
opposite. An agent that scores 5/6 here and misses `approval` has built a tool
that does the wrong thing while looking finished, which is worse than one that
did not converge.

```fact:who-uses
why: waiters, bar, kitchen, and two managers. The roles are the shape of the problem.
accept: who
accept: use|uses|using|user*|users|staff|employee*|people|team*|access|roles|involved
reject: take|takes|taking|cover|covers|swap|swaps|approv*
# The reject line keeps "who can take a shift" from being counted here. That is
# an eligibility question, and it has its own fact.
```

```fact:approval
why: THE hidden requirement. A swap is not final until Marius or Elena okays it.
accept: approv*|sign off|sign-off|authoris*|authoriz*|permission*|manager*|supervisor*|boss|vet|veto|okay it|ok it
accept: need*|require*|have to|has to|must|before|involv*|anyone else|somebody else|someone else|first
```

```fact:shift-details
why: date, start, end, section. Without section the training rule cannot be expressed.
accept: shift|shifts
accept: consist*|made up|details|define*|includ*|contain*|date|section*|station*|which part|record about|know about|store about|look like|made of
```

```fact:eligibility
why: only trained staff may cover a section. This is the safety rule under the convenience feature.
accept: take|takes|taking|cover|covers|covering|swap|swaps|swapping|pick up|picks up|accept*|claim*
accept: anyone|anybody|eligib*|qualifi*|train*|restrict*|rule|rules|allowed|who can|limit*|constraint*|same role|same section|any shift
```

```fact:unclaimed
why: an unclaimed shift 24 hours out is Marius's problem, and the tool has to say so.
accept: nobody|no one|no-one|unclaim*|not taken|no takers|nobody takes|left over|uncovered|expire*|no volunteer*|goes unfilled|unfilled
```

```fact:history
why: the March argument. He wants to be able to prove who was on.
accept: record|records|recorded|recording|log|logs|logged|histor*|audit*|track*|trail|look back|who did|see who|report*
accept: swap|swaps|swapped|shift|shifts|change*|cover*
```
