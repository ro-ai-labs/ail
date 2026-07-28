#!/usr/bin/env python3
"""Independent oracle for R001a: sum.total = sum.a + sum.b.

Isolates the checker/IR capability: representing `a + b` as a typed
expression node (operator + two operand references) instead of degrading a
non-literal arithmetic write to opaque text, or rejecting it outright via
AIL-EXPR-001. The arithmetic is identical to R001-add; what differs across
the R001[a-e] rungs is which pipeline stage the ladder asserts is real (see
each rung's intent.md). This oracle is a plain Python computation, never
derived from AIL's own output.
"""

import sys


def main(argv: list[str]) -> int:
    state = dict(arg.split("=", 1) for arg in argv if "=" in arg)
    total = int(state["sum.a"]) + int(state["sum.b"])
    print(f"sum.total={total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
