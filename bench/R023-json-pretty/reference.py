#!/usr/bin/env python3
"""Independent ordered-tree JSON renderer; stdlib parser, own string/layout encoder.

Numbers are opaque lexical tokens and objects are ordered pairs, never dictionaries.
No AIL implementation or formatter output supplies expectations for this module.
"""

import sys

sys.dont_write_bytecode = True

import json


class Number:
    """A JSON number lexeme, with no numeric coercion."""

    __slots__ = ("lexeme",)

    def __init__(self, lexeme: str):
        self.lexeme = lexeme


class Object:
    """Ordered JSON object member pairs, distinct from JSON arrays."""

    __slots__ = ("pairs",)

    def __init__(self, pairs):
        self.pairs = list(pairs)


def _reject_constant(token):
    raise ValueError("non-JSON number constant")


def _validate_unicode(value):
    if isinstance(value, str):
        if any(0xD800 <= ord(char) <= 0xDFFF for char in value):
            raise ValueError("invalid Unicode scalar")
    elif isinstance(value, Object):
        for key, member in value.pairs:
            _validate_unicode(key)
            _validate_unicode(member)
    elif isinstance(value, list):
        for member in value:
            _validate_unicode(member)


def parse(text: str):
    """Parse one full document into Object/list/Number/str/bool/None.

    Invalid JSON syntax or Unicode raises ValueError. Runtime/resource errors
    propagate; the oracle does not synthesize resource-limit domain errors.
    """
    value = json.loads(text, object_pairs_hook=Object, parse_int=Number,
                       parse_float=Number, parse_constant=_reject_constant)
    _validate_unicode(value)
    return value


def encode_string(value: str) -> str:
    """Canonical JSON string encoding: C0/DEL/C1, with only tab/CR/LF short."""
    _validate_unicode(value)
    encoded = ['"']
    short = {'"': '\\"', '\\': '\\\\', '\t': '\\t', '\r': '\\r', '\n': '\\n'}
    for char in value:
        point = ord(char)
        if char in short:
            encoded.append(short[char])
        elif point < 0x20 or 0x7F <= point <= 0x9F:
            encoded.append('\\u%04x' % point)
        else:
            encoded.append(char)
    encoded.append('"')
    return ''.join(encoded)


def encode(value) -> str:
    """Compact canonical JSON encoding of the oracle's typed tree."""
    if value is None:
        return 'null'
    if value is True:
        return 'true'
    if value is False:
        return 'false'
    if isinstance(value, Number):
        return value.lexeme
    if isinstance(value, str):
        return encode_string(value)
    if isinstance(value, Object):
        return '{' + ','.join(encode_string(key) + ':' + encode(member)
                              for key, member in value.pairs) + '}'
    if isinstance(value, list):
        return '[' + ','.join(encode(member) for member in value) + ']'
    raise TypeError('not an oracle JSON tree')


def render(value, depth: int = 0) -> str:
    """Recursively render a valid parsed tree, with no trailing LF."""
    indent = '  ' * depth
    child_indent = indent + '  '
    if isinstance(value, Object):
        if not value.pairs:
            return '{}'
        members = [child_indent + encode_string(key) + ': ' + render(member, depth + 1)
                   for key, member in value.pairs]
        return '{\n' + ',\n'.join(members) + '\n' + indent + '}'
    if isinstance(value, list):
        if not value:
            return '[]'
        members = [child_indent + render(member, depth + 1) for member in value]
        return '[\n' + ',\n'.join(members) + '\n' + indent + ']'
    return encode(value)


def render_text(text: str) -> str:
    """Parse/validate then render; syntax/Unicode errors raise ValueError."""
    return render(parse(text))


def evaluate_request(text: str) -> Object:
    """Return ordered ok/text/error result; REQUEST takes priority over JSON."""
    def result(ok, pretty, error):
        return Object([('ok', ok), ('text', pretty), ('error', error)])

    try:
        request = parse(text)
    except ValueError:
        return result(False, '', 'REQUEST')
    if not isinstance(request, Object):
        return result(False, '', 'REQUEST')
    selected = [member for key, member in request.pairs if key == 'json']
    if len(selected) != 1 or not isinstance(selected[0], str):
        return result(False, '', 'REQUEST')
    try:
        pretty = render_text(selected[0])
    except ValueError:
        return result(False, '', 'JSON')
    return result(True, pretty, '')


def main(argv):
    if len(argv) != 1 or not argv[0].startswith('data.request='):
        raise SystemExit('expected exactly one explicit data.request=<JSON> argument')
    request = argv[0].split('=', 1)[1]
    encoded = encode(evaluate_request(request)).encode('utf-8')
    sys.stdout.write('out.result=hex:' + encoded.hex() + '\n')


if __name__ == '__main__':
    main(sys.argv[1:])
