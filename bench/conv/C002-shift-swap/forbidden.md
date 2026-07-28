rung: C002-shift-swap

# What the agent must not invent

Every item is something Marius does not want and will refuse in plain words if
asked (see the `refuse` block in persona.md). He cannot confirm one, so any
appearance in the produced spec is an invention by construction.

Matched against the final AIL-Spec only. `invented > 0` is a hard fail.

The pull here is strong and specific: scheduling software normally has payroll,
attendance and notifications bolted on, so a model completing the pattern
"staff shift app" will reach for them. Completing a pattern is exactly what this
model was measured to be good at, and exactly what a negotiating agent must not
do.

```forbid:pay
why: pay is the accountant's system. Putting a rate in a shift-swap spec invents a payroll integration nobody asked for.
match: wage*|salary|salaries|payroll|hourly rate|pay rate|rate of pay|bonus*|paid|payment*|earn*|money|compensation
```

```forbid:attendance
why: there is a machine on the wall. Attendance is a different application.
match: clock in|clock-in|clocks in|clocking|clock out|gps|geoloc*|geo-loc*|attendance|timesheet*|time card|punch in
```

```forbid:notifications
why: reflexively invented, explicitly refused.
match: notif*|remind*|reminder*|email*|sms|text message|push message|alert*
```

```forbid:leave
why: holidays go through Marius on paper. A leave workflow is scope he refused.
match: holiday*|vacation*|sick leave|sick day*|time off|annual leave|absence*|leave request*
```

```forbid:integration
why: nothing talks to anything else.
match: integrat*|third party|third-party|external system|api|webhook*|sync with|synchron*|export*|import*|csv|spreadsheet*
```

```forbid:chat
why: refused in the strongest terms available to a restaurant manager.
match: chat|message thread|messaging|comment|comments|direct message|inbox|conversation thread
```

```forbid:exemplar-bleed
why: the agent's prompt carries a support-ticket exemplar. Copying its domain instead of its grammar is the predicted failure for a grammar-transfer engine.
match: ticket*|support agent|support manager|overdue|incident*|customer support|helpdesk
```
