#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    a, b = abs(int(s["n.a"])), abs(int(s["n.b"]))
    while b:
        a, b = b, a % b
    print(f"out.result={a}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
