#!/usr/bin/env python3
"""Independent word-frequency acceptance; build target/debug/ail first.

Frozen R021 reference supplies request/summary expectations. Token vectors are
hand calculated or constructed without that tokenizer. Fourteen routes each
cover the app and token probe. Generated AIL executables do not use Python.
Every subprocess uses the existing bounded process harness with a 90s deadline.
Optional --artifact-dir must initially be empty and retains real I/O evidence.
"""
import argparse
import collections
import ctypes
import hashlib
import json
from pathlib import Path
import runpy
import shutil
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import ail_process_differential as harness

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / 'target/debug/ail'
PACKAGE = REPO / 'examples/word-frequency'
LIBRARY = REPO / 'examples/ail_std_words.ail'
RUNG = REPO / 'bench/R021-word-frequency'
FINGERPRINT = 'sha256:468062b540458705e855e78c52757374'
REFERENCE = runpy.run_path(str(RUNG / 'reference.py'))
DELIMITERS = ''.join(chr(n) for n in [*range(9, 14), *range(28, 33), 0x85, 0xa0,
                                    0x1680, *range(0x2000, 0x200b), 0x2028,
                                    0x2029, 0x202f, 0x205f, 0x3000])
LIMIT = 1_048_576
COUNTS = collections.Counter()
RECORDS = []
ROOT = None


def digest(data):
    return hashlib.sha256(data).hexdigest()


def frozen():
    hashed = hashlib.sha256()
    members = {}
    for path in sorted(RUNG.rglob('*')):
        if not path.is_file():
            continue
        relative = path.relative_to(RUNG)
        if relative.parts[0] == 'program.ail':
            continue
        raw = path.read_bytes()
        members[str(relative)] = digest(raw)
        hashed.update(str(relative).encode() + b'\0' + raw + b'\0')
    assert len(members) == 77, members.keys()
    assert 'sha256:' + hashed.hexdigest()[:32] == FINGERPRINT
    return members


def dependency_hashes():
    return {str(path.relative_to(REPO)): digest(path.read_bytes())
            for root in (LIBRARY, RUNG / 'program.ail')
            for path in sorted(root.rglob('*')) if path.is_file()}


def invoke(command, input=b''):
    command = list(map(str, command))
    index = len(RECORDS)
    start = time.monotonic()
    record = {'command': command, 'timeout_seconds': 90,
              'stdin_bytes': len(input), 'stdin_sha256': digest(input)}
    RECORDS.append(record)
    (ROOT / 'io' / f'{index:04d}.stdin').write_bytes(input)
    try:
        result = harness.invoke(command, input, timeout=90)
    except BaseException as error:
        record.update(status=None, exception=repr(error),
                      seconds=round(time.monotonic() - start, 4))
        raise
    record.update(status=result.returncode, seconds=round(time.monotonic() - start, 4),
                  stdout_bytes=len(result.stdout), stderr_bytes=len(result.stderr),
                  stdout_sha256=digest(result.stdout), stderr_sha256=digest(result.stderr))
    (ROOT / 'io' / f'{index:04d}.stdout').write_bytes(result.stdout)
    (ROOT / 'io' / f'{index:04d}.stderr').write_bytes(result.stderr)
    return result


def checked(command):
    result = invoke(command)
    assert result.returncode == 0 and result.stderr == b'', (command, result.stderr[-2000:])
    return result.stdout


def inspect(program):
    # Function bodies are actions in actual semantic IR. Check every instruction
    # and all Words.* helpers, rather than guessing a 'functions' JSON member.
    assert program['external_bindings'] == [] and program['capability_grants'] == []
    helpers = set()
    for action in program['actions']:
        pure = action['action'].startswith('Words.')
        if pure:
            helpers.add(action['action'])
            assert action['instructions'][0]['opcode'] == 'VALUE_FUNCTION'
        for instruction in action['instructions']:
            opcode = instruction['opcode']
            assert not opcode.startswith(('PROCESS_', 'HOST_'))
            assert opcode not in ('CALL_EXTERNAL', 'EVAL_EXPRESSION')
            if pure:
                assert not opcode.startswith(('FILE_', 'DIRECTORY_', 'STDIN_', 'STDOUT_', 'STDERR_'))
                assert opcode not in ('ACTION_BEGIN', 'EMIT_TRACE', 'OUTPUT_FIELD', 'RETURN_SUCCESS')
    assert {'Words.tokens', 'Words.pointer', 'Words.frequencies', 'Words.rank',
            'Words.summarize', 'Words.summarizeRequest'} == helpers
    COUNTS['ir_inspections'] += 1


