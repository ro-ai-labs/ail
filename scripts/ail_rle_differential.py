#!/usr/bin/env python3
"""Independent RLE32 acceptance: sequential Python expectations, actual files.

No codec/evaluator implementation is imported from AIL. Subprocess deadlines,
cleanup and artifact plumbing reuse the process-resource test harness only.
"""
import argparse
import collections
import ctypes
import hashlib
import json
from pathlib import Path
import random
import re
import runpy
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import ail_process_differential as harness

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / 'target/debug/ail'
RUNG = REPO / 'bench/R027-run-length-codec'
PACKAGE = REPO / 'examples/run-length-codec'
LIBRARY = REPO / 'examples/ail_std_rle.ail'
REFERENCE = runpy.run_path(str(RUNG / 'reference.py'))
FINGERPRINT = 'sha256:2084c99467f7ff315c452cd604436b08'
SEED = 0xA11E027
LIMIT = 1_048_576
COUNTS = collections.Counter()
STATE = re.compile(r'^(?!trace=)[a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*=')


def frozen():
    digest = hashlib.sha256()
    for path in sorted(RUNG.rglob('*')):
        if not path.is_file():
            continue
        relative = path.relative_to(RUNG)
        if relative.parts[0] == 'program.ail':
            continue
        digest.update(str(relative).encode() + b'\0' + path.read_bytes() + b'\0')
    actual = 'sha256:' + digest.hexdigest()[:32]
    assert actual == FINGERPRINT, ('frozen R027 inputs changed', actual)


def packet(count, value):
    return count.to_bytes(4, 'big') + bytes([value])


def transport(raw):
    return 'hex:' + raw.hex()


def checked(command):
    result = harness.invoke(command, timeout=90)
    assert result.returncode == 0, (command[:6], result.stdout[-2000:], result.stderr[-2000:])
    return result.stdout


def build(root, package, stem, actions, full=True):
    frontends = [('source', [package])]
    if full:
        core = root / f'{stem}.core'
        core.write_bytes(checked([AIL, 'ail-core', package]))
        rendered = root / f'{stem}-rendered'
        rendered.mkdir()
        (rendered / 'ail-package.md').write_text(
            f'# AIL Package\nname: {stem}-rendered\nversion: 1.0.0\nprofile: Application\n'
            'entry: spec.ail-spec.md\nfeatures: things, actions, functions, bytes, expressions, control-flow\n'
            'conformance: first-slice\n')
        (rendered / 'spec.ail-spec.md').write_bytes(checked([AIL, 'ail-spec', '--core-file', core]))
        frontends += [('core', ['--core-file', core]), ('rendered', [rendered])]
    result = []
    for name, arguments in frontends:
        bytecode = root / f'{stem}-{name}.json'
        bytecode.write_bytes(checked([AIL, 'ail-lower', *arguments]))
        program = json.loads(bytecode.read_bytes())
        assert program['external_bindings'] == [] and program['capability_grants'] == []
        for action in program['actions']:
            for instruction in action['instructions']:
                opcode = instruction['opcode']
                assert not opcode.startswith(('PROCESS_', 'RLE_', 'CODEC_'))
                assert opcode not in ('HOST_EVAL', 'EVAL_EXPRESSION', 'CALL_EXTERNAL')
                if action['action'].startswith('Rle.'):
                    assert not opcode.startswith(('FILE_', 'DIRECTORY_', 'STDIN_', 'STDOUT_'))
        forms = [('direct', bytecode, arguments)]
        if full:
            ste = root / f'{stem}-{name}.ail-ste'
            ste.write_bytes(checked([AIL, 'ail-ste', *arguments]))
            forms.append(('ste', ste, [ste]))
        for form, artifact, compile_arguments in forms:
            binaries = {}
            for action in actions:
                binary = root / f'{stem}-{name}-{form}-{action}.elf'
                checked([AIL, 'ail-compile', *compile_arguments, '--action', action,
                         '--target', 'linux-x86_64-elf', '--out', binary])
                binaries[action] = binary
            for backend in ('VM', 'native'):
                result.append((f'{stem}/{name}/{form}/{backend}', artifact, binaries, backend))
            print(f'built {stem}/{name}/{form}', flush=True)
    return result


