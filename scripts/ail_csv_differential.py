#!/usr/bin/env python3
"""Independent CSV, real I/O and artifact-route acceptance; cargo build first.

Frozen R022 reference.py supplies aggregation expectations. Direct parse vectors
are hand calculated. The generated programs use no Python or host CSV parser.
Every process has a deadline; failures retain bounded evidence in --artifact-dir.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import re
import runpy
import shutil
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import ail_process_differential as harness

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / 'target/debug/ail'
PACKAGE = REPO / 'examples/csv-aggregate'
LIBRARY = REPO / 'examples/ail_std_csv.ail'
RUNG = REPO / 'bench/R022-csv-aggregate'
FINGERPRINT = 'sha256:421457ed6f6825d12c9d9bb1c268ff25'
REFERENCE = runpy.run_path(str(RUNG / 'reference.py'))
TRIM = ''.join(chr(n) for n in [*range(9, 14), *range(28, 33), 0x85, 0xa0,
                              0x1680, *range(0x2000, 0x200b), 0x2028,
                              0x2029, 0x202f, 0x205f, 0x3000])
STATE = re.compile(rb'^(?!trace=)[a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*=')
COUNTS = collections.Counter()
RECORDS = []


def frozen():
    digest = hashlib.sha256()
    count = 0
    for path in sorted(RUNG.rglob('*')):
        if not path.is_file():
            continue
        relative = path.relative_to(RUNG)
        if relative.parts[0] == 'program.ail':
            continue
        count += 1
        digest.update(str(relative).encode() + b'\0' + path.read_bytes() + b'\0')
    assert 'sha256:' + digest.hexdigest()[:32] == FINGERPRINT
    assert count == 131


def invoke(command, input=b''):
    start = time.monotonic()
    result = harness.invoke(command, input, timeout=90)
    RECORDS.append({'command': list(map(str, command)), 'status': result.returncode,
                    'seconds': round(time.monotonic() - start, 4),
                    'stdout_bytes': len(result.stdout), 'stderr_bytes': len(result.stderr),
                    'stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
                    'stderr_sha256': hashlib.sha256(result.stderr).hexdigest(),
                    'stderr_tail': result.stderr[-1500:].decode('utf-8', 'replace')
                    if result.returncode else ''})
    return result


def checked(command):
    result = invoke(command)
    assert result.returncode == 0, (command[:6], result.stderr[-2000:])
    return result.stdout


def inspect(program):
    assert program['external_bindings'] == [] and program['capability_grants'] == []
    # Walk both ordinary actions and function IR; effect checks are semantic,
    # not merely a search for forbidden words in source comments.
    def walk(value, pure=False):
        if isinstance(value, dict):
            name = str(value.get('action', value.get('name', value.get('function', ''))))
            pure = pure or name.startswith('Csv.')
            opcode = value.get('opcode', '')
            assert not opcode.startswith(('PROCESS_', 'HOST_', 'CSV_'))
            assert opcode not in ('CALL_EXTERNAL', 'EVAL_EXPRESSION')
            if pure:
                assert not opcode.startswith(('FILE_', 'DIRECTORY_', 'STDIN_', 'STDOUT_', 'STDERR_'))
            for child in value.values():
                walk(child, pure)
        elif isinstance(value, list):
            for child in value:
                walk(child, pure)
    walk(program)


def build(root, package, stem, actions, full=True):
    frontends = [('source', [package])]
    if full:
        core = root / (stem + '.core')
        core.write_bytes(checked([AIL, 'ail-core', package]))
        rendered = root / (stem + '-rendered')
        rendered.mkdir()
        (rendered / 'ail-package.md').write_text(
            '# AIL Package\nname: csv-rendered\nversion: 0.1.0\nprofile: Application\n'
            'entry: spec.ail-spec.md\nfeatures: things, actions, functions, text, lists, json, files, expressions, control-flow, traces\n'
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
        # One separately compiled and executed saved bytecode route is enough:
        # adding it for every frontend would repeat the same flattened program.
        if name == 'source':
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


def exact(result, wanted, context):
    assert result.returncode == 0 and result.stderr == b'' and result.stdout == wanted, (
        context, result.returncode, result.stdout[:700], wanted[:700], result.stderr[-1500:])
    COUNTS['checks'] += 1


def selected(result, wanted, context):
    # Non-stdio native execution reports the action/trace on stderr; the VM
    # keeps ordinary trace reports on stdout. Stream/report use exact(), which
    # still requires empty stderr in --stdio mode.
    native = context[0].endswith('/native')
    if context[0].startswith('parser/'):
        trace = b'action Parse started\nparsed JSON data.text\n'
        if wanted == b'{"error":"TEXT"}':
            trace += b'assigned literal out.result\n'
        trace += b'trace Parsed\n'
    else:
        trace = b'action Analyze started\ntrace Analyzed\n'
    wanted_stderr = trace if native else b''
    assert result.returncode == 0 and result.stderr == wanted_stderr, (context, result.stderr[-1500:])
    actual = [line for line in result.stdout.split(b'\n') if STATE.match(line)]
    assert actual == [b'out.result=' + wanted], (context, [x[:700] for x in actual], wanted[:700])
    COUNTS['checks'] += 1


def fatal(result, category, context, private=()):
    assert result.returncode > 0 and result.stdout == b'', (context, result.returncode, result.stdout[:700])
    if category == 'fuel':
        assert b'step limit' in result.stderr or b'instruction-step' in result.stderr, (context, result.stderr[-1500:])
    else:
        assert re.findall(rb'AIL-RUNTIME-([A-Z-]+):', result.stderr) == [category.encode()], (
            context, result.stderr[-1500:])
    assert all(secret not in result.stdout + result.stderr for secret in private), context
    COUNTS['checks'] += 1


def encoded(value):
    return REFERENCE['encode'](value).encode('utf-8')


def expected(request):
    return encoded(REFERENCE['evaluate_request'](request))


def request(csv):
    # JSON transport escapes NUL: no raw NUL is ever placed in argv.
    return json.dumps({'csv': csv}, ensure_ascii=False, separators=(',', ':'))


def fixed(routes):
    cases = []
    for path in sorted((RUNG / 'cases').glob('*.args')):
        state = dict(line.split('=', 1) for line in path.read_text().splitlines()
                     if line.strip() and not line.lstrip().startswith('#'))
        text = state.get('data.request', '')
        cases.append((path.stem, text, expected(text)))
    assert len(cases) == 129
    for route in routes:
        for label, text, wanted in cases:
            selected(run(route, 'Analyze', ['data.request=' + text]), wanted, (route[0], label))
            COUNTS['frozen'] += 1
        print('frozen', route[0], len(cases), flush=True)


def parser_package(root):
    package = root / 'parser'
    package.mkdir()
    (package / 'ail-package.md').write_text(
        '# AIL Package\nname: csv-parser-test\nversion: 0.1.0\nprofile: Application\n'
        'entry: spec.ail-spec.md\nfeatures: things, actions, functions, json, text, control-flow, traces\n'
        f'imports: {LIBRARY} compatible ^0.1 as Csv\nconformance: first-slice\n')
    (package / 'spec.ail-spec.md').write_text('''The application CsvParserTest manages parser tests.
A Data has:
- request: Text
- text: Text
- error: Text
An Out has:
- result: Json
Action: Parse.
When Parse happens:
- the system parses JSON text data.request into data.text
- try:
- the system calls Csv.parse with text = data.text into out.result
- recover TEXT into data.error:
- the system assigns {"error":"TEXT"} to out.result
- end try
- the system returns out.result
- the system records a trace event named Parsed
''')
    return package


def parser_vectors():
    # Literal rows are independent of both the AIL parser and Python csv.reader.
    cases = [('empty', '', []), ('LF', '\n', [[]]), ('CR', '\r', [[]]),
             ('CRLF', '\r\n', [[]]), ('blank-two', '\n\n', [[], []]),
             ('quoted-empty', '""', [['']]), ('comma', ',', [['', '']]),
             ('trailing', 'a,', [['a', '']]), ('adjacent', ',,', [['', '', '']]),
             ('trailing-record', 'a\n', [['a']]),
             ('mixed', 'a\r\nb\rc\n', [['a'], ['b'], ['c']]),
             ('embedded', '"a\r\nb\rc\n"', [['a\r\nb\rc\n']]),
             ('escape', '"a""b",c', [['a"b', 'c']]),
             ('literal', '\x00é🙂 \t', [['\x00é🙂 \t']]),
             ('bom', '\ufeffx', [['\ufeffx']]),
             ('bare-quote', 'a"b', None), ('leading-space-quote', ' "a"', None),
             ('tail-space', '"a" ', None), ('tail-junk', '"a"x', None),
             ('unclosed', '"a', None), ('odd', '"""', None),
             ('late-malformed', 'a,1\r\n' * 384 + '"', None)]
    for n in (510, 511, 512, 513, 1023, 1024):
        value = 'x' * n
        cases += [(f'quote-pair-{n}', '"' + value + '""z"', [[value + '"z']]),
                  (f'quoted-CRLF-{n}', '"' + value + '\r\nz"', [[value + '\r\nz']]),
                  (f'CRLF-{n}', value + '\r\nx', [[value], ['x']])]
    cases += [('ASCII32k', 'x' * 32768, [['x' * 32768]]),
              ('Unicode32k', 'é' * 32768, [['é' * 32768]]),
              ('1024-records', 'a\n' * 1024, [['a']] * 1024)]
    return cases