def build(root, package, stem, actions, full=True):
    frontends = [('source', [package])]
    if full:
        core = root / (stem + '.core')
        core.write_bytes(checked([AIL, 'ail-core', package]))
        rendered = root / (stem + '-rendered')
        rendered.mkdir()
        (rendered / 'ail-package.md').write_text(
            '# AIL Package\nname: words-rendered\nversion: 0.1.0\nprofile: Application\n'
            'entry: spec.ail-spec.md\nfeatures: things, actions, functions, text, lists, json, stdio, expressions, control-flow, traces\n'
            'conformance: first-slice\n')
        (rendered / 'spec.ail-spec.md').write_bytes(checked([AIL, 'ail-spec', '--core-file', core]))
        frontends += [('core', ['--core-file', core]), ('rendered', [rendered])]
    routes = []
    for name, arguments in frontends:
        bytecode = root / f'{stem}-{name}.ailbc.json'
        bytecode.write_bytes(checked([AIL, 'ail-lower', *arguments]))
        inspect(json.loads(bytecode.read_bytes()))
        forms = [('direct', arguments, [AIL, 'ail-run', *arguments])]
        if full:
            ste = root / f'{stem}-{name}.ail-ste'
            ste.write_bytes(checked([AIL, 'ail-ste', *arguments]))
            forms.append(('ste', [ste], [AIL, 'ail-vm', ste]))
        if name == 'source' and full:
            forms.append(('bytecode', [bytecode], [AIL, 'ail-vm', bytecode]))
        for form, compile_arguments, vm in forms:
            binaries = {}
            for action in actions:
                binary = root / f'{stem}-{name}-{form}-{action}.elf'
                checked([AIL, 'ail-compile', *compile_arguments, '--action', action,
                         '--target', 'linux-x86_64-elf', '--out', binary])
                binaries[action] = binary
            routes += [(f'{stem}/{name}/{form}/VM', vm, binaries, False),
                       (f'{stem}/{name}/{form}/native', [], binaries, True)]
            print('built', stem, name, form, flush=True)
    return routes


def run(route, action, arguments=(), input=b'', stdio=False):
    _, vm, binaries, native = route
    command = [binaries[action]] if native else [*vm, '--action', action]
    return invoke(command + (['--stdio'] if stdio else []) + list(arguments), input)


def encoded(value):
    return REFERENCE['encode'](value).encode('utf-8')


def transport(value):
    # Literal UTF-8 keeps long Unicode words within Linux's single-argv bound.
    # json.dumps still escapes NUL so argv contains no raw NUL.
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'))


def selected(result, wanted, route, action):
    is_probe = action == 'Tokenize'
    event = 'Tokenized' if is_probe else 'Analyzed'
    report = f'action {action} started\n'.encode()
    if is_probe:
        report += b'parsed JSON data.text\n'
    report += f'trace {event}\n'.encode()
    if route[3]:
        assert result.returncode == 0 and result.stderr == report, (route[0], result.stderr[-1000:])
        assert result.stdout == b'out.result=' + encoded(wanted) + b'\n', (route[0], result.stdout[:700])
    else:
        prefix = b'ail-vm succeeded\n' if 'ail-vm' in list(map(str, route[1])) else b'ail-run succeeded\n'
        start = f'action {action} started'.encode()
        end = f' -> trace {event}\n'.encode()
        if is_probe:
            traces = [start + b' -> parsed JSON data.text' + end]
        elif wanted['ok']:
            traces = [start + b' -> recovered JSON-PATH into local.error' * wanted['unique'] + end]
        else:
            # The two catchable REQUEST paths have fixed, value-free reports.
            traces = [start + f' -> recovered {category} into local.error'.encode() + end
                      for category in ('JSON', 'JSON-PATH')]
        allowed = [prefix + b'out.result=' + encoded(wanted) + b'\ntrace=' + trace for trace in traces]
        assert result.returncode == 0 and result.stderr == b'' and result.stdout in allowed, (
            route[0], result.returncode, result.stdout[:700], allowed[0][:700], result.stderr[-1000:])
    COUNTS['checks'] += 1


