#!/usr/bin/env python3
"""Independent strict-CSV/i64 aggregation oracle; Python standard library only.

CSV syntax is a declarative full grammar, and csv.reader decodes the rows.
Neither expected values nor parsing behavior come from executing AIL.
"""
from __future__ import annotations

import csv
import io
import json
import re
import sys

# The host csv module's 128 KiB default must not become a dialect restriction.
# Runtime Text/Json/fuel/allocation bounds are separate from this domain oracle.
csv.field_size_limit(sys.maxsize)

MINIMUM = -(1 << 63)
MAXIMUM = (1 << 63) - 1
# field := quoted | bare; row := field (',' field)*
# quoted := '"' (nonquote | '""')* '"'; bare excludes all syntax characters.
# document := (row terminator)* row?; terminator := CRLF | CR | LF.
# Empty rows are legal in this grammar; csv.reader supplies their [] meaning.
# A lone CR cannot match before LF: each CRLF has exactly one interpretation,
# even if a later malformed field forces regex backtracking.
FIELD = r'(?:"(?:[^"]|"")*"|[^",\r\n]*)'
ROW = FIELD + r'(?:,' + FIELD + r')*'
DOCUMENT = re.compile(r'(?:' + ROW + r'(?:\r\n|\r(?!\n)|\n))*(?:' + ROW + r')?')
INTEGER = re.compile(r'-?(?:0|[1-9][0-9]*)')


class DomainError(Exception):
    pass


class Object(list):
    """Preserve JSON member pairs; distinguish objects from arrays."""


class Number(str):
    """Ignore unselected numbers without converting them or treating as Text."""


def reject_constant(_):
    raise ValueError('non-JSON constant')


def request_csv(text):
    document = json.loads(text, object_pairs_hook=Object, parse_int=Number,
                          parse_float=Number, parse_constant=reject_constant)
    # Validate every decoded JSON string, including unused keys and members.
    def scalar_strings(item):
        if isinstance(item, str):
            item.encode('utf-8')
        elif isinstance(item, Object):
            for key, child in item:
                key.encode('utf-8')
                scalar_strings(child)
        elif isinstance(item, list):
            for child in item:
                scalar_strings(child)
    scalar_strings(document)
    if not isinstance(document, Object):
        raise ValueError('request object required')
    selected = [value for key, value in document if key == 'csv']
    if len(selected) != 1 or type(selected[0]) is not str:
        raise ValueError('one csv string required')
    return selected[0]


def parse(text):
    if DOCUMENT.fullmatch(text) is None:
        raise DomainError('CSV')
    # Python <=3.10 rejects NUL despite its being ordinary field data here.
    # An absent non-syntax scalar is a reversible alphabet renaming, not a CSV
    # parser: commas, quotes and line endings are unchanged for csv.reader.
    sentinel = None
    reader_text = text
    if '\x00' in text:
        occupied = set(text)
        sentinel = next(chr(code) for code in range(0xE000, 0x110000)
                        if chr(code) not in occupied)
        reader_text = text.replace('\x00', sentinel)
    try:
        # newline='' preserves quoted CR, LF and CRLF exactly.
        rows = list(csv.reader(io.StringIO(reader_text, newline=''), delimiter=',',
                               quotechar='"', doublequote=True,
                               skipinitialspace=False, strict=True))
        return rows if sentinel is None else [[field.replace(sentinel, '\x00')
                                               for field in row] for row in rows]
    except csv.Error as error:
        raise DomainError('CSV') from error


def failure(code):
    return {'ok': False, 'rows': 0, 'total': 0, 'groups': [], 'error': code}


def amount(token):
    if INTEGER.fullmatch(token) is None:
        raise DomainError('AMOUNT')
    negative = token.startswith('-')
    digits = token[1:] if negative else token
    bound = str(-MINIMUM if negative else MAXIMUM)
    # Avoid Python's decimal-digit conversion limit for huge invalid amounts.
    if len(digits) > len(bound) or (len(digits) == len(bound) and digits > bound):
        raise DomainError('AMOUNT')
    return int(token)


def checked(value):
    if not MINIMUM <= value <= MAXIMUM:
        raise DomainError('OVERFLOW')
    return value


def summarize(text):
    try:
        rows = parse(text)
        if not rows:
            raise DomainError('HEADER')
        header, data = rows[0], rows[1:]
        if (not header or any(name == '' for name in header)
                or len(set(header)) != len(header)
                or 'category' not in header or 'amount' not in header):
            raise DomainError('HEADER')
        if any(len(row) != len(header) for row in data):
            raise DomainError('ROW')
        category_index, amount_index = header.index('category'), header.index('amount')
        groups = {}
        total = 0
        for row in data:
            value = amount(row[amount_index])
            category = row[category_index]
            count, subtotal = groups.get(category, (0, 0))
            subtotal = checked(subtotal + value)
            total = checked(total + value)
            groups[category] = count + 1, subtotal
        return {'ok': True, 'rows': len(data), 'total': total,
                'groups': [{'category': category, 'rows': groups[category][0],
                            'total': groups[category][1]} for category in sorted(groups)],
                'error': ''}
    except DomainError as error:
        return failure(str(error))


def evaluate_request(text):
    try:
        return summarize(request_csv(text))
    except (ValueError, UnicodeError):
        return failure('REQUEST')


