#!/usr/bin/env python3
"""Independent oracle for R001d: sum.total = sum.a + sum.b, VM only.

Isolates the VM evaluator: given the value model from R001c, can the
interpreter read two runtime fields and combine them with `+`? Same
arithmetic as R001-add and R001a; what differs is that this rung's
`targets: vm` metadata means only the bytecode VM is exercised, deferring
native codegen to R001e. Plain Python computation, never derived from AIL's
own output.
"""

import sys


def main(argv: list[str]) -> int:
    state = dict(arg.split("=", 1) for arg in argv if "=" in arg)
    total = int(state["sum.a"]) + int(state["sum.b"])
    print(f"sum.total={total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
