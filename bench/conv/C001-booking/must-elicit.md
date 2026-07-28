rung: C001-booking
facts: 6

# What the agent must ask about

A fact counts as elicited only when one of the agent's own **questions** matches
the rule. Answers do not count, prose does not count, and a spec that happens to
contain the right thing does not count -- the measurement is whether the agent
asked, because that is the behaviour the product depends on.

Rule grammar (identical in every rung file):

- `accept:` one alternation group; every accept line must match the same
  question, any one alternative inside a line is enough.
- `reject:` if any of these appears in the question, that question does not
  count for this fact.
- a trailing `*` is a prefix match, so `cancel*` covers cancelled and
  cancellation.

Each fact below is deliberately given two groups where a single keyword would
also fire on a neighbouring question. Over-crediting a fact makes the benchmark
say the agent negotiated when it did not, which is the failure this whole
exercise exists to avoid.

```fact:services
why: Dana books three different things and they are not interchangeable.
accept: service|services|groom|grooming|grooms|treatment*|wash|bath|nail*|haircut*|appointment type*|type of appointment|kind of appointment|types of appointment
accept: offer*|provide*|do you do|are there|available|list|which|what kind*|what kinds|what type*|what types|different|each
```

```fact:duration
why: 45 minutes and 90 minutes are different slots. Without this the tool books nonsense.
accept: long|duration|minutes|hour|hours|slot length|how much time|amount of time
accept: appointment*|booking*|bookings|service*|groom|grooming|each|session*|slot|slots|visit*
reject: opening hours|business hours|working hours
# 'time slot' was removed from the first group after a live run: "pick a specific
# time slot" satisfied it while asking nothing about how long anything takes.
# Rules are tightened before a rung is frozen, never after.
```

```fact:hours
why: Tuesday to Saturday, 9 to 6, and nothing may run past 6.
accept: open|opens|opening|opened|closed|closing|closes|opening hours|operating hours|business hours|working hours|what hours|which hours|hours are you|hours do you|days are you
accept: you|your|shop|salon|week|day|days|when
reject: how long does|how long is|how much time
# Bare 'hours', 'days' and 'availab*' were removed from the first group after a
# live run: "see real-time availability and pick a specific time slot" and "which
# days are fully booked" both satisfied them while asking nothing about when the
# shop is open. The group now needs an opening/closing word.
```

```fact:customer-details
why: name, phone, dog's name, breed. An agent that guesses this guesses wrong.
accept: name|phone|number|contact|details|information|info
accept: customer*|client*|owner*|person|people|caller*|user*|need from|collect|take from|ask for
```

```fact:cancellation
why: 24 hours notice is a real rule and the agent will not invent it correctly.
accept: cancel|cancels|cancelled|cancelling|cancellation*|reschedul*|change the booking|change a booking|move the booking|move a booking|call off|no show|no-show
```

```fact:who-books
why: both the customer and Dana create bookings. Missing one halves the tool.
accept: who
accept: book|books|booking|bookings|make the appointment|makes an appointment|create*|enter*|add*|take the booking
```
