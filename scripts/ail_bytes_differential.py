#!/usr/bin/env python3
"""Independent Bytes and binary-file oracle. Build target/debug/ail first.

Expected byte results use Python bytes, slicing and strict codecs, never AIL
helpers. Existing process-oracle code supplies only artifact/subprocess plumbing.
Every command is bounded and killed/reaped on timeout, including private groups.
"""
import argparse
import ctypes
import json
from pathlib import Path
import random
import re
import resource
import signal
import struct
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import ail_process_differential as h

AIL = h.AIL
SEED = 0xB17E5026
LIMIT = 1_048_576
COUNTS = {'checks': 0}
STATE = re.compile(r'^(?!trace=)[a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*=')
op, seed = h.op, h.seed


def transport(raw):
    return 'hex:' + raw.hex()


def literal(key, value, kind='Bytes'):
    return op('ASSIGN_LITERAL', key=key, type=kind, value=json.dumps(value, ensure_ascii=False))


def display(key='data.result', kind='Bytes'):
    conversion = op('BYTES_TO_HEX', source=key, key='data.report') if kind == 'Bytes' else op('JSON_FORMAT', source=key, key='data.report', type=kind)
    return [conversion, op('STDOUT_WRITE_TEXT', source='data.report')]


def execute(artifacts, backend, arguments=(), input=b'', *, action='Run', stdio=True, **kwargs):
    prefix = [AIL, 'ail-vm', artifacts[0], '--action', action] if backend == 'VM' else [artifacts[1]]
    return h.invoke(prefix + (['--stdio'] if stdio else []) + list(arguments), input, timeout=30, **kwargs)


def exact(result, wanted=b''):
    COUNTS['checks'] += 1
    assert result.returncode == 0, (result.returncode, result.stdout[:500], result.stderr[-2000:])
    assert result.stdout == wanted, (len(result.stdout), result.stdout[:700], len(wanted), wanted[:700])
    assert result.stderr == b'', result.stderr[-1000:]


def fatal(result, category):
    COUNTS['checks'] += 1
    assert result.returncode > 0, (result.returncode, result.stdout[:500], result.stderr[-1000:])
    assert result.stdout == b'', result.stdout[:500]
    assert f'AIL-RUNTIME-{category}:'.encode() in result.stderr, result.stderr[-1500:]


def states(result, wanted):
    COUNTS['checks'] += 1
    assert result.returncode == 0, (result.returncode, result.stdout[:500], result.stderr[-2000:])
    actual = [line for line in result.stdout.decode().splitlines() if STATE.match(line)]
    assert actual == [key + '=' + value for key, value in sorted(wanted.items())], (actual, wanted)


SOURCE = '''The application Octets manages bytes.

A Data has:
- a: Bytes
- b: Bytes
- text: Text
- hex: Text
- start: Int
- end: Int
- index: Int
- value: Int

An Output has:
- length: Int
- got: Int
- part: Bytes
- joined: Bytes
- changed: Bytes
- encoded: Bytes
- decoded: Text
- parsed: Bytes
- hex: Text
- order: Int
- recursive: Bytes

Function: repeat.
The function needs:
- body: Bytes
- n: Int
The function produces:
- result: Bytes
The function stores:
- child: Bytes
- result: Bytes
- part: Bytes
- other: Bytes
- text: Text
- hex: Text
- count: Int
- value: Int
- order: Int
The function executes:
- if local.n == 0:
- the function counts bytes in local.body into local.count
- the function gets byte 0 from bytes local.body into local.value
- the function slices bytes local.body from 0 to 1 into local.part
- the function joins bytes local.part and local.body into local.other
- the function sets byte 0 in bytes local.other to 255 into local.other
- the function assigns "é" to local.text
- the function encodes text local.text as UTF-8 bytes into local.part
- the function decodes bytes local.part as UTF-8 text into local.text
- the function assigns "80FF" to local.hex
- the function decodes hexadecimal text local.hex into bytes local.part
- the function encodes bytes local.part as hexadecimal text into local.hex
- the function compares bytes local.body with local.part into local.order
- the function returns local.body
- end if
- the function calls repeat with body = local.body; n = local.n - 1 into local.child
- the function joins bytes local.body and local.child into local.result
- the function returns local.result

Action: Run.
When Run happens:
- the system counts bytes in data.a into output.length
- the system gets byte data.index from bytes data.a into output.got
- the system slices bytes data.a from data.start to data.end into output.part
- the system joins bytes data.a and data.b into output.joined
- the system sets byte data.index in bytes data.a to data.value into output.changed
- the system encodes text data.text as UTF-8 bytes into output.encoded
- the system decodes bytes output.encoded as UTF-8 text into output.decoded
- the system decodes hexadecimal text data.hex into bytes output.parsed
- the system encodes bytes output.parsed as hexadecimal text into output.hex
- the system compares bytes data.a with data.b into output.order
- the system calls repeat with body = data.a; n = 3 into output.recursive
- the system returns output.length
- the system returns output.got
- the system returns output.part
- the system returns output.joined
- the system returns output.changed
- the system returns output.encoded
- the system returns output.decoded
- the system returns output.parsed
- the system returns output.hex
- the system returns output.order
- the system returns output.recursive
- the system records a trace event named Done
'''


