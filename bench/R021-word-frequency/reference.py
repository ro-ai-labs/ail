#!/usr/bin/env python3
"""Independent word-frequency oracle: regex, Counter and scalar lexical sort.

The JSON reader retains all object pairs and opaque numeric lexemes. No AIL
implementation, output, host parser primitive, or float conversion supplies an
expectation. Ordinary runtime resource failures are outside this domain oracle.
"""
from collections import Counter
import json
import re
import sys

# Exactly 29 pinned separators, rather than Python's evolving \\s category.
WORD = re.compile(r'[^\u0009-\u000d\u001c-\u0020\u0085\u00a0\u1680'
                  r'\u2000-\u200a\u2028-\u2029\u202f\u205f\u3000]+')


class Members(tuple):
    """Object member pairs, kept distinct from JSON arrays."""


class NumberToken:
    """Opaque syntax-valid JSON number, deliberately neither Text nor Float."""
    def __init__(self, lexeme):
        self.lexeme = lexeme


class RequestError(Exception):
    pass


def non_json_constant(_token):
    raise RequestError('non-JSON constant')


def validate_unicode(value):
    """Reject surrogate scalars throughout the document, including ignored keys."""
    if type(value) is str:
        value.encode('utf-8', errors='strict')
    elif isinstance(value, Members):
        for key, child in value:
            key.encode('utf-8', errors='strict')
            validate_unicode(child)
    elif type(value) is list:
        for child in value:
            validate_unicode(child)


def request_text(request):
    document = json.loads(request, object_pairs_hook=Members,
                          parse_int=NumberToken, parse_float=NumberToken,
                          parse_constant=non_json_constant)
    validate_unicode(document)
    if not isinstance(document, Members):
        raise RequestError('object required')
    selected = [value for key, value in document if key == 'text']
    if len(selected) != 1 or type(selected[0]) is not str:
        raise RequestError('one actual string member named text required')
    return selected[0]


def tokens(text):
    return WORD.findall(text)


def summarize(text):
    words = tokens(text)
    counts = Counter(words)
    ranked = sorted(counts.items(), key=lambda item: (-item[1], item[0]))
    return {'ok': True, 'words': len(words), 'unique': len(counts),
            'top': [{'word': word, 'count': count} for word, count in ranked[:5]],
            'error': ''}


def failure():
    return {'ok': False, 'words': 0, 'unique': 0, 'top': [], 'error': 'REQUEST'}


def evaluate_request(request):
    try:
        text = request_text(request)
    except (json.JSONDecodeError, RequestError, UnicodeError):
        return failure()
    return summarize(text)


def encode(value):
    """Compact AIL Json transport, including C0/DEL/C1 lowercase escapes."""
    if type(value) is str:
        pieces = ['"']
        short = {'"': '\\"', '\\': '\\\\', '\t': '\\t', '\r': '\\r', '\n': '\\n'}
        for ch in value:
            code = ord(ch)
            if ch in short:
                pieces.append(short[ch])
            elif code < 32 or 127 <= code <= 159:
                pieces.append('\\u%04x' % code)
            else:
                pieces.append(ch)
        pieces.append('"')
        return ''.join(pieces)
    if type(value) is bool:
        return 'true' if value else 'false'
    if type(value) is int:
        return str(value)
    if value is None:
        return 'null'
    if type(value) is list:
        return '[' + ','.join(map(encode, value)) + ']'
    if type(value) is dict:
        return '{' + ','.join(encode(key) + ':' + encode(child)
                              for key, child in value.items()) + '}'
    raise TypeError('unsupported oracle output value')


def main(argv):
    selected = [arg.split('=', 1)[1] for arg in argv if arg.startswith('data.request=')]
    result = evaluate_request(selected[0]) if len(selected) == 1 else failure()
    sys.stdout.write('out.result=' + encode(result) + '\n')


if __name__ == '__main__':
    main(sys.argv[1:])
