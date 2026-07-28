package-name: marius-shift-swap
profile: Application
features: things, actions, failures, guarantees, traces
max-turns: 12

# How the produced spec is scored against Marius's hidden intent

Pass/fail is fixed by the harness:

    invented == 0  and  converged  and  checked  and  elicited == 100%

The rules below are the finer reading, reported as `rubric N/M`. They cannot
raise the pass rate.

The one that matters is `approval-required`. Marius said "swap between
themselves" and meant "swap between themselves, and then I say yes". A spec
without an approval precondition is a fluent, checker-clean description of the
wrong system, and it is worth noticing that nothing in `ail-check` can tell you
that. That gap is the point of this benchmark.

```require:application-declared
match: the application
match: manages
```

```require:roles
why: managers exist and are distinct from staff.
match: manager*|supervisor*
match: staff|employee*|waiter*|server*|cook*|chef*|worker*|bar|kitchen
```

```require:shift-thing
match: shift
```

```require:swap-action
match: action
match: swap*|cover*|trade*|offer*|take*|claim*
```

```require:approval-required
why: the hidden requirement, expressed as a precondition rather than a comment.
match: approv*
match: requires|manager*|guarantees|action
```

```require:eligibility-guard
why: a waiter with no bar training must not be able to take a bar shift.
match: train*|qualifi*|eligib*|section*|role*|allowed
match: requires|guarantees|only
```

```require:unclaimed-failure
why: he described this case himself.
match: failure
match: nobody|no one|unclaim*|not taken|expire*|no takers|unfilled|uncovered
```

```require:trace
match: records a trace event named
```