def direct(routes):
    vectors = parser_vectors()
    for route in routes:
        for label, csv, rows in vectors:
            wanted = {'error': 'TEXT'} if rows is None else rows
            selected(run(route, 'Parse', ['data.request=' + json.dumps(csv, ensure_ascii=False)]),
                     encoded(wanted), (route[0], label))
            COUNTS['parser'] += 1
        print('parser', route[0], len(vectors), flush=True)


def io(routes, root):
    source = root / 'PRIVATE-CSV-PATH.csv'
    report = root / 'PRIVATE-REPORT-PATH.json'
    private = [str(source).encode(), str(report).encode(), b'PRIVATE-INPUT-SENTINEL']
    good = 'category,amount\na,7\na,2\n'
    mixed = ('\n \t\r\n\u00a0\n' + request(good) + '\r\n' + '{broken}\n' +
             request('category,amount\n"') + '\n' + request('wrong,header') + '\n' +
             request('category,amount\na,1,x') + '\n' + request('category,amount\na,+1') + '\n' +
             request('category,amount\na,9223372036854775807\na,1') + '\n' +
             '  {"csv":"category,amount","unused":"PRIVATE-INPUT-SENTINEL"} \t\r\n' +
             '\u00a0' + request(good) + '\u00a0\n' + request(good))
    wanted = b''.join(expected(line) + b'\n' for line in mixed.split('\n') if line.strip(TRIM))
    failures = [('CSV', 'category,amount\nPRIVATE-INPUT-SENTINEL,"'),
                ('HEADER', 'wrong,header\nPRIVATE-INPUT-SENTINEL,1'),
                ('ROW', 'category,amount\nPRIVATE-INPUT-SENTINEL,1,x'),
                ('AMOUNT', 'category,amount\nPRIVATE-INPUT-SENTINEL,+1'),
                ('OVERFLOW', 'category,amount\nPRIVATE-INPUT-SENTINEL,9223372036854775807\nPRIVATE-INPUT-SENTINEL,1')]
    sentinel = b'PREEXISTING-REPORT\x00\r\n'
    for route in routes:
        context = route[0]
        result = run(route, 'Stream', input=mixed.encode(), stdio=True)
        exact(result, wanted, (context, 'mixed-stream'))
        assert all(secret not in result.stdout + result.stderr for secret in private)
        exact(run(route, 'Stream', input=b' \t\r\n', stdio=True), b'', (context, 'blank-stream'))
        fatal(run(route, 'Stream', input=request(good).encode()), 'CAPABILITY', (context, 'missing-stdio'))
        fatal(run(route, 'Stream', input=b'\xff', stdio=True), 'TEXT', (context, 'invalid-UTF8'))
        fatal(run(route, 'Stream', input=b'x' * 1048577, stdio=True), 'TEXT-LIMIT', (context, 'stdin-bound'))
        source.write_text(good)
        grants = ['--read-file=input=' + str(source), '--write-file=report=' + str(report)]
        exact(run(route, 'WriteReport', grants, stdio=True), b'', (context, 'report-success'))
        assert report.read_bytes() == encoded(REFERENCE['summarize'](good)) + b'\n'
        for code, csv in failures:
            assert REFERENCE['summarize'](csv)['error'] == code
            source.write_text(csv)
            report.write_bytes(sentinel)
            fatal(run(route, 'WriteReport', grants, stdio=True), 'VALUE', (context, code), private)
            assert report.read_bytes() == sentinel
            COUNTS['preservation'] += 1
        source.write_text(good)
        wrong = [[], grants[:1], grants[1:],
                 ['--write-file=input=' + str(source), grants[1]],
                 [grants[0], '--read-file=report=' + str(report)]]
        for index, bindings in enumerate(wrong):
            report.write_bytes(sentinel)
            fatal(run(route, 'WriteReport', bindings, stdio=True), 'CAPABILITY', (context, 'grant', index), private)
            assert report.read_bytes() == sentinel
            COUNTS['preservation'] += 1
        source.write_bytes(b'\xffPRIVATE-INPUT-SENTINEL')
        report.write_bytes(sentinel)
        fatal(run(route, 'WriteReport', grants, stdio=True), 'TEXT', (context, 'file-UTF8'), private)
        assert report.read_bytes() == sentinel
        COUNTS['preservation'] += 1
        # Whole read completes before write, so deliberate same-file bindings work.
        source.write_text(good)
        exact(run(route, 'WriteReport', [grants[0], '--write-file=report=' + str(source)], stdio=True),
              b'', (context, 'alias'))
        assert source.read_bytes() == encoded(REFERENCE['summarize'](good)) + b'\n'
        deep = '{"csv":"category,amount","unused":' + '[' * 65 + '0' + ']' * 65 + '}'
        fatal(run(route, 'Analyze', ['data.request=' + deep], stdio=True), 'JSON-LIMIT', (context, 'depth'))
        dense = request('category,amount\n"' + '""' * 8192 + '",1')
        fatal(run(route, 'Analyze', ['data.request=' + dense], stdio=True), 'fuel', (context, 'quote-fuel'))
        print('I/O and fatal bounds', context, flush=True)


