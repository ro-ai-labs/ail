#!/usr/bin/env python3
"""Independent oracle for R001c: sum.mirror_a = sum.a, sum.mirror_b = sum.b.

No operator -- this isolates the VM's value model from the evaluator.
Passing requires the VM to read two runtime fields as typed Int values and
write them to two other fields, canonicalized (case-03 uses a leading zero
and a signed zero: "007" -> "7", "-0" -> "0"). A VM that copies raw field
text instead of a typed value would emit "007" and "-0" unchanged, diverging
from this oracle. Plain Python computation, never derived from AIL's output.
"""

import sys


def main(argv: list[str]) -> int:
    state = dict(arg.split("=", 1) for arg in argv if "=" in arg)
    mirror_a = int(state["sum.a"])
    mirror_b = int(state["sum.b"])
    print(f"sum.mirror_a={mirror_a}")
    print(f"sum.mirror_b={mirror_b}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