def encode(value):
    """AIL documented Json encoding (41-json-conversion.md), not json.dumps.

    C0 and DEL/C1 controls use lowercase unicode escapes, except LF/CR/TAB.
    Backspace/form feed therefore use \\u0008/\\u000c, never \\b/\\f.
    """
    if type(value) is str:
        escapes = {'"': '\\"', '\\': '\\\\', '\n': '\\n', '\r': '\\r', '\t': '\\t'}
        return '"' + ''.join(escapes.get(char, f'\\u{ord(char):04x}'
                                       if ord(char) < 32 or 127 <= ord(char) <= 159
                                       else char) for char in value) + '"'
    if value is True:
        return 'true'
    if value is False:
        return 'false'
    if value is None:
        return 'null'
    if type(value) is int:
        return str(value)
    if type(value) is list:
        return '[' + ','.join(map(encode, value)) + ']'
    if type(value) is dict:
        return '{' + ','.join(encode(key) + ':' + encode(child)
                              for key, child in value.items()) + '}'
    raise TypeError('unsupported oracle output')


def self_test():
    accepted = [('', []), ('\n', [[]]), ('\r', [[]]), ('\r\n', [[]]),
                (',', [['', '']]), ('""', [['']]), ('a\n', [['a']]),
                ('\n\n', [[], []]), ('a,\r\n', [['a', '']]),
                ('"a\r\nb\rc\nd",x', [['a\r\nb\rc\nd', 'x']]),
                ('"a""b",c', [['a"b', 'c']]), ('""""', [['"']]),
                ('\r\n\r\n', [[], []]), ('\r\r\n', [[], []]),
                ('\x00,é🙂', [['\x00', 'é🙂']]),
                ('"\x00\ue000\ue001",\x00', [['\x00\ue000\ue001', '\x00']])]
    for source, expected in accepted:
        assert parse(source) == expected, repr(source)
    rejected = ['"', '"""', 'a"', 'a"b', ' "a"', '"a" ', '"a"x',
                '"a"\t', 'a,"b', 'a\n"b', '"a""', '"a";b', 'a\\"b',
                # Many earlier CRLF records must not make a malformed tail
                # explore exponentially many CR + empty-LF interpretations.
                'category,amount\r\n' + 'a,1\r\n' * 384 + '"']
    for source in rejected:
        try:
            parse(source)
        except DomainError as error:
            assert str(error) == 'CSV'
        else:
            raise AssertionError(repr(source))
    hand = [
        ('category,amount\na,4\na,5', '{"ok":true,"rows":2,"total":9,"groups":[{"category":"a","rows":2,"total":9}],"error":""}'),
        ('amount,category,note\n-2,z,ignored\n7,a,x\n0,z,y', '{"ok":true,"rows":3,"total":5,"groups":[{"category":"a","rows":1,"total":7},{"category":"z","rows":2,"total":-2}],"error":""}'),
        ('category,amount', '{"ok":true,"rows":0,"total":0,"groups":[],"error":""}'),
        ('category,amount\n,3\nx,-0\n,-1', '{"ok":true,"rows":3,"total":2,"groups":[{"category":"","rows":2,"total":2},{"category":"x","rows":1,"total":0}],"error":""}'),
        ('category,amount\nx,-9223372036854775808\ny,9223372036854775807', '{"ok":true,"rows":2,"total":-1,"groups":[{"category":"x","rows":1,"total":-9223372036854775808},{"category":"y","rows":1,"total":9223372036854775807}],"error":""}'),
        ('category,amount\na,+1\nb,1,extra', '{"ok":false,"rows":0,"total":0,"groups":[],"error":"ROW"}'),
        ('category,amount\na,+1\nb,"bad', '{"ok":false,"rows":0,"total":0,"groups":[],"error":"CSV"}'),
        ('category,amount\na,9223372036854775807\na,1\na,-1', '{"ok":false,"rows":0,"total":0,"groups":[],"error":"OVERFLOW"}'),
        ('category,amount\na,01', '{"ok":false,"rows":0,"total":0,"groups":[],"error":"AMOUNT"}'),
        ('', '{"ok":false,"rows":0,"total":0,"groups":[],"error":"HEADER"}'),
    ]
    for source, expected in hand:
        assert encode(summarize(source)) == expected, repr(source)
    assert encode('\x00\b\f\n\r\t\x7f\x85\x9fé🙂') == '"\\u0000\\u0008\\u000c\\n\\r\\t\\u007f\\u0085\\u009fé🙂"'
    good = '{"csv":"category,amount","x":1,"x":2}'
    assert evaluate_request(good) == summarize('category,amount')
    for source in ['{"csv":"","c\\u0073v":""}', '{"csv":0}', '{}', '[]',
                   '{"csv":"category,amount","x":"\\ud800"}', '{"csv":"", "x":NaN}']:
        assert evaluate_request(source) == failure('REQUEST'), source
    return {'accepted_parse_vectors': len(accepted), 'rejected_parse_vectors': len(rejected),
            'hand_aggregate_outputs': len(hand), 'encoder_vectors': 1,
            'request_vectors': 7}


def main(arguments):
    if arguments == ['--self-test']:
        print(json.dumps(self_test(), sort_keys=True))
        return 0
    state = dict(argument.split('=', 1) for argument in arguments if '=' in argument)
    print('out.result=' + encode(evaluate_request(state.get('data.request', ''))))
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
