#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    n = int(s["n.value"])
    prime = n > 1 and all(n % d for d in range(2, int(n ** 0.5) + 1))
    print(f"out.prime={1 if prime else 0}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
