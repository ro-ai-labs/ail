#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    n, steps = int(s["n.value"]), 0
    while n != 1:
        n = n // 2 if n % 2 == 0 else 3 * n + 1
        steps += 1
    print(f"out.steps={steps}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