def run(route, action, arguments=(), stdio=False, **kwargs):
    _, artifact, binaries, backend = route
    command = [AIL, 'ail-vm', artifact, '--action', action] if backend == 'VM' else [binaries[action]]
    return harness.invoke(command + (['--stdio'] if stdio else []) + list(arguments), timeout=90, **kwargs)


def states(result, wanted, context):
    COUNTS['checks'] += 1
    assert result.returncode == 0 and not error_categories(result.stderr), (context, result.returncode, result.stdout[:500], result.stderr[-2000:])
    actual = [line for line in result.stdout.decode().splitlines() if STATE.match(line)]
    expected = [key + '=' + value for key, value in sorted(wanted.items())]
    assert actual == expected, (context, [line[:500] for line in actual], [line[:500] for line in expected])


def fatal(result, category, context):
    COUNTS['checks'] += 1
    assert result.returncode > 0 and result.stdout == b'', (context, result.returncode, result.stdout[:500], result.stderr[-1000:])
    assert error_categories(result.stderr) == [category], (context, result.stderr[-2000:])


def error_categories(stderr):
    return [code.decode('ascii') for code in re.findall(rb'AIL-RUNTIME-([A-Z-]+):', stderr)]


def assertion_self_check():
    assert error_categories(b'AIL-RUNTIME-BYTES: malformed\n') == ['BYTES']
    assert error_categories(b'AIL-RUNTIME-BYTES-LIMIT: full\n') != ['BYTES']
    assert error_categories(b'AIL-RUNTIME-BYTES-LIMIT: full\n') == ['BYTES-LIMIT']


def transform(route, label, mode, value):
    wanted = REFERENCE['transform'](mode, value)
    arguments = ['data.mode=' + mode] if mode is not None else []
    if value is not None:
        arguments.append('data.input=' + value)
    states(run(route, 'Transform', arguments), wanted, (route[0], label))


def fixed(routes):
    cases = []
    for path in sorted((RUNG / 'cases').glob('*.args')):
        state = dict(line.strip().split('=', 1) for line in path.read_text().splitlines()
                     if line.strip() and not line.lstrip().startswith('#'))
        cases.append((path.stem, state['data.mode'], state.get('data.input')))
    # Keep the frozen rung untouched; absent mode follows the same MODE fallback.
    cases += [('absent-mode-valid-bytes', None, 'hex:4141'),
              ('absent-mode-absent-bytes', None, None),
              ('absent-mode-malformed-bytes', None, 'not-a-byte-transport')]
    for route in routes:
        for case in cases:
            transform(route, *case)
        print(f'fixed {route[0]}: {len(cases)} (146 frozen + 3 missing-mode regressions)', flush=True)


def generated(routes):
    rng = random.Random(SEED)
    cases = []
    for index in range(96):
        runs = [(rng.randrange(1, 130), rng.randrange(256)) for _ in range(rng.randrange(1, 30))]
        if index % 3 == 0:
            # Intentionally adjacent equal runs challenge canonical encoder merge.
            value = rng.randrange(256)
            runs[0:0] = [(31, value), (32, value), (33, value)]
        raw = b''.join(bytes([value]) * count for count, value in runs)
        encoded = b''.join(packet(count, value) for count, value in runs)
        cases += [(f'encode-{index}', 'encode', transport(raw)),
                  (f'decode-{index}', 'decode', transport(encoded))]
        if index % 4 == 0:
            cases.append((f'truncated-{index}', 'decode', transport(encoded[:-rng.randrange(1, 5)])))
            insertion = rng.randrange(len(runs) + 1) * 5
            cases.append((f'zero-{index}', 'decode', transport(encoded[:insertion] + packet(0, 255) + encoded[insertion:])))
    for route in routes:
        for case in cases:
            transform(route, *case)
        print(f'generated {route[0]}: {len(cases)}', flush=True)


