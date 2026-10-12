#!/usr/bin/env python3
"""Independent dense Conway Life oracle; no AIL/runtime imports.

Direct grids are immutable tuples. JSON objects retain all member occurrences,
and number tokens remain opaque until selected strict-i64 validation.
"""
import json
import re
import sys

sys.dont_write_bytecode = True
MIN_I64 = -(1 << 63)
MAX_I64 = (1 << 63) - 1
NUMBER = re.compile(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?')
INTEGER = re.compile(r'-?(?:0|[1-9][0-9]*)\Z')


class VALUE(ValueError):
    """Invalid direct API domain argument."""


class RequestError(ValueError):
    pass


class Number(str):
    """Unconverted, syntactically valid JSON number lexeme."""


class Object(list):
    """Ordered duplicate-preserving JSON member pairs."""


def _i64(value):
    return type(value) is int and MIN_I64 <= value <= MAX_I64


def _bits(cells):
    if type(cells) not in (list, tuple) or any(type(c) is not int or c not in (0, 1) for c in cells):
        raise VALUE('VALUE')
    return tuple(cells)


def _grid(width, height, cells):
    if not _i64(width) or not _i64(height) or width <= 0 or height <= 0:
        raise VALUE('VALUE')
    bits = _bits(cells)
    # Python multiplication cannot overflow; division mirrors bounded validation.
    if width > MAX_I64 // height or width * height != len(bits):
        raise VALUE('VALUE')
    return bits


def _next(width, height, cells):
    result = []
    for y in range(height):
        for x in range(width):
            count = 0
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    nx, ny = x + dx, y + dy
                    if (dx or dy) and 0 <= nx < width and 0 <= ny < height:
                        count += cells[ny * width + nx]
            alive = cells[y * width + x]
            result.append(int(count == 3 or (alive == 1 and count == 2)))
    return tuple(result)


def step(width, height, cells):
    return _next(width, height, _grid(width, height, cells))


def evolve(width, height, cells, generations):
    current = _grid(width, height, cells)
    if not _i64(generations) or generations < 0:
        raise VALUE('VALUE')
    for _ in range(generations):
        current = _next(width, height, current)
    return current


def population(cells):
    return sum(_bits(cells))


def _scalar(text):
    if any(0xD800 <= ord(c) <= 0xDFFF for c in text):
        raise RequestError('invalid Unicode scalar')
    return text


def _parse(text):
    """Iterative complete JSON parser; no integer conversion or nesting cap."""
    if type(text) is not str:
        raise RequestError('not text')
    _scalar(text)
    n, pos = len(text), 0
    frames = []
    root = None
    have_root = False

    def whitespace(p):
        while p < n and text[p] in ' \t\r\n':
            p += 1
        return p

    def string(p):
        try:
            value, end = json.decoder.scanstring(text, p + 1, True)
        except ValueError as error:
            raise RequestError('invalid string') from error
        return _scalar(value), end

    def attach(value):
        nonlocal root, have_root
        if not frames:
            if have_root:
                raise RequestError('trailing root')
            root, have_root = value, True
        else:
            frame = frames[-1]
            if isinstance(frame[0], Object):
                frame[0].append((frame[2], value))
            else:
                frame[0].append(value)
            frame[1] = 'comma'

    while True:
        pos = whitespace(pos)
        if frames:
            container, state, key = frames[-1]
            is_object = isinstance(container, Object)
            closer = '}' if is_object else ']'
            if state in ('first', 'key') and is_object:
                if state == 'first' and pos < n and text[pos] == closer:
                    frames.pop()
                    pos += 1
                    continue
                if pos >= n or text[pos] != '"':
                    raise RequestError('expected member name')
                key, pos = string(pos)
                pos = whitespace(pos)
                if pos >= n or text[pos] != ':':
                    raise RequestError('expected colon')
                frames[-1][1:3] = ['value', key]
                pos += 1
                continue
            if state == 'comma':
                if pos < n and text[pos] == closer:
                    frames.pop()
                    pos += 1
                    continue
                if pos >= n or text[pos] != ',':
                    raise RequestError('expected comma')
                frames[-1][1] = 'key' if is_object else 'value'
                pos += 1
                continue
            if state == 'first' and not is_object and pos < n and text[pos] == closer:
                frames.pop()
                pos += 1
                continue
        elif have_root:
            if pos != n:
                raise RequestError('trailing input')
            return root
        if pos >= n:
            raise RequestError('missing value')
        token = text[pos]
        if token in '{[':
            value = Object() if token == '{' else []
            attach(value)
            frames.append([value, 'first', None])
            pos += 1
        elif token == '"':
            value, pos = string(pos)
            attach(value)
        elif token in '-0123456789':
            match = NUMBER.match(text, pos)
            if match is None:
                raise RequestError('invalid number')
            attach(Number(match.group()))
            pos = match.end()
        else:
            for spelling, value in (('true', True), ('false', False), ('null', None)):
                if text.startswith(spelling, pos):
                    attach(value)
                    pos += len(spelling)
                    break
            else:
                raise RequestError('invalid token')


def _selected_integer(token):
    if type(token) is not Number or INTEGER.fullmatch(token) is None:
        raise RequestError('expected integer')
    negative = token.startswith('-')
    magnitude = token[1:] if negative else token
    bound = '9223372036854775808' if negative else '9223372036854775807'
    if len(magnitude) > len(bound) or (len(magnitude) == len(bound) and magnitude > bound):
        raise RequestError('outside i64')
    return int(token)


def _failure(category):
    return {'ok': False, 'cells': [], 'population': 0, 'error': category}


def evaluate_request(text):
    try:
        document = _parse(text)
        if type(document) is not Object:
            raise RequestError('expected object')
        fields = {}
        for key, value in document:
            if key in ('width', 'height', 'cells', 'generations'):
                if key in fields:
                    raise RequestError('duplicate selected name')
                fields[key] = value
        if set(fields) != {'width', 'height', 'cells', 'generations'}:
            raise RequestError('missing selected name')
        width = _selected_integer(fields['width'])
        height = _selected_integer(fields['height'])
        generations = _selected_integer(fields['generations'])
        if type(fields['cells']) is not list:
            raise RequestError('expected array')
        cells = tuple(_selected_integer(c) for c in fields['cells'])
    except RequestError:
        return _failure('REQUEST')
    try:
        cells = _grid(width, height, cells)
    except VALUE:
        return _failure('GRID')
    if generations < 0:
        return _failure('GENERATIONS')
    result = evolve(width, height, cells, generations)
    return {'ok': True, 'cells': list(result), 'population': population(result), 'error': ''}


def encode(result):
    return json.dumps(result, ensure_ascii=False, separators=(',', ':'), allow_nan=False)


def main(arguments):
    if len(arguments) != 1 or not arguments[0].startswith('data.request='):
        raise SystemExit('expected exactly one data.request argument')
    encoded = encode(evaluate_request(arguments[0][len('data.request='):])).encode('utf-8')
    sys.stdout.write('out.result=hex:' + encoded.hex() + '\n')


if __name__ == '__main__':
    main(sys.argv[1:])