def exact(result, wanted, context):
    assert result.returncode == 0 and result.stderr == b'' and result.stdout == wanted, (
        context, result.returncode, result.stdout[:700], wanted[:700], result.stderr[-1500:])
    COUNTS['checks'] += 1


def fatal(result, category, context, private=()):
    assert result.returncode > 0 and result.stdout == b'', (context, result.returncode, result.stdout[:700])
    # Only the one category and the backend's actual fixed diagnostic is allowed.
    reports = {
        'CAPABILITY': (b'AIL-RUNTIME-CAPABILITY: standard input is not granted\n',
                       b'action Count started\nAIL-RUNTIME-CAPABILITY: standard-stream grant is absent\n'),
        'TEXT': (b'AIL-RUNTIME-TEXT: standard input is not valid UTF-8\n',
                 b'AIL-RUNTIME-TEXT: missing or invalid UTF-8 text\n'),
        'TEXT-LIMIT': (b'AIL-RUNTIME-TEXT-LIMIT: standard input exceeds 1048576 bytes\n',
                       b'AIL-RUNTIME-TEXT-LIMIT: text size or allocation budget exceeded\n'),
        'LIST-LIMIT': (b'AIL-RUNTIME-LIST-LIMIT: encoded result exceeds 1048576 bytes\n',
                       b'AIL-RUNTIME-LIST-LIMIT: list size or allocation budget exceeded\n'),
    }
    if category in reports:
        assert result.stderr in reports[category], (context, result.stderr[-1500:])
    else:
        assert result.stderr.startswith(b'AIL-RUNTIME-' + category.encode() + b':'), (context, result.stderr[-1500:])
    assert all(secret not in result.stdout + result.stderr for secret in private), context
    COUNTS['checks'] += 1


def probe_package(root):
    package = root / 'token-probe'
    package.mkdir()
    (package / 'ail-package.md').write_text(
        '# AIL Package\nname: words-token-probe\nversion: 0.1.0\nprofile: Application\n'
        'entry: spec.ail-spec.md\nfeatures: things, actions, functions, text, lists, json, expressions, control-flow, traces\n'
        f'imports: {LIBRARY} compatible ^0.1 as Words\nconformance: first-slice\n')
    (package / 'spec.ail-spec.md').write_text('''The application WordsTokenProbe manages exact token tests.
A Data has:
- request: Text
- text: Text
An Out has:
- result: List<Text>
Action: Tokenize.
When Tokenize happens:
- the system parses JSON text data.request into data.text
- the system calls Words.tokens with text = data.text into out.result
- the system returns out.result
- the system records a trace event named Tokenized
''')
    return package


def smoke(routes, probe_routes):
    for route in routes:
        selected(run(route, 'Analyze', ['data.request={"text":"a a b"}']),
                 {'ok': True, 'words': 3, 'unique': 2,
                  'top': [{'word': 'a', 'count': 2}, {'word': 'b', 'count': 1}], 'error': ''}, route, 'Analyze')
    for route in probe_routes:
        selected(run(route, 'Tokenize', ['data.request="a a b"']), ['a', 'a', 'b'], route, 'Tokenize')
    COUNTS['report_smokes'] += len(routes) + len(probe_routes)


def fixed(routes):
    cases = []
    for path in sorted((RUNG / 'cases').glob('*.args')):
        state = dict(line.split('=', 1) for line in path.read_text().splitlines()
                     if line.strip() and not line.lstrip().startswith('#'))
        text = state.get('data.request', '')
        cases.append((path.stem, text, REFERENCE['evaluate_request'](text)))
    assert len(cases) == 75
    (ROOT / 'frozen-expectations.json').write_text(json.dumps(
        [{'case': label, 'expected': encoded(wanted).decode()} for label, _, wanted in cases],
        ensure_ascii=False, indent=2) + '\n')
    for route in routes:
        for label, text, wanted in cases:
            selected(run(route, 'Analyze', ['data.request=' + text]), wanted, route, 'Analyze')
            COUNTS['frozen_requests'] += 1
        print('frozen', route[0], len(cases), flush=True)


