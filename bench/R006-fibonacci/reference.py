#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    n = int(s["n.value"])
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    print(f"out.result={a}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
