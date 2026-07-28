#!/usr/bin/env python3
"""Independent oracle for R001b: sum.total = sum.a + sum.b, sum.label unchanged.

Isolates type resolution: sum.a/sum.b are Int and must be added and
canonicalized; sum.label is Text and must survive byte-for-byte even when it
looks numeric (case-03 uses the Text label "007"). A checker/evaluator that
does not really resolve operand types -- one that treats every field as an
int-if-parseable soup instead -- would canonicalize "007" to "7" and diverge
from this oracle. Plain Python computation, never derived from AIL's output.
"""

import sys


def main(argv: list[str]) -> int:
    state = dict(arg.split("=", 1) for arg in argv if "=" in arg)
    total = int(state["sum.a"]) + int(state["sum.b"])
    label = state["sum.label"]
    print(f"sum.total={total}")
    print(f"sum.label={label}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
