package-name: dana-booking
profile: Application
features: things, actions, failures, guarantees, traces
max-turns: 12

# How the produced spec is scored against Dana's hidden intent

Pass/fail for the rung is fixed by the harness and is not negotiable here:

    invented == 0  and  converged  and  checked  and  elicited == 100%

The rules below are the finer-grained reading: did the spec that came out
actually describe *Dana's* application. They are reported as `rubric N/M` and
are a diagnostic, not a second gate. A rubric that could raise the pass rate
would be a rubric worth softening, so it cannot.

`checked` is `ail-check` reporting zero diagnostics on a package the harness
assembles from the metadata above plus the agent's spec. Note honestly what
that does and does not mean: on the current build the parser accepts a bullet it
does not understand and drops it silently, so `checked` is a floor, not a
ceiling. It says the spec parses. It says nothing about whether it is Dana's.

```require:application-declared
match: the application
match: manages
```

```require:booking-thing
why: there must be a thing representing the booked slot.
match: booking|bookings|appointment*|slot|slots
```

```require:book-action
match: action
match: book|books|booking|appointment*|creat*|reserv*
```

```require:cancel-action
why: cancellation with 24 hours notice was an explicit requirement.
match: cancel|cancels|cancelled|cancelling|cancellation*
```

```require:services-modelled
why: three services with different lengths. A spec with one generic booking lost the requirement.
match: service*|groom|grooming|wash|bath|nail*|treatment*
```

```require:trace
why: every action in an AIL-Spec records what happened.
match: records a trace event named
```

```require:slot-taken-failure
why: Dana described the double-booking case herself. It is the one failure she asked for.
match: failure
match: taken|unavailab*|not available|already booked|double|conflict*|full|busy|occupied
```