def file_check(route, action, source, output, raw, expected=None, error=None):
    source.write_bytes(raw)
    sentinel = b'preserve-existing-output\0\xff'
    output.write_bytes(sentinel)
    result = run(route, action, ['--read-file=input=' + str(source), '--write-file=output=' + str(output)], stdio=True)
    if error is not None:
        fatal(result, error, (route[0], action, len(raw)))
        assert output.read_bytes() == sentinel, (route[0], action, 'failed codec wrote output')
    else:
        COUNTS['checks'] += 1
        assert result.returncode == 0 and result.stdout == result.stderr == b'', (route[0], action, result.returncode, result.stdout[:500], result.stderr[-2000:])
        assert output.read_bytes() == expected, (route[0], action, len(raw), len(output.read_bytes()), len(expected))
    assert str(source).encode() not in result.stdout + result.stderr
    assert str(output).encode() not in result.stdout + result.stderr


def files(root, routes):
    source, output = root / 'input = binary.dat', root / 'output = binary.dat'
    rng = random.Random(SEED ^ 0xF11E)
    large_runs = b''.join(bytes([rng.randrange(256)]) * rng.randrange(256, 2048) for _ in range(120))
    ordinary = [b'', bytes(range(256)), b'\0\xff\x80\r\n' * 257,
                b'A' * 255 + b'B' * 256 + b'C' * 65535 + b'D' * 65536,
                b'Z' * LIMIT, large_runs]
    for route in routes:
        for raw in ordinary:
            encoded = REFERENCE['encode'](raw)
            file_check(route, 'EncodeFile', source, output, raw, encoded)
            file_check(route, 'DecodeFile', source, output, encoded, raw)
        # Adjacent equal packets are legal decoder input, including a full-size result.
        full = packet(65536, 0x80) * 16
        file_check(route, 'DecodeFile', source, output, full, b'\x80' * LIMIT)
        for encoded in [packet(0, 0), packet(3, 1) + packet(0, 2), packet(0xffffffff, 1),
                        packet(LIMIT + 1, 0), packet(LIMIT, 0) + packet(1, 0),
                        packet(0xffffffff, 1) + packet(0, 2), packet(1, 0) + b'\0']:
            file_check(route, 'DecodeFile', source, output, encoded, error='BYTES')
        # 209,716 one-byte runs require1,048,580 encoded bytes. Preflight domain
        # rejection must win before trying to construct an oversized Bytes value.
        expansion = bytes(range(256)) * 819 + bytes(range(52))
        assert len(expansion) == 209716 and len(expansion) * 5 > LIMIT
        file_check(route, 'EncodeFile', source, output, expansion, error='BYTES')
        file_check(route, 'EncodeFile', source, output, b'x' * (LIMIT + 1), error='BYTES-LIMIT')
        print(f'files {route[0]}', flush=True)


def helper_package(root):
    package = root / 'sizes'
    package.mkdir()
    (package / 'ail-package.md').write_text(
        '# AIL Package\nname: rle-size-oracle\nversion: 1.0.0\nprofile: Application\n'
        'entry: spec.ail-spec.md\nfeatures: things, actions, functions, bytes, expressions\n'
        f'imports: {LIBRARY} compatible ^0.1 as Rle\nconformance: first-slice\n')
    (package / 'spec.ail-spec.md').write_text('''The application SizeProbe manages bytes.

A Data has:
- body: Bytes
- size: Int

Action: EncodedSize.
When EncodedSize happens:
- the system reads bytes from file resource input into data.body
- the system calls Rle.encodedSize with data = data.body into data.size
- the system returns data.size
- the system records a trace event named Sized

Action: DecodedSize.
When DecodedSize happens:
- the system reads bytes from file resource input into data.body
- the system calls Rle.decodedSize with data = data.body into data.size
- the system returns data.size
- the system records a trace event named Sized
''')
    return package


