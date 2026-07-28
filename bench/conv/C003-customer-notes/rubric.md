package-name: ioana-customer-notes
profile: Application
features: things, actions, failures, guarantees, traces, secrets
max-turns: 12

# How the produced spec is scored against Ioana's hidden intent

Pass/fail is fixed by the harness:

    invented == 0  and  converged  and  checked  and  elicited == 100%

The rules below are the finer reading, reported as `rubric N/M`. They cannot
raise the pass rate.

This rung is the one where a checker-clean spec can be actively dangerous. A
spec that says every user may read every note will pass `ail-check` with zero
diagnostics, because the checker validates an artifact in isolation and has no
idea what Ioana said or what is in a note. `secret-marked` and
`failure-permission` are therefore the two rules that carry the real content of
this rung: they are the machine-checkable trace of the agent having noticed the
problem and having done something about it, rather than having written down the
first sentence Ioana said.

```require:application-declared
match: the application
match: manages
```

```require:note-thing
match: note
```

```require:secret-marked
why: the verification digits and the personal remarks must be Secret<...>. This is the hidden security requirement, and it is mechanically visible in the spec text.
match: secret<
```

```require:restricted-read
why: reading a note is not universal. The spec has to say so, in the grammar's own words.
match: does not reveal|readable only|reads|requires
match: permission*|denied|not allowed|role*|support|only
```

```require:roles-differentiated
why: support and field engineers are not sales and billing. A single "user" role loses the whole point.
match: support|engineer*|agent*
match: sales|billing|finance
```

```require:append-only
why: notes are never edited after the March argument.
match: guarantees|requires|does not
match: edit*|chang*|modif*|amend*|immutab*|append*|overwrit*
```

```require:audit-trace
match: records a trace event named
```

```require:failure-permission
why: someone will try to read a note they may not read, and the spec must say what happens.
match: failure
match: permission*|denied|not allowed|unauthoris*|unauthoriz*|refused|forbidden
```
