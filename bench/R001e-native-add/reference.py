#!/usr/bin/env python3
"""Independent oracle for R001e: sum.total = sum.a + sum.b, native only.

Isolates native x86-64 codegen: given a working VM evaluator (R001d), can the
native backend emit real machine code -- register loads and an `add` -- for
the same computation? Same arithmetic as R001-add, R001a, and R001d; this
rung's `targets: native` metadata means only the compiled binary is
exercised, not the VM. Plain Python computation, never derived from AIL's
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