def token_vectors():
    cases = [('empty', '', []), ('nonempty', 'word', ['word']),
             ('adjacent', ' \ta\r\n\u00a0b\u3000 ', ['a', 'b']),
             ('all-delimiters-empty', DELIMITERS, []),
             ('all-delimiters-between', 'a' + DELIMITERS + 'b', ['a', 'b']),
             ('case-punctuation', 'A a a, "a" a. 1 01', ['A', 'a', 'a,', '"a"', 'a.', '1', '01']),
             ('Unicode', 'é e\u0301 🙂 \U00010000 \ue000', ['é', 'e\u0301', '🙂', '\U00010000', '\ue000']),
             ('NUL-BOM-zero-width', '\x00 x\ufeffy x\u200by', ['\x00', 'x\ufeffy', 'x\u200by']),
             ('pointer-controls', '/ ~ /a~b a/b \x01\x08\x0e\x1b\x7f\x80\x9f \\ "',
              ['/', '~', '/a~b', 'a/b', '\x01\x08\x0e\x1b\x7f\x80\x9f', '\\', '"'])]
    for separator in DELIMITERS:
        cases.append((f'delimiter-U+{ord(separator):04X}', separator + 'a' + separator * 2 + 'b' + separator,
                      ['a', 'b']))
    cases += [('ASCII32k', 'x' * 32768, ['x' * 32768]),
              ('Unicode32k', 'é' * 32768, ['é' * 32768]),
              ('separators32k', (DELIMITERS * 1130)[:32768], []),
              ('1024-words', 'w ' * 1024, ['w'] * 1024)]
    return cases


def direct(routes):
    vectors = token_vectors()
    (ROOT / 'token-expectations.json').write_text(json.dumps(
        [{'case': label, 'input': text, 'expected': words} for label, text, words in vectors],
        ensure_ascii=False, indent=2) + '\n')
    for route in routes:
        for label, text, words in vectors:
            selected(run(route, 'Tokenize', ['data.request=' + transport(text)]), words, route, 'Tokenize')
            COUNTS['direct_tokens'] += 1
        print('tokens', route[0], len(vectors), flush=True)


def io(routes):
    values = [('empty', ''), ('all-whitespace', DELIMITERS),
              ('Unicode-NUL', 'é\x00 é\x00 🙂 e\u0301 \ufeffx x\u200by'),
              ('multiple-lines', 'a a\nb\r\nc'), ('no-final-LF', 'b a a'),
              ('JSON-looking-raw', '{"text":"a a"}'),
              ('ASCII32k', 'x' * 32768), ('Unicode32k', 'é' * 32768),
              ('separators32k', (DELIMITERS * 1130)[:32768]),
              ('1024-repeated', 'w ' * 1024),
              ('128-distinct-ties', ' '.join(f'w{i:03d}' for i in range(127, -1, -1)) + ' w003 w003 w100'),
              ('one-MiB-spaces', ' ' * LIMIT),
              ('sample', (PACKAGE / 'input.txt').read_text())]
    expected = {label: encoded(REFERENCE['summarize'](text)) + b'\n' for label, text in values}
    assert (PACKAGE / 'expected.json').read_bytes() == expected['sample']
    for route in routes:
        for label, text in values:
            exact(run(route, 'Count', input=text.encode(), stdio=True), expected[label], (route[0], label))
            COUNTS['raw_stdin_successes'] += 1
        private = (b'PRIVATE-WORD-INPUT',)
        fatal(run(route, 'Count', input=private[0]), 'CAPABILITY', (route[0], 'missing-stdio'), private)
        fatal(run(route, 'Count', input=b'\xff' + private[0], stdio=True), 'TEXT', (route[0], 'invalid-UTF8'), private)
        fatal(run(route, 'Count', input=private[0] + b'x' * (LIMIT + 1 - len(private[0])), stdio=True),
              'TEXT-LIMIT', (route[0], 'one-MiB-plus-one'), private)
        # Valid Text, but its canonical List<Text> representation crosses 1MiB.
        fatal(run(route, 'Count', input=b'\0' * 174763, stdio=True), 'LIST-LIMIT', (route[0], 'escaped-list-bound'))
        deep = '{"text":"PRIVATE-WORD-INPUT","unused":' + '[' * 65 + '0' + ']' * 65 + '}'
        fatal(run(route, 'Analyze', ['data.request=' + deep], stdio=True), 'JSON-LIMIT', (route[0], 'JSON-depth'), private)
        COUNTS['fatal_bounds'] += 5
        print('raw I/O and fatal bounds', route[0], flush=True)