def sizes(root, full):
    routes = build(root, helper_package(root), 'sizes', ('EncodedSize', 'DecodedSize'), full)
    source = root / 'sizes.bin'
    encoded = [(b'', 0), (b'A' * 65536, 5), (bytes(range(256)), 1280),
               (b'A' * 31 + b'B' * 32 + b'A' * 33, 15)]
    decoded = [(b'', 0), (packet(1, 0), 1), (packet(0xffffffff, 255), 0xffffffff),
               (packet(0xffffffff, 255) * 2048, 0xffffffff * 2048),
               (packet(256, 0) + packet(65536, 0), 65792)]
    for route in routes:
        for action, cases in [('EncodedSize', encoded), ('DecodedSize', decoded)]:
            for raw, size in cases:
                source.write_bytes(raw)
                states(run(route, action, ['--read-file=input=' + str(source)]), {'data.size': str(size)}, (route[0], action, len(raw)))
        source.write_bytes(packet(0xffffffff, 0) + packet(0, 1))
        fatal(run(route, 'DecodedSize', ['--read-file=input=' + str(source)], stdio=True), 'BYTES', (route[0], 'full validation before size'))
        print(f'sizes {route[0]}', flush=True)
    # Maximal input packet count still fits signed i64 and does not expand bytes.
    for route in [routes[0], routes[1]]:
        raw = packet(0xffffffff, 0) * (LIMIT // 5)
        source.write_bytes(raw)
        states(run(route, 'DecodedSize', ['--read-file=input=' + str(source)]),
               {'data.size': str((LIMIT // 5) * 0xffffffff)}, (route[0], 'maximal count sum'))
        raw = bytes(range(256)) * (LIMIT // 256)
        source.write_bytes(raw)
        states(run(route, 'EncodedSize', ['--read-file=input=' + str(source)]),
               {'data.size': str(LIMIT * 5)}, (route[0], 'oversized encoded result size'))


def mutation(root, routes):
    program = json.loads(routes[0][1].read_bytes())
    replacements = 0
    for action in program['actions']:
        if action['action'].startswith('Rle.'):
            for instruction in action['instructions']:
                if instruction['opcode'] == 'RAISE' and instruction['operands'] == {'category': 'BYTES'}:
                    instruction['operands']['category'] = 'TEXT'
                    replacements += 1
    assert replacements, 'library no longer explicitly raises BYTES; review mutation anchor'
    artifact = root / 'wrong-domain-error.json'
    artifact.write_text(json.dumps(program))
    binary = root / 'wrong-domain-error.elf'
    checked([AIL, 'ail-compile', artifact, '--action', 'Transform', '--target', 'linux-x86_64-elf', '--out', binary])
    encoded = transport(packet(0, 255))
    wanted = REFERENCE['transform']('decode', encoded)
    assert wanted == {'out.error': 'BYTES', 'out.value': 'hex:'}
    for backend in ('VM', 'native'):
        route = ('mutated/' + backend, artifact, {'Transform': binary}, backend)
        result = run(route, 'Transform', ['data.mode=decode', 'data.input=' + encoded])
        fatal(result, 'TEXT', route[0])
    print(f'mutation: changing {replacements} domain raises from BYTES to TEXT rejected on both backends', flush=True)

    program = json.loads(routes[0][1].read_bytes())
    function = next(action for action in program['actions'] if action['action'] == 'Rle.readCount')
    returns = [instruction for instruction in function['instructions']
               if instruction['opcode'] == 'RETURN_VALUE' and instruction['operands']['type'] == 'Int']
    assert len(returns) == 1 and 'i:256' in returns[0]['operands']['expression'].split()
    returns[0]['operands']['expression'] = ' '.join(
        'i:255' if token == 'i:256' else token for token in returns[0]['operands']['expression'].split())
    artifact, binary = root / 'wrong-count.json', root / 'wrong-count.elf'
    artifact.write_text(json.dumps(program))
    checked([AIL, 'ail-compile', artifact, '--action', 'Transform', '--target', 'linux-x86_64-elf', '--out', binary])
    encoded = transport(packet(256, 65))
    wanted = REFERENCE['transform']('decode', encoded)
    expected = [key + '=' + value for key, value in sorted(wanted.items())]
    for backend in ('VM', 'native'):
        route = ('mutated-count/' + backend, artifact, {'Transform': binary}, backend)
        result = run(route, 'Transform', ['data.mode=decode', 'data.input=' + encoded])
        COUNTS['checks'] += 1
        assert result.returncode == 0 and not error_categories(result.stderr), (route[0], result.stderr)
        actual = [line for line in result.stdout.decode().splitlines() if STATE.match(line)]
        assert actual != expected, (route[0], 'oracle failed to detect a corrupted big-endian count')
    print('mutation: changing the count byte multiplier 256 to 255 rejected on both backends', flush=True)


def budgets(root, routes):
    program = json.loads(routes[0][1].read_bytes())
    action = next(action for action in program['actions'] if action['action'] == 'Transform')
    op = harness.op
    raw_size = (LIMIT - 4) // 2
    prefix = [op('ASSIGN_LITERAL', key='budget.raw', type='Bytes', value=json.dumps(transport(b'a' * raw_size))),
              op('EVAL_INT', key='budget.index', expression='i:0'), op('LABEL', name='budget_head'),
              op('BRANCH_INT', left='f:budget.index', comparison='>=', right='i:64', label='budget_done'),
              op('BYTES_SLICE', source='budget.raw', start='i:0', end='i:' + str(raw_size), key='budget.result'),
              op('EVAL_INT', key='budget.index', expression='f:budget.index i:1 +'), op('JUMP', label='budget_head'),
              op('LABEL', name='budget_done')]
    assert action['instructions'][0]['opcode'] == 'ACTION_BEGIN'
    action['instructions'][1:1] = prefix
    artifact, binary = root / 'exhausted-arena.json', root / 'exhausted-arena.elf'
    artifact.write_text(json.dumps(program))
    checked([AIL, 'ail-compile', artifact, '--action', 'Transform', '--target', 'linux-x86_64-elf', '--out', binary])
    for backend in ('VM', 'native'):
        route = ('exhausted-arena/' + backend, artifact, {'Transform': binary}, backend)
        # Literal empty/error results and mode validation require no arena charge.
        states(run(route, 'Transform', ['data.mode=unknown']), REFERENCE['transform']('unknown', None), route[0])
        # The BYTES handler must not catch fatal allocation exhaustion.
        fatal(run(route, 'Transform', ['data.mode=encode', 'data.input=hex:41']), 'BYTES-LIMIT', route[0])
    print('shared allocation exhaustion remains fatal; unknown mode still bypasses codec', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--section', choices=['all', 'fixed', 'generated', 'files', 'sizes', 'mutation', 'budgets'], default='all')
    args = parser.parse_args()
    assertion_self_check()
    frozen()
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix='ail-rle-oracle-') as directory:
        root = Path(directory)
        if args.section != 'sizes':
            actions = ('Transform', 'EncodeFile', 'DecodeFile') if args.section in ('all', 'files') else ('Transform',)
            routes = build(root, PACKAGE, 'codec', actions, args.section not in ('mutation', 'budgets'))
            if args.section in ('all', 'fixed'):
                fixed(routes)
            if args.section in ('all', 'generated'):
                generated(routes)
            if args.section in ('all', 'files'):
                files(root, routes)
            if args.section in ('all', 'mutation'):
                mutation(root, routes)
            if args.section in ('all', 'budgets'):
                budgets(root, routes)
        if args.section in ('all', 'sizes'):
            sizes(root, args.section == 'all')
    frozen()
    print(json.dumps({'seed': hex(SEED), 'checks': COUNTS['checks'], 'commands': harness.COUNTS['executions'],
                      'seconds': round(time.monotonic() - started, 2), 'frozen_fingerprint': FINGERPRINT}, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
