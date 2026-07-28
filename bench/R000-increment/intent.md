rung: R000-increment
tier: 0
action: IncrementCounter
targets: vm,native
blocked-on: nothing

# R000 — Increment a counter

Calibration rung. This is the one computation AIL provably performs today:
adding a compile-time integer literal to a runtime field.

It exists so the ladder score is falsifiable in both directions. If R000 passes
while R001 and above fail, the harness is working and the score is a real
statement about the language. If R000 also fails, the harness itself is broken
and no other result on the ladder can be trusted.

Given `counter.value`, the program must increment it by 1.
