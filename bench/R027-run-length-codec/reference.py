#!/usr/bin/env python3
"""Independent sequential RLE32 oracle, authored before the AIL implementation.

No balanced composition or AIL/runtime helper supplies expected values. Validate
all packets and calculate size before allocating any decoded output.
"""
import re
import sys

MAX_RAW = 1_048_576
MAX_TRANSPORT = 4 + 2 * MAX_RAW


class CodecError(Exception):
    pass


def from_transport(text):
    if text is None:
        raise CodecError('BYTES')
    if len(text.encode('utf-8')) > MAX_TRANSPORT:
        raise CodecError('BYTES-LIMIT')
    if re.fullmatch(r'hex:(?:[0-9a-f]{2})*', text) is None:
        raise CodecError('BYTES')
    return bytes.fromhex(text[4:])


def to_transport(data):
    return 'hex:' + data.hex()


def encode(data):
    result = bytearray()
    start = 0
    while start < len(data):
        end = start + 1
        while end < len(data) and data[end] == data[start]:
            end += 1
        if len(result) + 5 > MAX_RAW:
            raise CodecError('BYTES')
        result.extend((end - start).to_bytes(4, 'big'))
        result.append(data[start])
        start = end
    return bytes(result)


def decoded_size(data):
    if len(data) % 5:
        raise CodecError('BYTES')
    size = 0
    for offset in range(0, len(data), 5):
        count = int.from_bytes(data[offset:offset + 4], 'big')
        if count == 0:
            raise CodecError('BYTES')
        size += count
    return size


def decode(data):
    size = decoded_size(data)
    if size > MAX_RAW:
        raise CodecError('BYTES')
    result = bytearray()
    for offset in range(0, len(data), 5):
        count = int.from_bytes(data[offset:offset + 4], 'big')
        result.extend(bytes([data[offset + 4]]) * count)
    return bytes(result)


def transform(mode, value):
    if mode not in ('encode', 'decode'):
        return {'out.error': 'MODE', 'out.value': 'hex:'}
    try:
        raw = from_transport(value)
        result = encode(raw) if mode == 'encode' else decode(raw)
        return {'out.error': '', 'out.value': to_transport(result)}
    except CodecError as error:
        if str(error) != 'BYTES':
            raise
        return {'out.error': 'BYTES', 'out.value': 'hex:'}


def main(arguments):
    state = dict(arg.split('=', 1) for arg in arguments if '=' in arg)
    try:
        result = transform(state.get('data.mode'), state.get('data.input'))
    except CodecError as error:
        print('AIL-RUNTIME-' + str(error), file=sys.stderr)
        return 1
    for key, value in sorted(result.items()):
        print(key + '=' + value)
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
