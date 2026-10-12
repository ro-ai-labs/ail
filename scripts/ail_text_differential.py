#!/usr/bin/env python3
"""Compare Unicode text execution with Python. Run cargo build first.

Checks VM/native results, strict native UTF-8, and saved-Core secret rules.
Uses fixed-seed generated inputs; never modifies the frozen capability ladder.
"""
import json, pathlib, random, subprocess, tempfile
REPO = pathlib.Path(__file__).resolve().parent.parent
AIL = str(REPO / 'target/debug/ail')
def run(args, **kw):
    return subprocess.run(args, cwd=REPO, capture_output=True, timeout=15, **kw)
def op(code, **operands):
    return dict(opcode=code, operands=operands)
with tempfile.TemporaryDirectory(prefix='ail-independent-text-review-') as scratch:
    root = pathlib.Path(scratch)
    exported = run([AIL, 'ail-core', 'examples/unicode-text'], text=True)
    assert exported.returncode == 0, exported.stderr
    for field, expected in [('Input.first', 'AIL005'), ('Input.second', 'AIL005'), ('Input.start', 'AIL005'), ('Input.end', 'AIL005'), ('Output.part', 'AIL002')]:
        lines = []
        for line in exported.stdout.splitlines():
            if line.startswith(('edge reads ', 'edge writes ')):
                continue
            if line.startswith('node Field ' + field + ' : '):
                line = line.replace('secret=false', 'secret=true')
            lines.append(line)
        source = root / 'secret.core'
        source.write_text('\n'.join(lines) + '\n')
        checked = run([AIL, 'ail-lower', '--core-file', str(source)], text=True)
        assert checked.returncode != 0 and expected in checked.stdout + checked.stderr, (field, checked.stdout, checked.stderr)
    print('PASS: five saved-Core secret source, needle, bound, and destination probes without inferred access edges')
    steps = [op('TEXT_JOIN', key='out.joined', left='input.value', right='input.needle'), op('TEXT_LENGTH', key='out.count', source='input.value'), op('TEXT_SLICE', key='out.slice', source='input.value', start='f:input.start', end='f:input.end'), op('TEXT_FIND', key='out.index', source='input.value', needle='input.needle')]
    steps += [op('OUTPUT_FIELD', key='out.' + key) for key in ['joined', 'count', 'slice', 'index']] + [op('RETURN_SUCCESS')]
    program = dict(kind='AIL-Bytecode', package='independent-review', version='0.1', profile='Application', capability_grants=[], target_support={}, external_bindings=[], failures=[], actions=[dict(action='Run', instructions=steps)])
    source = root / 'program.json'
    source.write_text(json.dumps(program))
    binary = root / 'program.elf'
    compiled = run([AIL, 'ail-compile', str(source), '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', str(binary)], text=True)
    assert compiled.returncode == 0, (compiled.stdout, compiled.stderr)
    random.seed(765)  # Stable regression cases.
    chars = ['a', 'b', 'é', '🙂', '\u0301', '\u0085', '\U0010ffff', '\ud7ff', '\ue000', '=', '\n', '\r', '\t']
    cases = []
    for index in range(600):
        haystack = ''.join(random.choices(chars, k=random.randrange(60)))
        needle = ''.join(random.choices(chars, k=random.randrange(8))) if index % 2 else haystack[random.randrange(len(haystack) + 1):][:random.randrange(7)]
        start = random.choice([-2**63, 2**63 - 1, random.randrange(-80, 80)])
        end = random.choice([-2**63, 2**63 - 1, random.randrange(-80, 80)])
        cases.append((haystack, needle, start, end))
    for haystack, needle in [('ababababac', 'ababac'), ('ééé🙂éééé🙂', 'éééé🙂'), ('a'*60000, 'a'*29999+'b'), ('a'*59999+'b', 'a'*29999+'b'), ('ababa'*2000+'c', 'ababa'*999+'c')]:
        cases.append((haystack, needle, 1, -1))
    for index, (haystack, needle, start, end) in enumerate(cases):
        args = ['input.value=' + haystack, 'input.needle=' + needle, 'input.start=' + str(start), 'input.end=' + str(end)]
        result = run([str(binary)] + args)
        expected = f'out.count={len(haystack)}\nout.index={haystack.find(needle)}\nout.joined={haystack}{needle}\nout.slice={haystack[start:end]}\n'.encode()
        assert result.returncode == 0 and result.stdout == expected, (index, result.returncode, result.stdout, expected, result.stderr)
        vm = run([AIL, 'ail-vm', str(source), '--action', 'Run'] + args)
        assert vm.returncode == 0 and vm.stdout.startswith(b'ail-vm succeeded\n' + expected + b'trace='), (index, vm.returncode, vm.stdout, expected, vm.stderr)
    print(f'PASS: {len(cases)} VM/native/Python Unicode text differentials, including repetitive search inputs')
    for value in [b'\xff', b'\xc0\x80', b'\xed\xa0\x80', b'\xf4\x90\x80\x80', b'\xe0\x80\x80', b'\xf0\x80\x80\x80', b'\xc2', b'\xe2\x82', b'\xf0\x9f\x92']:
        result = run([bytes(binary), b'input.value=' + value, b'input.needle=', b'input.start=0', b'input.end=1'])
        assert result.returncode != 0 and not result.stdout and b'AIL-RUNTIME-TEXT' in result.stderr, (value, result)
    print('PASS: nine malformed UTF-8 forms rejected without result stdout')