def pure(root):
    routes = h.routes(root, h.package(root, 'bytes-pure', SOURCE), 'bytes-pure')
    rng = random.Random(SEED)
    cases = [(bytes(range(256)), b'\0\xff\x80', 'Aé🙂e\u0301', 'Aa00Ff', -1, -(1 << 63), (1 << 63) - 1, 127),
             (b'\0', b'', '', '', 0, 1, 0, 255),
             (b'abc', b'abc', '漢', '000102', -3, -2, -1, 0)]
    for _ in range(61):
        raw = bytes(rng.randrange(256) for _ in range(rng.randrange(1, 257)))
        other = bytes(rng.randrange(256) for _ in range(rng.randrange(257)))
        text = ''.join(rng.choice('aé🙂漢\u0301') for _ in range(rng.randrange(15)))
        encoded = bytes(rng.randrange(256) for _ in range(rng.randrange(100))).hex()
        if rng.randrange(2):
            encoded = encoded.upper()
        cases.append((raw, other, text, encoded, rng.randrange(-len(raw), len(raw)),
                      rng.choice([-(1 << 63), (1 << 63) - 1, rng.randrange(-300, 301)]),
                      rng.choice([-(1 << 63), (1 << 63) - 1, rng.randrange(-300, 301)]), rng.randrange(256)))
    for number, artifacts in enumerate(routes):
        for backend in ('VM', 'native'):
            for raw, other, text, hexadecimal, index, start, end, value in cases:
                changed = bytearray(raw)
                changed[index] = value
                wanted = {'output.length': str(len(raw)), 'output.got': str(raw[index]),
                          'output.part': transport(raw[start:end]), 'output.joined': transport(raw + other),
                          'output.changed': transport(changed), 'output.encoded': transport(text.encode()),
                          'output.decoded': text, 'output.parsed': transport(bytes.fromhex(hexadecimal)),
                          'output.hex': hexadecimal.lower(), 'output.order': str((raw > other) - (raw < other)),
                          'output.recursive': transport(raw * 4)}
                arguments = ['data.a=' + transport(raw), 'data.b=' + transport(other), 'data.text=' + text,
                             'data.hex=' + hexadecimal, 'data.index=' + str(index), 'data.start=' + str(start),
                             'data.end=' + str(end), 'data.value=' + str(value)]
                states(execute(artifacts, backend, arguments, stdio=False), wanted)
            print(f'pure route {number}/{backend}: {len(cases)}', flush=True)
    return routes


