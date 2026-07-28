#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    n, r = int(s["n.value"]), 1
    for i in range(2, n + 1):
        r *= i
    print(f"out.result={r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