def mutation(root):
    library = root / 'mutated-library.ail'
    shutil.copytree(LIBRARY, library)
    source = library / 'spec.ail-spec.md'
    original = source.read_text()
    operation = '- the function sorts list local.keys in ascending order into local.keys'
    assert original.count(operation) == 1
    source.write_text(original.replace(operation, operation.replace('ascending', 'descending')))
    package = root / 'mutated-app'
    shutil.copytree(PACKAGE, package)
    manifest = package / 'ail-package.md'
    manifest.write_text(manifest.read_text().replace('../ail_std_csv.ail', str(library)))
    text = request('category,amount\nz,1\na,2')
    wanted = expected(text)
    routes = build(root, package, 'mutation', ['Analyze'], full=False)
    for route in routes[:2]:
        result = run(route, 'Analyze', ['data.request=' + text])
        try:
            selected(result, wanted, (route[0], 'deliberate-sort-mutation'))
        except AssertionError as error:
            (root / ('mutation-' + ('native' if route[3] else 'VM') + '.json')).write_text(
                json.dumps({'operation': operation, 'expected': wanted.decode(),
                            'actual': result.stdout.decode()[-2000:], 'status': result.returncode,
                            'mismatch': str(error)[:3000]}, indent=2) + '\n')
            wanted_stderr = b'action Analyze started\ntrace Analyzed\n' if route[3] else b''
            assert result.returncode == 0 and result.stderr == wanted_stderr
            COUNTS['mutation_detected'] += 1
        else:
            raise AssertionError('independent oracle accepted deliberate sort mutation')
    print('mutation detected by independent expectations on VM/native', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact-dir', type=Path, help='retain artifacts and bounded execution records')
    parser.add_argument('--io-only', action='store_true', help='focused source/bytecode I/O development gate')
    args = parser.parse_args()
    frozen()
    assert AIL.is_file(), 'cargo build first'
    temporary = None
    if args.artifact_dir:
        root = args.artifact_dir.resolve()
        root.mkdir(parents=True, exist_ok=True)
        assert not any(root.iterdir()), 'artifact directory must be empty'
    else:
        temporary = tempfile.TemporaryDirectory(prefix='ail-csv-')
        root = Path(temporary.name)
    start = time.monotonic()
    try:
        routes = build(root, PACKAGE, 'app', ['Analyze', 'Stream', 'WriteReport'], not args.io_only)
        if not args.io_only:
            fixed(routes)
            direct(build(root, parser_package(root), 'parser', ['Parse']))
        io(routes, root)
        if not args.io_only:
            mutation(root)
        frozen()
        result = {'status': 'PASS', 'counts': dict(COUNTS), 'routes': len(routes),
                  'commands': len(RECORDS), 'seconds': round(time.monotonic() - start, 4),
                  'frozen': FINGERPRINT}
        (root / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, sort_keys=True), flush=True)
    finally:
        (root / 'commands.json').write_text(json.dumps(RECORDS, indent=2) + '\n')
        if temporary:
            temporary.cleanup()


if __name__ == '__main__':
    main()