def invalid(root):
    operations = {
        'length': op('BYTES_LENGTH', source='data.a', key='data.result'),
        'get': op('BYTES_GET', source='data.a', index='f:data.index', key='data.result'),
        'slice': op('BYTES_SLICE', source='data.a', start='f:data.start', end='f:data.end', key='data.result'),
        'set': op('BYTES_SET', source='data.a', index='f:data.index f:data.divisor /', value='f:data.value f:data.zero /', key='data.result'),
        'concat': op('BYTES_CONCAT', left='data.a', right='data.b', key='data.result'),
        'from_hex': op('BYTES_FROM_HEX', source='data.a', key='data.result'),
        'from_utf8': op('BYTES_FROM_UTF8', source='data.a', key='data.result'),
        'to_utf8': op('BYTES_TO_UTF8', source='data.a', key='data.result'),
        'compare': op('BYTES_COMPARE', left='data.a', right='data.b', key='data.result'),
    }
    compiled = {name: h.program(root, 'invalid-' + name, [instruction]) for name, instruction in operations.items()}
    cases = [('length', value, {}, 'BYTES') for value in [None, '', 'HEX:', 'hex:0', 'hex:AA', 'hex:gg', 'hex:00 ', '00', 'hex:\x00']]
    cases += [('get', 'hex:', {'index': '0'}, 'INDEX'), ('get', 'hex:00', {'index': '-9223372036854775808'}, 'INDEX'),
              ('get', 'hex:00', {'index': 'bad'}, 'INT'), ('get', 'hex:FF', {'index': 'bad'}, 'BYTES'),
              ('slice', 'hex:FF', {'start': 'bad', 'end': 'bad'}, 'BYTES'),
              ('slice', 'hex:00', {'start': 'bad', 'end': 'bad'}, 'INT'),
              ('set', 'hex:FF', {'index': '99', 'divisor': '0', 'value': '256', 'zero': '0'}, 'BYTES'),
              ('set', 'hex:00', {'index': '99', 'divisor': '1', 'value': '256', 'zero': '0'}, 'DIVZERO'),
              ('set', 'hex:00', {'index': '99', 'divisor': '1', 'value': '256', 'zero': '1'}, 'INDEX'),
              ('set', 'hex:00', {'index': '0', 'divisor': '1', 'value': '-1', 'zero': '1'}, 'BYTES'),
              ('set', 'hex:00', {'index': '0', 'divisor': '1', 'value': '256', 'zero': '1'}, 'BYTES'),
              ('concat', 'hex:00', {'b': 'hex:FF'}, 'BYTES'), ('compare', 'hex:', {}, 'BYTES')]
    cases += [('from_hex', value, {}, 'BYTES') for value in ['0', 'gg', '00 ', 'hex:00', '０１', '+1']]
    cases += [('from_hex', None, {}, 'TEXT'), ('from_utf8', None, {}, 'TEXT')]
    bad_utf8 = [b'\xff', b'\xc0\x80', b'\xed\xa0\x80', b'\xf4\x90\x80\x80', b'\xe2\x82', b'\x80', b'\xf5\x80\x80\x80']
    cases += [('to_utf8', transport(raw), {}, 'TEXT') for raw in bad_utf8]
    for name, value, fields, category in cases:
        artifacts = compiled[name]
        arguments = ['data.' + key + '=' + field for key, field in fields.items()]
        if value is not None:
            if '\0' in value:
                artifacts = h.program(root, 'nul-transport', [seed('data.a', value), operations[name]])
            else:
                arguments += ['data.a=' + value]
        for backend in ('VM', 'native'):
            fatal(execute(artifacts, backend, arguments), category)
    # Invalid native UTF-8 must be classified before from-hex syntax inspection.
    for name in ('from_hex', 'from_utf8'):
        fatal(execute(compiled[name], 'native', [b'data.a=\xff']), 'TEXT')
    # All Unicode boundary scalars, NUL and combining marks roundtrip by length.
    for text in ['\0', '\x7f\x80\u07ff\u0800\ud7ff\ue000\uffff\U00010000\U0010ffff', 'e\u0301🙂']:
        artifacts = h.program(root, 'unicode-' + str(len(text)), [seed('data.text', text),
            op('BYTES_FROM_UTF8', source='data.text', key='data.raw'), op('BYTES_TO_UTF8', source='data.raw', key='data.result'),
            op('STDOUT_WRITE_TEXT', source='data.result')])
        for backend in ('VM', 'native'):
            exact(execute(artifacts, backend), text.encode())
    print('invalid transports, strict Unicode and numeric priority', flush=True)


FILE_SOURCE = '''The application Binary manages files.

A Data has:
- raw: Bytes
- count: Int

Action: Run.
When Run happens:
- the system reads bytes from file resource input into data.raw
- the system writes bytes data.raw to file resource output
- the system counts bytes in data.raw into data.count
- the system returns data.count
- the system records a trace event named Copied
'''


