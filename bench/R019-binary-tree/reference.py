#!/usr/bin/env python3
"""Independent iterative BST oracle, frozen before the AIL implementation."""
import json
import sys


class Node:
    def __init__(self, value):
        self.value = value
        self.count = 1
        self.left = None
        self.right = None


def analyze(values, query):
    root = None
    for value in values:
        if root is None:
            root = Node(value)
            continue
        cursor = root
        while True:
            if value == cursor.value:
                cursor.count += 1
                break
            side = 'left' if value < cursor.value else 'right'
            child = getattr(cursor, side)
            if child is None:
                setattr(cursor, side, Node(value))
                break
            cursor = child

    ordered = []
    stack = []
    cursor = root
    while cursor is not None or stack:
        while cursor is not None:
            stack.append(cursor)
            cursor = cursor.left
        cursor = stack.pop()
        ordered.extend([cursor.value] * cursor.count)
        cursor = cursor.right

    present = False
    cursor = root
    while cursor is not None:
        if query == cursor.value:
            present = True
            break
        cursor = cursor.left if query < cursor.value else cursor.right

    height = 0
    pending = [(root, 1)] if root is not None else []
    while pending:
        cursor, depth = pending.pop()
        height = max(height, depth)
        if cursor.left is not None:
            pending.append((cursor.left, depth + 1))
        if cursor.right is not None:
            pending.append((cursor.right, depth + 1))
    return {'out.sorted': json.dumps(ordered, separators=(',', ':')),
            'out.present': str(present).lower(), 'out.height': str(height)}


def main(arguments):
    state = dict(arg.split('=', 1) for arg in arguments if '=' in arg)
    values = json.loads(state['data.values'])
    query = int(state['data.query'])
    if type(values) is not list or any(type(v) is not int or not -(2**63) <= v < 2**63 for v in values):
        raise ValueError('R019 expects List<Int>')
    if not -(2**63) <= query < 2**63:
        raise ValueError('R019 expects Int query')
    for key, value in sorted(analyze(values, query).items()):
        print(key + '=' + value)
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
