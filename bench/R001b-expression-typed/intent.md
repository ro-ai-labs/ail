rung: R001b-expression-typed
tier: 0
action: TypedAddNumbers
targets: vm
blocked-on: type resolution over expression operands (Int vs Text); no mismatch diagnostic exists

# R001b — Resolve the expression's type

Second rung of the R001-add decomposition. Given `sum.a`, `sum.b`, and an
untouched `sum.label`, compute `sum.total = a + b` and pass `sum.label`
through unchanged. `sum.a` and `sum.b` are declared `Int`; `sum.label` is
declared `Text`.

Case-03 sets `sum.label=007` — a Text value that looks numeric. The checker
must resolve `sum.a + sum.b` as an `Int` expression and treat `sum.label` as
`Text` regardless of its shape. Canonicalizing `sum.label` — turning `007`
into `7` — is exactly the failure mode of a checker with no real notion of an
expression's type, only a parse-if-possible fallback that cannot tell a
Text field from an Int one. Resolving the type of the accepted case is the
prerequisite for rejecting a mismatch such as `Int + Text` later.

This rung's cases cover only the accepted, well-typed direction. Rejecting a
type-incorrect expression with a stable diagnostic is part of the same
capability, but is not independently scorable under the current
`ail_bench.py` contract: a rung only reaches `pass` by having `ail-check`
report zero diagnostics and then matching VM/native/oracle state, so a rung
whose defining behavior is "the checker refuses this program" can never
leave `check-failed`. See the ladder report for the proposed minimal runner
extension that would let a rejection be scored directly.