def files(root):
    routes = h.routes(root, h.package(root, 'bytes-files', FILE_SOURCE), 'bytes-files')
    source, destination = root / 'input = arbitrary.bin', root / 'output = exact.bin'
    values = [b'', bytes(range(256)), b'\0\xff\x80\r\n' * 100, b'\xff' * LIMIT]
    for number, artifacts in enumerate(routes):
        for backend in ('VM', 'native'):
            for value in values:
                source.write_bytes(value)
                destination.write_bytes(b'preserve me until validated')
                result = execute(artifacts, backend, ['--read-file=input=' + str(source), '--write-file=output=' + str(destination)], stdio=False)
                states(result, {'data.count': str(len(value))})
                assert destination.read_bytes() == value
                assert str(root).encode() not in result.stdout + result.stderr
            source.write_bytes(b'x' * (LIMIT + 1))
            fatal(execute(artifacts, backend, ['--read-file=input=' + str(source), '--write-file=output=' + str(destination)]), 'BYTES-LIMIT')
            print(f'files route {number}/{backend}', flush=True)
    writer = h.program(root, 'bytes-writer', [op('FILE_WRITE_BYTES', resource='output', source='data.raw')])
    reader = h.program(root, 'bytes-reader', [op('FILE_READ_BYTES', resource='input', key='data.raw')])
    text_reader = h.program(root, 'text-reader', [op('FILE_READ_TEXT', resource='input', key='data.text')])
    fifo = root / 'fifo'
    import os
    os.mkfifo(fifo)
    for backend in ('VM', 'native'):
        for raw in [None, 'hex:FF', 'hex:0', 'hex:gg']:
            destination.write_bytes(b'original\0\xff')
            arguments = [] if raw is None else ['data.raw=' + raw]
            fatal(execute(writer, backend, arguments), 'CAPABILITY')
            fatal(execute(writer, backend, ['--write-file=output=' + str(destination), *arguments]), 'BYTES')
            assert destination.read_bytes() == b'original\0\xff'
        for grant in ['--read-file', '--read-file=', '--read-file=unused', '--read-file==x', '--read-file=unused=', '--read-file=bad/name=x', '--write-file=unused=']:
            destination.write_bytes(b'original')
            fatal(execute(writer, backend, [grant, '--write-file=output=' + str(destination), 'data.raw=hex:00ff']), 'CAPABILITY')
            assert destination.read_bytes() == b'original'
        fatal(execute(reader, backend, ['input=' + str(source), 'read-file.input=' + str(source)]), 'CAPABILITY')
        for path in [fifo, root, Path('/dev/null')]:
            fatal(execute(reader, backend, ['--read-file=input=' + str(path)]), 'IO')
            fatal(execute(writer, backend, ['--write-file=output=' + str(path), 'data.raw=hex:00']), 'IO')
        fatal(execute(writer, backend, ['--write-file=output=' + str(root / 'missing' / 'file'), 'data.raw=hex:00']), 'IO')
        source.write_bytes(b'\xff')
        fatal(execute(text_reader, backend, ['--read-file=input=' + str(source)]), 'TEXT')
        # Caller-selected fixed-file symlinks retain ordinary filesystem semantics.
        link = root / ('link-' + backend)
        link.symlink_to(destination)
        exact(execute(writer, backend, ['--write-file=output=' + str(root / 'discarded'), '--write-file=output=' + str(link), 'data.raw=hex:00ff']))
        assert destination.read_bytes() == b'\0\xff' and not (root / 'discarded').exists()
        exact(execute(writer, backend, ['--write-file=output=' + str(destination), 'data.raw=hex:']))
        assert destination.read_bytes() == b''
        def short_write():
            resource.setrlimit(resource.RLIMIT_FSIZE, (17, 17))
            signal.signal(signal.SIGXFSZ, signal.SIG_IGN)
        fatal(execute(writer, backend, ['--write-file=output=' + str(destination), 'data.raw=' + transport(bytes(range(256)))], prepare=short_write), 'IO')
        assert destination.read_bytes() == bytes(range(17)), 'partial writes are observable, not rolled back'
    print('file grants, validation before truncation, regular-only opens, exact/partial writes', flush=True)


