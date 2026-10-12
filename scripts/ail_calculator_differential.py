#!/usr/bin/env python3
"""Independent, bounded acceptance oracle for the pure-AIL calculator.

Expected values come exclusively from the frozen recursive-descent/AST Python
reference, which deliberately differs from the application's postfix parser.
Only process deadlines/cleanup are reused from the process-resource test tool.
Run after building target/debug/ail; no Python is used by generated executables.
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
RUNG = REPO / 'bench/R026-expression-calculator'
PACKAGE = REPO / 'examples/expression-calculator'
REFERENCE = runpy.run_path(str(RUNG / 'reference.py'))
SEED = 0xCA1C026
LIMIT = 1_048_576
FINGERPRINT = 'sha256:1933508153ecb4338f90515c8b317322'
STATE = re.compile(r'^(?!trace=)[a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*=')
TRIM = ''.join(chr(n) for n in [*range(9, 14), *range(28, 33), 0x85, 0xa0,
                              0x1680, *range(0x2000, 0x200b), 0x2028,
                              0x2029, 0x202f, 0x205f, 0x3000])
COUNTS = collections.Counter()


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
    assert actual == FINGERPRINT, ('frozen calculator inputs changed', actual)


def expected(expression, variables='{}'):
    try:
        value = REFERENCE['parse_json'](variables)
    except (ValueError, UnicodeError):
        result = REFERENCE['failure']('REQUEST')
    else:
        result = REFERENCE['evaluate'](expression, value)
    return REFERENCE['encode'](result)


def request(expression, variables='{}'):
    return '{"expression":' + json.dumps(expression, ensure_ascii=False) + ',"variables":' + variables + '}'


def stream_expected(text):
    records = [REFERENCE['encode'](REFERENCE['evaluate_request'](line))
               for line in text.split('\n') if line.strip(TRIM)]
    return ''.join(record + '\n' for record in records).encode()


def checked(command):
    result = harness.invoke(command, timeout=90)
    assert result.returncode == 0, (command[:6], result.stdout[-1000:], result.stderr[-3000:])
    return result.stdout


def build(root, full):
    """Three frontends, each direct/STE, each VM/native; both entry actions."""
    frontends = [('source', [PACKAGE])]
    if full:
        core = root / 'calculator.core'
        core.write_bytes(checked([AIL, 'ail-core', PACKAGE]))
        rendered = root / 'rendered'
        rendered.mkdir()
        # Saved Core is flattened and already contains qualified Calc functions.
        # The rendered package therefore needs no second relative import.
        (rendered / 'ail-package.md').write_text(
            '# AIL Package\nname: calculator-rendered\nversion: 0.1.0\n'
            'profile: Application\nentry: spec.ail-spec.md\n'
            'features: things, actions, functions, text, lists, json, expressions, control-flow\n'
            'conformance: first-slice\n')
        (rendered / 'spec.ail-spec.md').write_bytes(checked([AIL, 'ail-spec', '--core-file', core]))
        frontends += [('core', ['--core-file', core]), ('rendered', [rendered])]
    routes = []
    for name, arguments in frontends:
        bytecode = root / f'{name}.ailbc.json'
        bytecode.write_bytes(checked([AIL, 'ail-lower', *arguments]))
        protocol = json.loads(bytecode.read_bytes())
        assert protocol['external_bindings'] == [] and protocol['capability_grants'] == []
        for action in protocol['actions']:
            for instruction in action['instructions']:
                opcode = instruction['opcode']
                assert not opcode.startswith(('PROCESS_', 'FILE_', 'DIRECTORY_'))
                assert opcode not in ('HOST_EVAL', 'EVAL_EXPRESSION', 'CALL_EXTERNAL')
                if action['action'].startswith('Calc.'):
                    assert not opcode.startswith(('STDIN_', 'STDOUT_'))
        forms = [('direct', bytecode, arguments)]
        if full:
            ste = root / f'{name}.ail-ste'
            ste.write_bytes(checked([AIL, 'ail-ste', *arguments]))
            forms.append(('ste', ste, [ste]))
        for form, artifact, compile_arguments in forms:
            binaries = {}
            for action in ('Evaluate', 'Stream'):
                binary = root / f'{name}-{form}-{action}.elf'
                checked([AIL, 'ail-compile', *compile_arguments, '--action', action,
                         '--target', 'linux-x86_64-elf', '--out', binary])
                binaries[action] = binary
            for backend in ('VM', 'native'):
                routes.append((f'{name}/{form}/{backend}', artifact, binaries, backend))
            print(f'built {name}/{form}', flush=True)
    return routes


def run(route, action, arguments=(), input=b'', stdio=False):
    _, artifact, binaries, backend = route
    prefix = [AIL, 'ail-vm', artifact, '--action', action] if backend == 'VM' else [binaries[action]]
    return harness.invoke(prefix + (['--stdio'] if stdio else []) + list(arguments),
                          input, timeout=30)


def extract(result):
    assert result.returncode == 0, (result.returncode, result.stdout[-1000:], result.stderr[-2000:])
    return [line for line in result.stdout.decode().splitlines() if STATE.match(line)]


def evaluate(route, label, expression, variables='{}'):
    wanted = ['out.result=' + expected(expression, variables)]
    result = run(route, 'Evaluate', ['data.expression=' + expression, 'data.variables=' + variables])
    actual = extract(result)
    assert actual == wanted, (route[0], label, expression[:300], variables[:300], actual, wanted)
    # This sentinel is intentionally unused input, never part of any result.
    assert b'calculator-private-sentinel' not in result.stdout + result.stderr
    COUNTS['evaluate'] += 1


def stream(route, label, text):
    result = run(route, 'Stream', input=text.encode(), stdio=True)
    wanted = stream_expected(text)
    assert result.returncode == 0, (route[0], label, result.returncode, result.stderr[-2000:])
    assert result.stdout == wanted, (route[0], label, result.stdout[:1500], wanted[:1500])
    assert result.stderr == b'', (route[0], label, result.stderr[:1000])
    COUNTS['stream_batches'] += 1
    COUNTS['stream_records'] += wanted.count(b'\n')


def fixed(routes):
    cases = []
    for case in sorted((RUNG / 'cases').glob('*.args')):
        state = dict(line.strip().split('=', 1) for line in case.read_text().splitlines()
                     if line.strip() and not line.lstrip().startswith('#'))
        cases.append((case.stem, state['data.expression'], state['data.variables']))
    for route in routes:
        for case in cases:
            evaluate(route, *case)
        print(f'fixed {route[0]}: {len(cases)}', flush=True)


def generated(routes, count):
    rng = random.Random(SEED)
    names = ['a', 'b', '_v2', 'A', 'zero', 'missing']
    leaves = ['0', '1', '2', '7', '0009', '42', '3037000500', '9223372036854775807', *names]
    separators = ['', ' ', '\t', '\v', '\f', '\r', '\n']

    def tree(depth):
        if depth == 0 or rng.random() < .28:
            return rng.choice(leaves)
        if rng.random() < .24:
            return rng.choice(['+', '-', '--', '-+', '+-+']) + tree(depth - 1)
        left, right = tree(depth - 1), tree(depth - 1)
        text = left + rng.choice(separators) + rng.choice('+-*/%') + rng.choice(separators) + right
        return '(' + text + ')' if rng.random() < .7 else text

    variables = ['{"a":-7,"b":3,"_v2":11,"A":4,"zero":0}',
                 '{"a":-9223372036854775808,"b":-1,"_v2":0,"A":1,"zero":0}',
                 '{"a":1.0,"b":true,"a":2,"_v2":null,"A":1e0,"zero":0}',
                 '{"a":9,"b":2,"_v2":7,"A":8,"zero":0,"unused":"calculator-private-sentinel","unused":[null]}']
    cases = []
    for index in range(count):
        expression = tree(rng.randrange(1, 6))
        if index % 5 == 0:
            expression += rng.choice([')', '(', '@', ' x', '1', ' +', ' **2', ' = 1', '🙂', '\u00a0'])
        cases.append(request(expression, rng.choice(variables)))
    distribution = collections.Counter(REFERENCE['evaluate_request'](text)['error'] or 'OK' for text in cases)
    assert all(distribution[code] for code in ('OK', 'LEX', 'SYNTAX', 'NAME', 'OVERFLOW', 'DIVZERO'))
    for route in routes:
        for start in range(0, len(cases), 16):
            stream(route, f'generated-{start}', '\n'.join(cases[start:start + 16]))
        print(f'generated {route[0]}: {len(cases)}', flush=True)
    print('generated distribution ' + json.dumps(dict(sorted(distribution.items()))), flush=True)


def streams(routes):
    good = request('a * -3 + 2', '{"a":7}')
    records = [good, '', ' ', '\t\r', TRIM.replace('\n', ''),
               '{', 'null', '[]', 'true', '42', '"request"', '{}',
               '{"expression":"1"}', '{"variables":{}}',
               '{"expression":1,"variables":{}}', '{"expression":null,"variables":{}}',
               '{"expression":"1","variables":[]}',
               '{"expression":"1","expression":"2","variables":{}}',
               '{"expression":"1","variables":{},"variables":{}}',
               '{"expression":"1","variables":{},"extra":1,"extra":2}',
               request('a', '{"a":1,"a":2}'), request('a', '{"a":-0}'),
               request('a', '{"a":1E0}'), request('a', '{"a":1e999}'),
               request('a', '{"a":true}'), request('a', '{"a":"1"}'),
               request('1', '{"unused":[true,null,{"x":1e999}]}'),
               request('1/0 + missing'), request('missing + 1/0'),
               request('1/0 +'), request('1/0 @'), request('9223372036854775808 +'),
               request('1\n+\t2\r*\v3\f'), request('\0'),
               '\v' + good, good + '\f', '\u00a0' + good + '\u00a0',
               '\t' + good + '\r ', '{"expression":"\\ud800","variables":{}}',
               '{"expression":"1","variables":{"unused":"\\udfff"}}',
               good + '\0', '{"expression":"1","variables":{},}',
               '{"expression":"1","variables":{"a":NaN}}']
    # A valid request after every bad or blank record detects accidental state
    # carry-over and confirms catchable errors do not terminate the stream.
    text = '\n'.join(item for record in records for item in (record, good))
    for route in routes:
        stream(route, 'request-shapes-and-recovery', text)
        stream(route, 'empty-input', '')
        stream(route, 'blank-input', TRIM)
        sample = (PACKAGE / 'input.jsonl').read_text()
        stream(route, 'checked-in-sample', sample)
        print(f'streams {route[0]}', flush=True)


def limits(routes):
    cases = [('space-exact', ' ' * 4096, '{}'), ('space-over', ' ' * 4097, '{}'),
             ('unicode-exact', '🙂' * 4096, '{}'), ('unicode-over', '🙂' * 4097, '{}'),
             ('ascii-whitespace', '\t\r\v\f1\t+\r2\v*\f3', '{}'),
             ('zero-exact', '0' * 4096, '{}'),
             ('literal-overflow-full', '9' * 4096, '{}'),
             ('identifier-exact', 'a' * 4096, '{' + json.dumps('a' * 4096) + ':7}'),
             ('selected-variable-low', 'a', '{"a":-9223372036854775809}'),
             ('lexer-before-name', 'missing @', '{}'),
             ('512-before-invalid', '+' * 512 + '@', '{}'),
             ('513-before-invalid', '+' * 513 + '@', '{}'),
             ('unary-512', '+' * 511 + '1', '{}'),
             ('deep-512', '+' + '(' * 255 + '1' + ')' * 255, '{}')]
    for route in routes:
        for case in cases:
            evaluate(route, *case)
        stream(route, 'expression-nul-escaped', request('1\0+2') + '\n' + request('7'))
        # Splitting one blank line adds exactly four JSON bytes: ["..."] .
        result = run(route, 'Stream', input=b' ' * (LIMIT - 4), stdio=True)
        assert result.returncode == 0 and result.stdout == result.stderr == b'', (route[0], result.stderr[-1000:])
        COUNTS['limits'] += 1
        for label, data, code in [('split-size', b' ' * (LIMIT - 3), 'LIST-LIMIT'),
                                  ('stdin-size', b'a' * (LIMIT + 1), 'TEXT-LIMIT'),
                                  ('stdin-utf8', request('7').encode() + b'\n\xff', 'TEXT')]:
            result = run(route, 'Stream', input=data, stdio=True)
            assert result.returncode > 0 and result.stdout == b'', (route[0], label, result.returncode, result.stdout[:500])
            assert ('AIL-RUNTIME-' + code + ':').encode() in result.stderr, (route[0], label, result.stderr[-1000:])
            COUNTS['limits'] += 1
        denied = run(route, 'Stream', input=request('1').encode())
        assert denied.returncode > 0 and b'AIL-RUNTIME-CAPABILITY:' in denied.stderr, (route[0], denied.stderr[-1000:])
        assert denied.stdout == b'', (route[0], denied.stdout[:500])
        COUNTS['limits'] += 1
        print(f'limits {route[0]}', flush=True)


def mutation(root, routes):
    original = routes[0][1]
    protocol = json.loads(original.read_bytes())
    candidates = [instruction for action in protocol['actions'] if action['action'] == 'Calc.applyBinary'
                  for instruction in action['instructions'] if instruction['opcode'] == 'RETURN_VALUE'
                  and instruction['operands'].get('expression') == 'f:local.left f:local.right +']
    assert len(candidates) == 1, 'arithmetic mutation anchor changed; review oracle sensitivity'
    candidates[0]['operands']['expression'] = 'f:local.left f:local.right -'
    artifact = root / 'mutated.ailbc.json'
    artifact.write_text(json.dumps(protocol))
    binary = root / 'mutated.elf'
    checked([AIL, 'ail-compile', artifact, '--action', 'Evaluate', '--target', 'linux-x86_64-elf', '--out', binary])
    for backend in ('VM', 'native'):
        route = ('mutated/' + backend, artifact, {'Evaluate': binary}, backend)
        result = run(route, 'Evaluate', ['data.expression=2+3', 'data.variables={}'])
        actual = extract(result)
        assert actual == ['out.result={"ok":true,"value":-1,"error":""}'], actual
        assert actual != ['out.result=' + expected('2+3')]
        COUNTS['mutation_rejections'] += 1
    print('mutation: wrong addition rejected on VM and native', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--section', choices=['all', 'routes', 'generated', 'streams', 'limits', 'mutation'], default='all')
    parser.add_argument('--generated-count', type=int, default=256)
    args = parser.parse_args()
    assert 64 <= args.generated_count <= 1024
    frozen()
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0, 'cannot become cleanup subreaper'
    start = time.monotonic()
    with tempfile.TemporaryDirectory(prefix='ail-calculator-oracle-') as temporary:
        root = Path(temporary)
        routes = build(root, args.section in ('all', 'routes'))
        if args.section in ('all', 'routes'):
            fixed(routes)
        if args.section in ('all', 'generated'):
            generated(routes, args.generated_count)
        if args.section in ('all', 'streams'):
            streams(routes)
        if args.section in ('all', 'limits'):
            limits(routes)
        if args.section in ('all', 'mutation'):
            mutation(root, routes)
    frozen()
    print(json.dumps({'seed': hex(SEED), 'routes': len(routes), 'checks': dict(COUNTS),
                      'commands': harness.COUNTS['executions'], 'seconds': round(time.monotonic() - start, 2),
                      'frozen_fingerprint': FINGERPRINT}, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
