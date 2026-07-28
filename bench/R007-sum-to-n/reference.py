#!/usr/bin/env python3
"""Independent oracle. Reads key=value args, prints expected final state."""

import sys


def main(argv):
    s = dict(a.split("=", 1) for a in argv if "=" in a)
    n = int(s["n.value"])
    print(f"out.total={n * (n + 1) // 2}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