def mutation(root):
    library = root / 'mutated-library.ail'
    shutil.copytree(LIBRARY, library)
    source = library / 'spec.ail-spec.md'
    operation = '- the function sorts list local.keys in ascending order into local.keys'
    original = source.read_text()
    assert original.count(operation) == 1
    source.write_text(original.replace(operation, operation.replace('ascending', 'descending')))
    package = root / 'mutated-app'
    shutil.copytree(PACKAGE, package)
    manifest = package / 'ail-package.md'
    manifest.write_text(manifest.read_text().replace('../ail_std_words.ail', str(library)))
    request = transport({'text': 'z a z a b c d e f'})
    wanted = REFERENCE['evaluate_request'](request)
    for route in build(root, package, 'mutation', ['Analyze'], full=False):
        result = run(route, 'Analyze', ['data.request=' + request])
        # Successful execution and correct fixed reports must pass independently;
        # only a real wrong ranking/count result counts as a detected mutation.
        lines = result.stdout.split(b'\n')
        actual = next(line[len(b'out.result='):] for line in lines if line.startswith(b'out.result='))
        decoded = json.loads(actual)
        selected(result, decoded, route, 'Analyze')
        assert actual != encoded(wanted), 'independent oracle accepted deliberate ranking mutation'
        assert decoded['ok'] is True and decoded['words'] == wanted['words'] and decoded['unique'] == wanted['unique']
        (root / ('mutation-' + ('native' if route[3] else 'VM') + '.json')).write_text(json.dumps(
            {'operation': operation, 'replacement': operation.replace('ascending', 'descending'),
             'status': result.returncode, 'expected': encoded(wanted).decode(), 'actual': actual.decode(),
             'independent_mismatch': True}, ensure_ascii=False, indent=2) + '\n')
        COUNTS['semantic_mutations_detected'] += 1
    print('actual wrong ranking rejected on VM/native at successful status', flush=True)


def main():
    global ROOT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact-dir', type=Path, help='retain artifacts and actual command/I/O records')
    args = parser.parse_args()
    before = frozen()
    dependencies = dependency_hashes()
    compiler = digest(AIL.read_bytes())
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0
    temporary = None
    if args.artifact_dir:
        ROOT = args.artifact_dir.resolve()
        ROOT.mkdir(parents=True, exist_ok=True)
        assert not any(ROOT.iterdir()), 'artifact directory must initially be empty'
    else:
        temporary = tempfile.TemporaryDirectory(prefix='ail-words-')
        ROOT = Path(temporary.name)
    (ROOT / 'io').mkdir()
    started = time.monotonic()
    try:
        # Frontload fixed report behavior on both backends before the full matrix.
        probe = probe_package(ROOT)
        early = build(ROOT, PACKAGE, 'smoke-app', ['Analyze'], full=False)
        early_probe = build(ROOT, probe, 'smoke-probe', ['Tokenize'], full=False)
        smoke(early, early_probe)
        routes = build(ROOT, PACKAGE, 'app', ['Analyze', 'Count'])
        probe_routes = build(ROOT, probe, 'probe', ['Tokenize'])
        assert len(routes) == len(probe_routes) == 14
        fixed(routes)
        direct(probe_routes)
        io(routes)
        mutation(ROOT)
        assert frozen() == before and dependency_hashes() == dependencies
        assert digest(AIL.read_bytes()) == compiler
        summary = {'status': 'PASS', 'counts': dict(COUNTS), 'app_routes': len(routes),
                   'token_routes': len(probe_routes), 'commands': len(RECORDS),
                   'seconds': round(time.monotonic() - started, 4), 'frozen': FINGERPRINT,
                   'oracle_members': before, 'dependencies': dependencies, 'compiler_sha256': compiler}
        (ROOT / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
        print(json.dumps({key: value for key, value in summary.items()
                          if key not in ('oracle_members', 'dependencies')}, sort_keys=True), flush=True)
    except BaseException as error:
        (ROOT / 'failure.json').write_text(json.dumps(
            {'exception': repr(error), 'counts': dict(COUNTS), 'commands': len(RECORDS)}, indent=2) + '\n')
        raise
    finally:
        (ROOT / 'commands.json').write_text(json.dumps(RECORDS, ensure_ascii=False, indent=2) + '\n')
        if temporary:
            temporary.cleanup()


if __name__ == '__main__':
    main()
