#!/usr/bin/env python3
"""Independent lossless Json / OrderedDict LRU model; no AIL dependencies.

Model Json values are Object(pairs), Number(lexeme), list, str, bool or None.
Typed capacities are Python signed-i64 int (bool excluded). Invalid typed helper
arguments are caller misuse, not domain failures. Valid Json shape failures use
DomainError.category VALUE; request error strings remain distinct. Only the
operation-shape validation catch maps VALUE to OPERATIONS; execution errors propagate.
Every public result is a deep copy, including nested payloads and events.
"""
from __future__ import annotations
import copy
import json
import re
import sys
from collections import OrderedDict
from dataclasses import dataclass

@dataclass(frozen=True)
class Object:
    pairs: tuple
    def __init__(self, pairs):
        object.__setattr__(self, 'pairs', tuple((k, v) for k, v in pairs))

@dataclass(frozen=True)
class Number:
    lexeme: str

class DomainError(Exception):
    def __init__(self, category):
        self.category = category
        super().__init__(category)

NUMBER = re.compile(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?\Z')
INTEGER = re.compile(r'-?(?:0|[1-9][0-9]*)\Z')
I64_MIN, I64_MAX = -(1 << 63), (1 << 63) - 1

def text_valid(text):
    if not isinstance(text, str):
        raise TypeError('Text requires str')
    if any(0xd800 <= ord(c) <= 0xdfff for c in text):
        raise ValueError('invalid Unicode scalar')


def validate_json(value):
    if isinstance(value, Object):
        for key, item in value.pairs:
            text_valid(key)
            validate_json(item)
    elif isinstance(value, Number):
        if not isinstance(value.lexeme, str) or not NUMBER.fullmatch(value.lexeme):
            raise ValueError('invalid Json number token')
    elif isinstance(value, list):
        for item in value:
            validate_json(item)
    elif isinstance(value, str):
        text_valid(value)
    elif value is not None and type(value) is not bool:
        raise TypeError('invalid typed Json value')


def parse(text):
    text_valid(text)
    def bad_constant(token):
        raise ValueError('invalid Json constant ' + token)
    value = json.loads(text, object_pairs_hook=Object, parse_int=Number,
                       parse_float=Number, parse_constant=bad_constant)
    validate_json(value)
    return value


def quote(text):
    text_valid(text)
    short = {'"': '\\"', '\\': '\\\\', '\t': '\\t', '\r': '\\r', '\n': '\\n'}
    return '"' + ''.join(short[c] if c in short else
                          '\\u%04x' % ord(c) if ord(c) < 32 or 127 <= ord(c) <= 159
                          else c for c in text) + '"'


def encode(value):
    validate_json(value)
    def emit(v):
        if isinstance(v, Object):
            return '{' + ','.join(quote(k) + ':' + emit(x) for k, x in v.pairs) + '}'
        if isinstance(v, Number):
            return v.lexeme
        if isinstance(v, list):
            return '[' + ','.join(emit(x) for x in v) + ']'
        if isinstance(v, str):
            return quote(v)
        return 'null' if v is None else 'true' if v else 'false'
    return emit(value)


def strict_i64(value):
    if not isinstance(value, Number) or not INTEGER.fullmatch(value.lexeme):
        raise ValueError('capacity requires strict integer')
    token = value.lexeme
    negative = token.startswith('-')
    digits = token[1:] if negative else token
    limit = '9223372036854775808' if negative else '9223372036854775807'
    if len(digits) > len(limit) or (len(digits) == len(limit) and digits > limit):
        raise ValueError('capacity outside signed-i64')
    return int(token)  # lexical bound above guarantees at most 19 digits


def typed_capacity(capacity):
    if type(capacity) is not int:
        raise TypeError('typed capacity requires int')
    if capacity < I64_MIN or capacity > I64_MAX:
        raise ValueError('typed capacity outside signed-i64')
    if capacity < 0:
        raise DomainError('VALUE')
    return capacity


def selected(obj, name, category):
    values = [v for k, v in obj.pairs if k == name]
    if len(values) != 1:
        raise DomainError(category)
    return values[0]


def state_object(capacity, entries):
    return Object([('capacity', Number(str(capacity))), ('entries', Object(entries))])


def validated_state(state):
    validate_json(state)  # invalid typed transport remains caller misuse
    if not isinstance(state, Object) or len(state.pairs) != 2:
        raise DomainError('VALUE')
    cap = selected(state, 'capacity', 'VALUE')
    entries = selected(state, 'entries', 'VALUE')
    try:
        capacity = strict_i64(cap)
    except ValueError:
        raise DomainError('VALUE') from None
    if capacity < 0 or not isinstance(entries, Object):
        raise DomainError('VALUE')
    if len(entries.pairs) > capacity or len({k for k, _ in entries.pairs}) != len(entries.pairs):
        raise DomainError('VALUE')
    return capacity, OrderedDict(copy.deepcopy(entries.pairs))


def create(capacity):
    return state_object(typed_capacity(capacity), [])


def get(state, key):
    capacity, entries = validated_state(state)
    text_valid(key)
    hit = key in entries
    value = copy.deepcopy(entries[key]) if hit else None
    if hit:
        entries.move_to_end(key)
    return Object([('state', state_object(capacity, entries.items())), ('hit', hit), ('value', value)])


def put(state, key, value):
    capacity, entries = validated_state(state)
    text_valid(key)
    validate_json(value)
    replaced = key in entries
    evicted = None
    if capacity:
        if not replaced and len(entries) == capacity:
            old_key, old_value = entries.popitem(last=False)
            evicted = Object([('key', old_key), ('value', copy.deepcopy(old_value))])
        entries[key] = copy.deepcopy(value)
        entries.move_to_end(key)
    return Object([('state', state_object(capacity, entries.items())),
                   ('replaced', replaced), ('evicted', evicted)])


def validated_operations(operations):
    validate_json(operations)
    if not isinstance(operations, list):
        raise DomainError('VALUE')
    validated = []
    for operation in operations:
        if not isinstance(operation, Object):
            raise DomainError('VALUE')
        op = selected(operation, 'op', 'VALUE')
        key = selected(operation, 'key', 'VALUE')
        if op not in ('get', 'put') or not isinstance(op, str) or not isinstance(key, str):
            raise DomainError('VALUE')
        value = selected(operation, 'value', 'VALUE') if op == 'put' else None
        validated.append((op, key, value))
    return validated


def execute_validated(state, validated):
    """Shared transition execution; callers validate before entering this helper."""
    events = []
    for op, key, value in validated:
        transition = get(state, key) if op == 'get' else put(state, key, value)
        fields = dict(transition.pairs)
        state = fields['state']
        events.append(Object([('op', op), ('key', key)] + list(transition.pairs[1:])))
    return copy.deepcopy(Object([('state', state), ('events', events)]))


def run(capacity, operations):
    state = create(capacity)  # typed capacity priority precedes operation validation
    validated = validated_operations(operations)  # all shapes before transitions
    return execute_validated(state, validated)


def failure(category):
    return Object([('ok', False), ('state', None), ('events', []), ('error', category)])


def evaluate_request(text):
    try:
        request = parse(text)
    except (ValueError, TypeError):
        return failure('REQUEST')
    if not isinstance(request, Object):
        return failure('REQUEST')
    capacities = [v for k, v in request.pairs if k == 'capacity']
    sequences = [v for k, v in request.pairs if k == 'operations']
    if len(capacities) != 1 or len(sequences) != 1:
        return failure('REQUEST')
    operations = sequences[0]
    try:
        capacity = strict_i64(capacities[0])
    except ValueError:
        return failure('REQUEST')
    if not isinstance(operations, list):
        return failure('REQUEST')
    if capacity < 0:
        return failure('CAPACITY')
    try:
        validated = validated_operations(operations)
    except DomainError as error:
        if error.category != 'VALUE':
            raise
        return failure('OPERATIONS')
    # Deliberately outside the validator catch: internal errors are not responses.
    result = execute_validated(create(capacity), validated)
    return Object([('ok', True)] + list(result.pairs) + [('error', '')])


def main(argv):
    requests = [arg[len('data.request='):] for arg in argv if arg.startswith('data.request=')]
    if len(requests) != 1 or len(argv) != 1:
        raise ValueError('CLI requires exactly one explicit data.request=<raw JSON> argument')
    output = encode(evaluate_request(requests[0])).encode('utf-8').hex()
    sys.stdout.write('out.result=hex:' + output + '\n')

if __name__ == '__main__':
    main(sys.argv[1:])