def loop(body):
    return [seed('data.counter', '0'), op('LABEL', name='head'),
            op('BRANCH_INT', left='f:data.counter', comparison='<', right='f:data.iterations', label='body'),
            op('JUMP', label='done'), op('LABEL', name='body'), *body,
            op('EVAL_INT', expression='f:data.counter i:1 +', key='data.counter'), op('JUMP', label='head'),
            op('LABEL', name='done')]


def function(name, parameters, locals, body, result='Bytes'):
    return {'action': name, 'instructions': [op('VALUE_FUNCTION', parameters=parameters, locals=locals, result=result), *body]}


def identity():
    return function('identity', 'body:Bytes', '', [op('RETURN_VALUE', expression='f:local.body', type='Bytes')])


def aliases_and_functions(root):
    raw = b'\0\1\0\1'
    artifacts = h.program(root, 'aliases', [literal('data.raw', transport(raw)),
        op('COPY_FIELD', source='data.raw', key='data.old', text='save immutable alias'),
        *loop([op('TRY_BEGIN', errors='BYTES', key='data.error'),
               op('BYTES_SET', source='data.raw', index='i:0', value='i:256', key='data.raw'),
               op('RECOVER'), op('TRY_END')]),
        op('BYTES_SET', source='data.raw', index='i:1', value='i:255', key='data.raw'),
        op('BYTES_CONCAT', left='data.raw', right='data.old', key='data.raw'),
        *display('data.raw'), seed('data.separator', '|'), op('STDOUT_WRITE_TEXT', source='data.separator'),
        *display('data.old'), op('STDOUT_WRITE_TEXT', source='data.separator'), op('STDOUT_WRITE_TEXT', source='data.error')])
    wanted = ((b'\0\xff\0\1' + raw).hex() + '|' + raw.hex() + '|AIL-RUNTIME-BYTES').encode()
    for backend in ('VM', 'native'):
        exact(execute(artifacts, backend, ['data.iterations=512']), wanted)
    # Runtime dependency from a Bytes signature/return only, with no Bytes opcode.
    only_function = h.program(root, 'signature-only', [seed('data.raw', transport(raw)),
        op('CALL_VALUE', target='identity', arguments='body=f:data.raw', key='out.result'),
        op('OUTPUT_FIELD', key='out.result')], [identity()])
    for backend in ('VM', 'native'):
        states(execute(only_function, backend, stdio=False), {'out.result': transport(raw)})
    borrowed = function('inspect', 'body:Bytes', '', [op('RETURN_VALUE', expression='i:7', type='Int')], 'Int')
    inspect = h.program(root, 'validate-unused-parameter', [op('CALL_VALUE', target='inspect', arguments='body=f:data.raw', key='data.result'), *display(kind='Int')], [borrowed])
    for backend in ('VM', 'native'):
        exact(execute(inspect, backend, ['data.raw=hex:']), b'7')
        fatal(execute(inspect, backend, ['data.raw=hex:FF']), 'BYTES')
        fatal(execute(inspect, backend), 'BYTES')
    recursive = function('depth', 'body:Bytes;n:Int', 'child:Bytes', [
        op('BRANCH_INT', left='f:local.n', comparison='==', right='i:0', label='base'),
        op('CALL_VALUE', target='depth', arguments='body=f:local.body;n=f:local.n i:1 -', key='local.child'),
        op('RETURN_VALUE', expression='f:local.child', type='Bytes'), op('LABEL', name='base'),
        op('RETURN_VALUE', expression='f:local.body', type='Bytes')])
    depth = h.program(root, 'bytes-depth', [seed('data.raw', 'hex:80'),
        op('CALL_VALUE', target='depth', arguments='body=f:data.raw;n=f:data.depth', key='data.result'), *display()], [recursive])
    for backend in ('VM', 'native'):
        exact(execute(depth, backend, ['data.depth=63']), b'80')
        fatal(execute(depth, backend, ['data.depth=64']), 'DEPTH')
    print('alias preservation, recovery cleanup, borrowed arguments and 64/65 recursive frames', flush=True)


