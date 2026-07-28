#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    a, b = int(s["n.a"]), int(s["n.b"])
    q = abs(a) // abs(b) * (1 if (a < 0) == (b < 0) else -1)
    print(f"out.difference={a - b}")
    print(f"out.product={a * b}")
    print(f"out.quotient={q}")
    print(f"out.remainder={a - q * b}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
