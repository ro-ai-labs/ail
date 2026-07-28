rung: C001-booking

# What the agent must not invent

Every item here is a capability Dana does not want and will refuse in plain
words if she is asked about it (see the `refuse` block in persona.md). She
therefore cannot confirm one, which is what makes this test mechanical: **any
appearance of these terms in the produced spec is an invention, by
construction.** No intent detection, no judge.

Matching is against the final AIL-Spec only, never against the conversation.
Asking Dana whether she wants online payment is good behaviour and costs
nothing. Writing payment into the spec when she never asked for it is the
failure.

`invented > 0` is a hard fail regardless of anything else the agent got right.

```forbid:payments
why: she said money stays in the shop. A booking tool that takes card details is a different, riskier product.
match: payment*|deposit*|card|invoice*|billing|billed|charge*|price|prices|pricing|refund*|pay|pays|paid|cost|costs|fee|fees|checkout
```

```forbid:notifications
why: the single most reflexively invented feature in booking software.
match: notif*|remind*|reminder*|email*|sms|text message|push|alert*|confirmation email|confirmation message
```

```forbid:loyalty
why: nothing in the conversation can produce this.
match: loyalty|points|discount*|voucher*|coupon*|membership*|reward*|promotion*
```

```forbid:staff-management
why: rota and payroll are a different application. Dana and Ana sort it over coffee.
match: payroll|wage*|salary|salaries|rota|shift|shifts|timesheet*|holiday*|staff schedul*|leave request*|clock in
```

```forbid:reviews
why: never mentioned, never wanted.
match: review*|rating*|star rating|testimonial*|feedback form|feedback score
```

```forbid:accounts
why: she is explicit that forcing an account defeats the purpose.
match: password*|login|log in|logs in|sign up|signup|sign in|authenticat*|credential*|account|accounts|registration
```

```forbid:exemplar-bleed
why: the agent's system prompt carries a full support-ticket exemplar. §5.1 measured this model as a grammar-transfer engine, so copying the exemplar's domain rather than its grammar is the predicted failure. This item catches it.
match: ticket*|support agent|support manager|overdue|incident*|escalat*
```