def limits(root):
    raw_file = root / 'limit.bin'
    raw_file.write_bytes(b'a' * LIMIT)
    read = [op('FILE_READ_BYTES', resource='input', key='data.raw')]
    # File read itself produces the maximal legal 2,097,156-byte transport.
    raw_length = h.program(root, 'raw-limit', read + [op('BYTES_LENGTH', source='data.raw', key='data.result'), *display(kind='Int')])
    to_hex = h.program(root, 'hex-text-limit', read + [op('BYTES_TO_HEX', source='data.raw', key='data.hex'),
        op('TEXT_LENGTH', source='data.hex', key='data.result'), *display(kind='Int')])
    utf8 = h.program(root, 'utf8-raw-limit', [op('FILE_READ_TEXT', resource='input', key='data.text'),
        op('BYTES_FROM_UTF8', source='data.text', key='data.raw'), op('BYTES_LENGTH', source='data.raw', key='data.result'), *display(kind='Int')])
    from_hex = h.program(root, 'hex-input-limit', [op('FILE_READ_TEXT', resource='input', key='data.hex'),
        op('BYTES_FROM_HEX', source='data.hex', key='data.raw'), op('BYTES_LENGTH', source='data.raw', key='data.result'), *display(kind='Int')])
    for backend in ('VM', 'native'):
        raw_file.write_bytes(b'a' * LIMIT)
        grant = ['--read-file=input=' + str(raw_file)]
        exact(execute(raw_length, backend, grant), str(LIMIT).encode())
        exact(execute(utf8, backend, grant), str(LIMIT).encode())
        fatal(execute(to_hex, backend, grant), 'TEXT-LIMIT')
        exact(execute(from_hex, backend, grant), str(LIMIT // 2).encode())
        raw_file.write_bytes(b'\xff' * (LIMIT // 2))
        exact(execute(to_hex, backend, grant), str(LIMIT).encode())
        raw_file.write_bytes(b'\xff' * (LIMIT // 2 + 1))
        fatal(execute(to_hex, backend, grant), 'TEXT-LIMIT')
    maximal = transport(b'a' * LIMIT)
    concat = h.program(root, 'concat-result-limit', [literal('data.raw', maximal),
        op('BYTES_CONCAT', left='data.raw', right='data.extra', key='data.result'),
        op('BYTES_LENGTH', source='data.result', key='data.count'), *display('data.count', 'Int')])
    for backend in ('VM', 'native'):
        exact(execute(concat, backend, ['data.extra=hex:']), str(LIMIT).encode())
        fatal(execute(concat, backend, ['data.extra=hex:00']), 'BYTES-LIMIT')
    # Exactly one MiB of canonical transport: 4 prefix + 2*524286 raw bytes.
    one_mib = transport(b'a' * ((LIMIT - 4) // 2))
    inspector = function('inspect', 'body:Bytes', '', [op('BYTES_LENGTH', source='local.body', key='local.length'),
        op('RETURN_VALUE', expression='f:local.length', type='Int')], 'Int')
    inspector['instructions'][0]['operands']['locals'] = 'length:Int'
    borrowed = h.program(root, 'bytes-borrowed-arena', [literal('data.raw', one_mib),
        *loop([op('CALL_VALUE', target='inspect', arguments='body=f:data.raw', key='data.result')]), *display(kind='Int')], [inspector])
    for backend in ('VM', 'native'):
        exact(execute(borrowed, backend, ['data.iterations=128']), str((LIMIT - 4) // 2).encode())
    tail = [
        op('BRANCH_INT', left='f:data.extra', comparison='==', right='i:1', label='bytes'),
        op('BRANCH_INT', left='f:data.extra', comparison='==', right='i:2', label='list'),
        op('BRANCH_INT', left='f:data.extra', comparison='==', right='i:3', label='json'),
        op('BRANCH_INT', left='f:data.extra', comparison='==', right='i:4', label='file'),
        op('BYTES_LENGTH', source='data.result', key='data.count'), *display('data.count', 'Int'), op('RETURN_SUCCESS'),
        op('LABEL', name='bytes'), op('BYTES_SLICE', source='data.raw', start='i:0', end='i:0', key='data.result'), op('RETURN_SUCCESS'),
        op('LABEL', name='list'), op('LIST_EMPTY', key='data.list', type='Int'), op('RETURN_SUCCESS'),
        op('LABEL', name='json'), seed('data.json', 'null'), op('JSON_PARSE', key='data.document', source='data.json', type='Json'), op('RETURN_SUCCESS'),
        op('LABEL', name='file'), op('FILE_READ_BYTES', resource='input', key='data.result')]
    arena = h.program(root, 'bytes-return-arena', [literal('data.raw', one_mib),
        *loop([op('CALL_VALUE', target='identity', arguments='body=f:data.raw', key='data.result')]), *tail], [identity()])
    raw_file.write_bytes(b'')
    for backend in ('VM', 'native'):
        exact(execute(arena, backend, ['data.iterations=64', 'data.extra=0']), str((LIMIT - 4) // 2).encode())
        fatal(execute(arena, backend, ['data.iterations=65', 'data.extra=0']), 'BYTES-LIMIT')
        for mode, category in [(1, 'BYTES-LIMIT'), (2, 'LIST-LIMIT'), (3, 'JSON-LIMIT'), (4, 'BYTES-LIMIT')]:
            fatal(execute(arena, backend, ['data.iterations=64', 'data.extra=' + str(mode), '--read-file=input=' + str(raw_file)]), category)
    forward = function('forward', 'body:Bytes', 'child:Bytes', [
        op('CALL_VALUE', target='identity', arguments='body=f:local.body', key='local.child'),
        op('RETURN_VALUE', expression='f:local.child', type='Bytes')])
    forwarding = h.program(root, 'bytes-forward-arena', [literal('data.raw', one_mib),
        *loop([op('CALL_VALUE', target='forward', arguments='body=f:data.raw', key='data.result')]),
        op('BYTES_LENGTH', source='data.result', key='data.count'), *display('data.count', 'Int')], [identity(), forward])
    file_arena = h.program(root, 'bytes-file-arena', [*loop([op('FILE_READ_BYTES', resource='input', key='data.raw')]),
        op('BYTES_LENGTH', source='data.raw', key='data.count'), *display('data.count', 'Int')])
    raw_file.write_bytes(b'a' * ((LIMIT - 4) // 2))
    for backend in ('VM', 'native'):
        exact(execute(forwarding, backend, ['data.iterations=32']), str((LIMIT - 4) // 2).encode())
        fatal(execute(forwarding, backend, ['data.iterations=33']), 'BYTES-LIMIT')
        exact(execute(file_arena, backend, ['data.iterations=64', '--read-file=input=' + str(raw_file)]), str((LIMIT - 4) // 2).encode())
        fatal(execute(file_arena, backend, ['data.iterations=65', '--read-file=input=' + str(raw_file)]), 'BYTES-LIMIT')
    print('raw/transport/Text limits and shared Bytes/List/Json return/file budgets', flush=True)


def imports(root):
    library = h.package(root, 'binary-library', FILE_SOURCE)
    caller = h.package(root, 'binary-importer', '''The application Imported manages binary files.

Action: Run.
When Run happens:
- the system calls Bin.Run
- the system returns bin.data.count
- the system records a trace event named Imported
''')
    manifest = caller / 'ail-package.md'
    manifest.write_text(manifest.read_text() + 'imports: ../binary-library compatible ^1.0 as Bin\n')
    assert library.is_dir()
    artifact, binary = root / 'imported.json', root / 'imported.elf'
    artifact.write_bytes(h.checked([AIL, 'ail-lower', caller]))
    h.checked([AIL, 'ail-compile', caller, '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', binary])
    source, destination = root / 'import-input.bin', root / 'import-output.bin'
    source.write_bytes(bytes(range(255, -1, -1)))
    for backend in ('VM', 'native'):
        states(execute((artifact, binary), backend, ['--read-file=bin.input=' + str(source), '--write-file=bin.output=' + str(destination)], stdio=False), {'bin.data.count': '256'})
        assert destination.read_bytes() == source.read_bytes()
        fatal(execute((artifact, binary), backend, ['--read-file=input=' + str(source), '--write-file=output=' + str(destination)]), 'CAPABILITY')
    print('imports qualify byte fields and fixed-file authority', flush=True)


def mutation(root):
    instructions = [literal('data.raw', 'hex:00'), op('BYTES_SET', source='data.raw', index='i:0', value='i:255', key='data.result'), *display()]
    original = h.program(root, 'mutation-original', instructions)
    changed = json.loads(original[0].read_bytes())
    target = next(i for i in changed['actions'][0]['instructions'] if i['opcode'] == 'BYTES_SET')
    target['operands']['value'] = 'i:254'
    artifact = root / 'mutation-wrong.json'
    artifact.write_text(json.dumps(changed))
    binary = root / 'mutation-wrong.elf'
    h.checked([AIL, 'ail-compile', artifact, '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', binary])
    for backend in ('VM', 'native'):
        exact(execute(original, backend), bytes([255]).hex().encode())
        result = execute((artifact, binary), backend)
        assert result.returncode == 0 and result.stdout == b'fe' and result.stderr == b''
        assert result.stdout != bytes([255]).hex().encode()
        COUNTS['checks'] += 1
    print('mutation: altered replacement rejected on VM/native', flush=True)


def demo(root):
    """Expected reports follow the documented header contract, not AIL helpers."""
    package = h.REPO / 'examples/binary-files'
    routes = h.routes(root, package, 'binary-demo', 'Inspect')
    signature = b'\x89PNG\r\n\x1a\n'

    def header(width, height, depth=8, color=6):
        # This example intentionally does not validate CRCs or compressed pixels.
        return signature + struct.pack('>I4sIIBBBBB', 13, b'IHDR', width, height, depth, color, 0, 0, 0) + b'\0' * 4

    fixture = (package / 'input.bin').read_bytes()
    cases = [(fixture, (2, 1, 8, 6)), (header(65537, 16777217), (65537, 16777217, 8, 6)),
             (header(2147483647, 2147483647, 16, 2), (2147483647, 2147483647, 16, 2)),
             (header(3, 7) + bytes(range(256)), (3, 7, 8, 6)),
             (header(0, 1), None), (header(1, 0), None), (header(2147483648, 1), None),
             (header(1, 1)[:32], None), (b'', None), (bytes(range(256)), None),
             (b'x' + header(1, 1)[1:], None),
             (signature + struct.pack('>I', 12) + header(1, 1)[12:], None),
             (header(1, 1)[:12] + b'IDAT' + header(1, 1)[16:], None)]
    source, copy, report = root / 'demo-input.bin', root / 'demo-copy.bin', root / 'demo-report.json'
    for number, artifacts in enumerate(routes):
        for backend in ('VM', 'native'):
            for value, recognized in cases:
                source.write_bytes(value)
                wanted = {'bytes': len(value), 'headerRecognized': recognized is not None,
                          'width': recognized[0] if recognized else 0, 'height': recognized[1] if recognized else 0,
                          'bitDepth': recognized[2] if recognized else 0, 'colorType': recognized[3] if recognized else 0}
                encoded = json.dumps(wanted, separators=(',', ':'))
                grants = ['--read-file=input=' + str(source), '--write-file=copy=' + str(copy), '--write-file=report=' + str(report)]
                states(execute(artifacts, backend, grants, action='Inspect', stdio=False), {'output.report': encoded})
                assert copy.read_bytes() == value
                assert report.read_bytes() == encoded.encode()
            print(f'demo route {number}/{backend}: {len(cases)} headers', flush=True)
    for backend in ('VM', 'native'):
        source.write_bytes(b'\0\xfflate write failure')
        fatal(execute(routes[0], backend, ['--read-file=input=' + str(source), '--write-file=copy=' + str(copy),
              '--write-file=report=' + str(root / 'missing' / 'report')], action='Inspect'), 'IO')
        assert copy.read_bytes() == source.read_bytes(), 'earlier successful copy must survive later report-write failure'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--section', choices=['all', 'pure', 'invalid', 'files', 'functions', 'limits', 'imports', 'mutation', 'demo'], default='all')
    args = parser.parse_args()
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0
    started = time.monotonic()
    sections = [('pure', pure), ('invalid', invalid), ('files', files), ('functions', aliases_and_functions), ('limits', limits), ('imports', imports), ('mutation', mutation), ('demo', demo)]
    with tempfile.TemporaryDirectory(prefix='ail-bytes-oracle-') as temporary:
        for name, run in sections:
            if args.section in ('all', name):
                run(Path(temporary))
    print(json.dumps({'seed': hex(SEED), 'checks': COUNTS['checks'], 'commands': h.COUNTS['executions'],
                      'seconds': round(time.monotonic() - started, 2)}, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
