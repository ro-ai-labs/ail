#!/usr/bin/env python3
"""Oracle for R000: increment counter.value by 1."""

import sys


def main(argv: list[str]) -> int:
    state = dict(arg.split("=", 1) for arg in argv if "=" in arg)
    value = int(state["counter.value"])
    print(f"counter.value={value + 1}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
