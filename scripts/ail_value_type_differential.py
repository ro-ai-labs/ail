#!/usr/bin/env python3
"""Independent recursive variant acceptance through the public AIL pipelines.

Run cargo build first. Python owns the expected trees, transport and failures;
no compiler/runtime module supplies expected values. Only subprocess deadlines
and descendant cleanup come from the existing process test harness. Frozen R019
inputs cannot be edited to make this suite pass.
"""
import argparse
import collections
import copy
import ctypes
import hashlib
import json
import math
from pathlib import Path
import random
import re
import runpy
import struct
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import ail_process_differential as harness

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / 'target/debug/ail'
RUNG = REPO / 'bench/R019-binary-tree'
REFERENCE = runpy.run_path(str(RUNG / 'reference.py'))
FINGERPRINT = 'sha256:2366607a2a2761edf25a49af0567ef69'
SEED = 0xA11D019
TRANSPORT_LIMIT = 8 * 1024 * 1024
LEAF_LIMIT = 1024 * 1024
COUNTS = collections.Counter()
STATE = re.compile(r'^(?!trace=)[a-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)*=')
LEAVES = {'integer': 'Int', 'floating': 'Float', 'boolean': 'Bool',
          'text': 'Text', 'bytes': 'Bytes', 'integers': 'List<Int>',
          'floats': 'List<Float>', 'texts': 'List<Text>', 'document': 'Json'}


def frozen():
    digest = hashlib.sha256()
    for path in sorted(RUNG.rglob('*')):
        if not path.is_file() or path.relative_to(RUNG).parts[0] == 'program.ail':
            continue
        digest.update(str(path.relative_to(RUNG)).encode() + b'\0' + path.read_bytes() + b'\0')
    assert 'sha256:' + digest.hexdigest()[:32] == FINGERPRINT, 'frozen R019 measurement changed'


def checked(command):
    result = harness.invoke(command, timeout=60)
    assert result.returncode == 0, (list(map(str, command[:7])), result.stdout[-1500:], result.stderr[-2500:])
    return result.stdout


def package(root, name, source, imports=''):
    directory = root / name
    directory.mkdir()
    (directory / 'ail-package.md').write_text(
        f'# AIL Package\nname: {name}\nversion: 0.1.0\nprofile: Application\n'
        'entry: spec.ail-spec.md\nfeatures: things, types, actions, functions, expressions, control-flow, bytes, json\n'
        + (f'imports: {imports}\n' if imports else '') + 'conformance: first-slice\n')
    (directory / 'spec.ail-spec.md').write_text(source)
    return directory


def build(root, source, stem, actions=('Run',), full=True):
    frontends = [('source', [source])]
    if full:
        core = root / f'{stem}.core'
        core.write_bytes(checked([AIL, 'ail-core', source]))
        rendered = package(root, stem + '-rendered', checked([AIL, 'ail-spec', '--core-file', core]).decode())
        frontends += [('core', ['--core-file', core]), ('rendered', [rendered])]
    result = []
    canonical_program = None
    for frontend, arguments in frontends:
        bytecode = root / f'{stem}-{frontend}.json'
        bytecode.write_bytes(checked([AIL, 'ail-lower', *arguments]))
        program = json.loads(bytecode.read_bytes())
        assert program['external_bindings'] == [] and program['capability_grants'] == []
        assert program.get('type_schemas'), (stem, 'missing executable schema')
        # Concrete schemas and behavior must survive Core and rendered routes.
        if canonical_program is None:
            canonical_program = program['type_schemas']
        assert program['type_schemas'] == canonical_program, (stem, frontend, 'schema round trip')
        forms = [('direct', bytecode, arguments)]
        if full:
            ste = root / f'{stem}-{frontend}.ail-ste'
            ste.write_bytes(checked([AIL, 'ail-ste', *arguments]))
            forms.append(('ste', ste, [ste]))
        for form, artifact, compiler_arguments in forms:
            binaries = {}
            for action in actions:
                binary = root / f'{stem}-{frontend}-{form}-{action}.elf'
                checked([AIL, 'ail-compile', *compiler_arguments, '--action', action,
                         '--target', 'linux-x86_64-elf', '--out', binary])
                binaries[action] = binary
            for backend in ('VM', 'native'):
                result.append((f'{stem}/{frontend}/{form}/{backend}', artifact, binaries, backend))
        print(f'built {stem}/{frontend}', flush=True)
    assert len(result) == (12 if full else 2)
    return result


def run(route, action='Run', arguments=(), **kwargs):
    _, artifact, binaries, backend = route
    command = [AIL, 'ail-vm', artifact, '--action', action] if backend == 'VM' else [binaries[action]]
    return harness.invoke(command + list(arguments), timeout=60, **kwargs)


def fields(result, context):
    assert result.returncode == 0 and not categories(result.stderr), (
        context, result.returncode, result.stdout[:500], result.stderr[-2000:])
    # U+0085 is valid Text; Python splitlines() would silently split its value.
    lines = [line for line in result.stdout.decode().split('\n') if STATE.match(line)]
    actual = dict(line.split('=', 1) for line in lines)
    assert len(actual) == len(lines), (context, 'duplicate output field')
    return actual


def states(result, expected, context):
    COUNTS['checks'] += 1
    actual = fields(result, context)
    assert actual == expected, (context, {k: v[:300] for k, v in actual.items()},
                                {k: v[:300] for k, v in expected.items()})


def categories(stderr):
    return [x.decode('ascii') for x in re.findall(rb'AIL-RUNTIME-([A-Z-]+):', stderr)]


def fatal(result, category, context):
    COUNTS['checks'] += 1
    assert result.returncode > 0 and result.stdout == b'', (context, result.returncode, result.stdout[:500])
    assert categories(result.stderr) == [category], (context, category, result.stderr[-2000:])


def quoted(value):
    escapes = {'"': '\\"', '\\': '\\\\', '\n': '\\n', '\r': '\\r', '\t': '\\t'}
    return '"' + ''.join(escapes[c] if c in escapes else f'\\u{ord(c):04x}'
                         if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value) + '"'


def transport(value):
    return quoted(value) if isinstance(value, str) else '[' + ','.join(map(transport, value)) + ']'


def alternate(value):
    return json.dumps(value, ensure_ascii=True, indent=1)


def fixed(routes):
    cases = []
    for path in sorted((RUNG / 'cases').glob('*.args')):
        arguments = path.read_text().splitlines()
        data = dict(line.split('=', 1) for line in arguments)
        expected = REFERENCE['analyze'](json.loads(data['data.values']), int(data['data.query']))
        cases.append((path.stem, arguments, expected))
    for route in routes:
        for label, arguments, expected in cases:
            states(run(route, 'Analyze', arguments), expected, (route[0], label))
        print(f'fixed {route[0]}: {len(cases)}', flush=True)
    print(f'PASS: {len(cases)} frozen BST cases on {len(routes)} routes', flush=True)


def generated(routes):
    rng = random.Random(SEED)
    cases = []
    for index in range(72):
        values = [rng.randrange(-2000, 2001) for _ in range(rng.randrange(0, 160))]
        if index % 4 == 0:
            values = [rng.choice([-(2**63), 2**63 - 1, 0, -1, 1]) for _ in range(100)]
        query = rng.choice(values) if values and index % 2 else 9000 + index
        expected = REFERENCE['analyze'](values, query)
        assert int(expected['out.height']) <= 62
        cases.append((str(index), ['data.values=' + json.dumps(values), 'data.query=' + str(query)], expected))
    for route in routes:
        for label, arguments, expected in cases:
            states(run(route, 'Analyze', arguments), expected, (route[0], label))
        print(f'generated {route[0]}: {len(cases)}', flush=True)
    print(f'PASS: {len(cases)} seeded BST shapes on {len(routes)} routes', flush=True)


def identity(name, type_name):
    return (f'Function: {name}.\nThe function needs:\n- value: {type_name}\n'
            f'The function produces:\n- result: {type_name}\n'
            'The function executes:\n- the function returns local.value\n\n')


def leaf_source():
    text = ('The application LeafAcceptance manages values.\n\n'
            'Type: Box<T>.\nBox has variants:\n- Empty\n- Full(value: T)\n\n'
            'Type: List<T>.\nList has variants:\n- Nothing\n- Something(value: T)\n\nA Data has:\n')
    for name, type_name in LEAVES.items():
        text += f'- {name}: {type_name}\n- {name}Box: Box<{type_name}>\n'
    text += '\nAn Out has:\n'
    for name, type_name in LEAVES.items():
        text += f'- {name}: {type_name}\n- {name}Box: Box<{type_name}>\n- {name}Test: Bool\n'
    text += '\n'
    for name, type_name in LEAVES.items():
        text += identity('copy' + name, f'Box<{type_name}>')
    text += 'Action: Construct.\nWhen Construct happens:\n'
    for name, type_name in LEAVES.items():
        text += (f'- the system constructs Box<{type_name}> variant Full with value = data.{name} into data.{name}Box\n'
                 f'- the system calls copy{name} with value = data.{name}Box into out.{name}Box\n'
                 f'- the system gets field value of Box<{type_name}> variant Full from out.{name}Box into out.{name}\n'
                 f'- the system tests Box<{type_name}> variant Full of out.{name}Box into out.{name}Test\n')
    for name in LEAVES:
        for suffix in ('', 'Box', 'Test'):
            text += f'- the system returns out.{name}{suffix}\n'
    text += '- the system records a trace event named Constructed\n'
    text += '\nAction: Inspect.\nWhen Inspect happens:\n'
    for name, type_name in LEAVES.items():
        text += (f'- the system calls copy{name} with value = data.{name}Box into out.{name}Box\n'
                 f'- the system gets field value of Box<{type_name}> variant Full from out.{name}Box into out.{name}\n'
                 f'- the system tests Box<{type_name}> variant Full of data.{name}Box into out.{name}Test\n')
    for name in LEAVES:
        for suffix in ('', 'Box', 'Test'):
            text += f'- the system returns out.{name}{suffix}\n'
    text += '- the system records a trace event named Inspected\n'
    return text


def float_bits(value):
    number = float(value)
    return 0x7ff8000000000000 if math.isnan(number) else struct.unpack('>Q', struct.pack('>d', number))[0]


def check_leaf(type_name, actual, expected, context):
    if type_name == 'Float':
        assert float_bits(actual) == float_bits(expected), (context, actual, expected)
        assert actual in ('nan', 'inf', '-inf') or 'e' in actual, (context, 'noncanonical float', actual)
    elif type_name == 'List<Float>':
        actual_items = [] if actual == '[]' else actual[1:-1].split(',')
        expected_items = [] if expected == '[]' else expected[1:-1].split(',')
        assert len(actual_items) == len(expected_items), context
        for actual_item, expected_item in zip(actual_items, expected_items):
            check_leaf('Float', actual_item, expected_item, context)
    else:
        assert actual == expected, (context, actual[:300], expected[:300])


def leaves(root):
    source = package(root, 'builtin-leaves', leaf_source())
    routes = build(root, source, 'leaves', ('Construct', 'Inspect'))
    # Number spellings and repeated JSON members are deliberately preserved.
    ordinary = {'integer': ('+0007', '7'), 'floating': ('-0', '-0'), 'boolean': ('true', 'true'),
                'text': ('héllo🙂\x85"\\', 'héllo🙂\x85"\\'), 'bytes': ('hex:0080ff', 'hex:0080ff'),
                'integers': ('[ -0, 2, -3 ]', '[0,2,-3]'), 'floats': ('[ -0, nan, inf, -inf, 1.5 ]', '[-0,nan,inf,-inf,1.5]'),
                'texts': ('[ "a", "\\u007f", "é" ]', '["a","\\u007f","é"]'),
                'document': ('{ "x":1e+999,"x":-0.00E+09,"s":"\\u0085" }', '{"x":1e+999,"x":-0.00E+09,"s":"\\u0085"}')}
    cases = [ordinary]
    for raw, expected in [('0', '0'), ('-9223372036854775808', '-9223372036854775808'), ('9223372036854775807', '9223372036854775807')]:
        cases.append(ordinary | {'integer': (raw, expected)})
    for raw in ['nan', '+inf', '-inf', '5e-324', '1.7976931348623157e308']:
        cases.append(ordinary | {'floating': (raw, raw)})
    cases.append(ordinary | {'boolean': ('false', 'false'), 'text': ('', ''), 'bytes': ('hex:', 'hex:'),
                            'integers': ('[]', '[]'), 'floats': ('[]', '[]'), 'texts': ('[]', '[]'), 'document': ('null', 'null')})
    for route in routes:
        for index, case in enumerate(cases):
            for action in ('Construct', 'Inspect'):
                arguments = [f'data.{name}' + ('Box=' + alternate(['Full', raw]) if action == 'Inspect' else '=' + raw)
                             for name, (raw, _) in case.items()]
                actual = fields(run(route, action, arguments), (route[0], action, index))
                assert len(actual) == len(LEAVES) * 3
                for name, type_name in LEAVES.items():
                    COUNTS['checks'] += 1
                    context = (route[0], action, index, name)
                    leaf = actual['out.' + name]
                    check_leaf(type_name, leaf, case[name][1], context)
                    assert actual['out.' + name + 'Box'] == transport(['Full', leaf]), context
                    assert actual['out.' + name + 'Test'] == 'true', context
    print(f'PASS: all nine builtin leaves, custom calls, canonical transport and extraction on {len(routes)} routes', flush=True)
    return routes


def generics(root):
    library = package(root, 'generic-library', '''The application GenericLibrary manages recursive values.

Type: Pair<A,B>.
Pair has variants:
- Pair(first: A, second: B)

Type: Left<A,B>.
Left has variants:
- End
- More(head: A, tail: Right<B,A>)

Type: Right<A,B>.
Right has variants:
- End
- More(head: A, tail: Left<B,A>)

Type: Wrapped<T>.
Wrapped has variants:
- Wrap(value: Pair<T,Left<T,Text>>)

Type: List<T>.
List has variants:
- Nil
- Cons(head: T, tail: List<T>)

Function: make.
The function needs:
- value: Int
- label: Text
The function produces:
- result: Wrapped<Int>
The function stores:
- left: Left<Int,Text>
- right: Right<Text,Int>
- pair: Pair<Int,Left<Int,Text>>
- wrapped: Wrapped<Int>
The function executes:
- the function constructs Left<Int,Text> variant End with none into local.left
- the function constructs Right<Text,Int> variant More with tail = local.left; head = local.label into local.right
- the function constructs Left<Int,Text> variant More with tail = local.right; head = local.value into local.left
- the function constructs Pair<Int,Left<Int,Text>> variant Pair with second = local.left; first = local.value + 1 into local.pair
- the function constructs Wrapped<Int> variant Wrap with value = local.pair into local.wrapped
- the function returns local.wrapped
''')
    source = package(root, 'generic-client', '''The application GenericClient manages imported values.

A Data has:
- value: Int
- label: Text

An Out has:
- wrapped: Lib.Wrapped<Int>
- pair: Lib.Pair<Int,Lib.Left<Int,Text>>
- first: Int
- found: Bool
- collection: Lib.List<Int>

Action: Run.
When Run happens:
- the system calls Lib.make with label = data.label; value = data.value into out.wrapped
- the system gets field value of Lib.Wrapped<Int> variant Wrap from out.wrapped into out.pair
- the system gets field first of Lib.Pair<Int,Lib.Left<Int,Text>> variant Pair from out.pair into out.first
- the system tests Lib.Wrapped<Int> variant Wrap of out.wrapped into out.found
- the system constructs Lib.List<Int> variant Nil with none into out.collection
- the system constructs Lib.List<Int> variant Cons with tail = out.collection; head = data.value into out.collection
- the system returns out.wrapped
- the system returns out.pair
- the system returns out.first
- the system returns out.found
- the system returns out.collection
- the system records a trace event named Built
''', imports=f'{library} compatible ^0.1 as Lib')
    routes = build(root, source, 'generics')
    for route in routes:
        for number, label in [(0, ''), (-8, 'm🙂'), (2**63 - 2, 'quoted"\\')]:
            pair = ['Pair', str(number + 1), ['More', str(number), ['More', label, ['End']]]]
            states(run(route, arguments=['data.value=' + str(number), 'data.label=' + label]),
                   {'out.wrapped': transport(['Wrap', pair]), 'out.pair': transport(pair),
                    'out.first': str(number + 1), 'out.found': 'true',
                    'out.collection': transport(['Cons', str(number), ['Nil']])}, route[0])
    print('PASS: nested multi-parameter and mutually recursive generics, imported type/payload/local qualification', flush=True)
    return routes


def op(opcode, **operands):
    return {'opcode': opcode, 'operands': operands}


def seed(key, value):
    return op('SET_FIELD', key=key, value=value, text='independent variant fixture')


def output(key):
    return op('OUTPUT_FIELD', key=key)


def document(schemas, instructions, routines=()):
    return {'kind': 'AIL-Bytecode', 'package': 'value-type-oracle', 'version': '1',
            'profile': 'Application', 'capability_grants': [], 'target_support': {},
            'external_bindings': [], 'failures': [], 'type_schemas': schemas,
            'actions': [{'action': 'Run', 'instructions': instructions + [op('RETURN_SUCCESS')]}] + list(routines)}


def compiled(root, name, schemas, instructions, routines=(), profile='Application'):
    artifact = root / (name + '.json')
    program = document(schemas, instructions, routines)
    program['profile'] = profile
    artifact.write_text(json.dumps(program))
    binary = root / (name + '.elf')
    checked([AIL, 'ail-compile', artifact, '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', binary])
    return [(name + '/' + backend, artifact, {'Run': binary}, backend) for backend in ('VM', 'native')]


def test(type_name='Tree', variant='Empty', source='data.value', key='out.test'):
    return op('VALUE_TEST', type=type_name, variant=variant, source=source, key=key)


def get(type_name='Tree', variant='Node', field='value', source='data.value', key='out.value'):
    return op('VALUE_GET', type=type_name, variant=variant, field=field, source=source, key=key)


def construct(type_name, variant, arguments, key='out.value'):
    return op('VALUE_CONSTRUCT', type=type_name, variant=variant, arguments=arguments, key=key)


def routine(name, parameters, result, body, locals_=''):
    return {'action': name, 'instructions': [op('VALUE_FUNCTION', parameters=parameters, locals=locals_, result=result)] + body}


TREE_SCHEMA = {'Tree': {'Empty': {}, 'Node': {'left': 'Tree', 'right': 'Tree', 'value': 'Int'}}}


def caught(operation, errors='VALUE'):
    return [op('TRY_BEGIN', errors=errors, key='out.error'), operation,
            op('RECOVER'), op('TRY_END')]


def recovery(root):
    source = package(root, 'value-recovery', '''The application ValueRecovery manages trees.

Type: Tree.
Tree has variants:
- Empty
- Node(left: Tree, right: Tree, value: Int)

A Data has:
- value: Tree

An Out has:
- error: Text
- tested: Bool
- value: Int
- tree: Tree

Function: ignore.
The function needs:
- value: Tree
The function produces:
- result: Int
The function executes:
- the function returns 7

Action: Inspect.
When Inspect happens:
- the system assigns "" to out.error
- the system assigns false to out.tested
- the system assigns 41 to out.value
- the system constructs Tree variant Empty with none into out.tree
- try:
- the system tests Tree variant Empty of data.value into out.tested
- the system gets field value of Tree variant Node from data.value into out.value
- the system constructs Tree variant Node with value = out.value + 1; right = out.tree; left = data.value into out.tree
- recover VALUE into out.error:
- end try
- the system returns out.error
- the system returns out.tested
- the system returns out.value
- the system returns out.tree
- the system records a trace event named Inspected

Action: Unused.
When Unused happens:
- the system assigns "" to out.error
- the system assigns 41 to out.value
- try:
- the system calls ignore with value = data.value into out.value
- recover any into out.error:
- end try
- the system returns out.error
- the system returns out.value
- the system records a trace event named Called
''')
    routes = build(root, source, 'recovery', ('Inspect', 'Unused'))
    malformed = [None, '', 'null', '{}', '[]', '["Unknown"]', '["Empty","extra"]',
                 '["Empty"] trailing', '["Empty",]', '["Empty"', '["Empty"]\x00',
                 '["Node",["Empty"],["Empty"]]', '["Node",["Empty"],["Empty"],7]',
                 '["Node",["Empty"],["Empty"],"9223372036854775808"]',
                 '["Node",["Empty"],["Empty"],"1e0"]',
                 '["Node",["Empty"],["BadUnusedBranch"],"7"]',
                 '["Node",["Empty"],["Empty"],"PRIVATE_BAD_PAYLOAD"]',
                 '["Node",["Empty"],["Empty"],"\\ud800"]']
    # NUL cannot be a process argument; it is covered by an embedded seed below.
    seeded_only = malformed.pop(malformed.index('["Empty"]\x00'))
    valid = [(['Empty'], 0), (['Node', ['Empty'], ['Empty'], '+0007'], 7),
             (['Node', ['Node', ['Empty'], ['Empty'], '-2'], ['Empty'], '-0'], 0)]
    for route in routes:
        for raw in malformed:
            arguments = [] if raw is None else ['data.value=' + raw]
            for action in ('Inspect', 'Unused'):
                expected = {'out.error': 'AIL-RUNTIME-VALUE', 'out.value': '41'}
                if action == 'Inspect':
                    expected |= {'out.tested': 'false', 'out.tree': '["Empty"]'}
                result = run(route, action, arguments)
                states(result, expected, (route[0], action, 'malformed', raw))
                assert b'PRIVATE_BAD_PAYLOAD' not in result.stdout + result.stderr
        for tree, value in valid:
            raw = alternate(tree)
            states(run(route, 'Unused', ['data.value=' + raw]), {'out.error': '', 'out.value': '7'}, route[0])
            if tree[0] == 'Empty':
                expected = {'out.error': 'AIL-RUNTIME-VALUE', 'out.value': '41', 'out.tested': 'true', 'out.tree': '["Empty"]'}
            else:
                normalized = copy.deepcopy(tree)
                normalized[-1] = str(value)
                expected = {'out.error': '', 'out.value': str(value), 'out.tested': 'false',
                            'out.tree': transport(['Node', normalized, ['Empty'], str(value + 1)])}
            states(run(route, 'Inspect', ['data.value=' + raw]), expected, route[0])

    for route in compiled(root, 'embedded-nul', TREE_SCHEMA, [seed('data.value', seeded_only)] + caught(test()) + [output('out.error')]):
        states(run(route), {'out.error': 'AIL-RUNTIME-VALUE'}, route[0])

    schemas = TREE_SCHEMA | {'Leaves': {'Full': {'a': 'List<Int>', 'z': 'Int'}}}
    operations = [
        ('invalid-builtin', construct('Leaves', 'Full', 'a=f:data.bad;z=i:2'), 'VALUE'),
        ('sorted-validation', construct('Leaves', 'Full', 'a=f:data.bad;z=i:1 i:0 /'), 'VALUE'),
        ('integer-divzero', construct('Leaves', 'Full', 'a=f:data.good;z=i:1 i:0 /'), 'DIVZERO'),
        ('integer-overflow', construct('Leaves', 'Full', 'a=f:data.good;z=i:9223372036854775807 i:1 +'), 'OVERFLOW'),
        ('integer-missing', construct('Leaves', 'Full', 'a=f:data.good;z=f:missing.int'), 'INT')]
    for name, operation, category in operations:
        steps = [seed('data.bad', '[1,"PRIVATE_BAD"]'), seed('data.good', '[]'), seed('out.value', 'sentinel')]
        steps += caught(operation, 'any') + [output('out.error'), output('out.value')]
        for route in compiled(root, name, schemas, steps):
            states(run(route), {'out.error': 'AIL-RUNTIME-' + category, 'out.value': 'sentinel'}, route[0])

    malformed_leaves = {'Int': '7.0', 'Float': 'NaN', 'Bool': 'True', 'Bytes': 'hex:FF',
                        'List<Int>': '[1,true]', 'List<Float>': '[1,"2"]',
                        'List<Text>': '["ok",2]', 'Json': '{"a":1,}'}
    for type_name in LEAVES.values():
        schema = {'Box': {'Empty': {}, 'Full': {'value': type_name}}}
        # A test for Empty must validate Full's payload before returning false.
        raw = '["Full","\\ud800"]' if type_name == 'Text' else transport(['Full', malformed_leaves[type_name]])
        steps = [seed('data.value', raw), seed('out.test', 'true')] + caught(test('Box', 'Empty'))
        steps += [output('out.test'), output('out.error')]
        for route in compiled(root, 'malformed-leaf-' + type_name.replace('<', '-').replace('>', ''), schema, steps):
            states(run(route), {'out.test': 'true', 'out.error': 'AIL-RUNTIME-VALUE'}, route[0])

    # Alias destinations must be read completely before publication, and function
    # argument checks are sorted even when the body ignores every argument.
    routines = [routine('ignore', 'a:Tree;z:Tree', 'Int', [op('RETURN_VALUE', type='Int', expression='i:7')])]
    too_deep = '["Empty"]'
    for _ in range(64):
        too_deep = '["Node",' + too_deep + ',["Empty"],"0"]'
    for a, z, expected in [('["Unknown"]', too_deep, 'VALUE'), (too_deep, '["Unknown"]', 'VALUE-LIMIT')]:
        steps = [seed('data.a', a), seed('data.z', z), seed('out.value', 'sentinel')]
        steps += caught(op('CALL_VALUE', target='ignore', arguments='a=f:data.a;z=f:data.z', key='out.value'), 'any')
        steps += [output('out.error'), output('out.value')]
        for route in compiled(root, 'sorted-function-' + expected, TREE_SCHEMA, steps, routines):
            result = run(route)
            if expected == 'VALUE-LIMIT':
                fatal(result, expected, route[0])
            else:
                states(result, {'out.error': 'AIL-RUNTIME-VALUE', 'out.value': 'sentinel'}, route[0])
    identity_ = routine('copy', 'value:Tree', 'Tree', [op('RETURN_VALUE', type='Tree', expression='f:local.value')])
    steps = [seed('out.tree', '[ "Node", ["Empty"], ["Empty"], "+0007" ]'),
             op('CALL_VALUE', target='copy', arguments='value=f:out.tree', key='out.tree'),
             construct('Tree', 'Node', 'left=f:out.tree;right=f:out.tree;value=i:8', 'out.tree'),
             get(field='left', source='out.tree', key='out.tree'), output('out.tree')]
    for route in compiled(root, 'alias-custom', TREE_SCHEMA, steps, [identity_]):
        states(run(route), {'out.tree': '["Node",["Empty"],["Empty"],"7"]'}, route[0])

    # At the 64th frame, malformed custom arguments win over the next depth
    # check. The final local is declared but intentionally never initialized.
    recurse = routine('recurse', 'a:Tree;n:Int', 'Int', [
        op('BRANCH_INT', left='f:local.n', comparison='>=', right='i:63', label='last'),
        op('CALL_VALUE', target='recurse', arguments='a=f:local.a;n=f:local.n i:1 +', key='local.result'),
        op('RETURN_VALUE', type='Int', expression='f:local.result'), op('LABEL', name='last'),
        op('CALL_VALUE', target='recurse', arguments='a=f:local.unset;n=i:64', key='local.result'),
        op('RETURN_VALUE', type='Int', expression='f:local.result')], 'result:Int;unset:Tree')
    steps = [seed('data.tree', '["Empty"]'), seed('out.value', '41')]
    steps += caught(op('CALL_VALUE', target='recurse', arguments='a=f:data.tree;n=i:0', key='out.value'))
    steps += [output('out.value'), output('out.error')]
    for route in compiled(root, 'custom-before-call-depth', TREE_SCHEMA, steps, [recurse]):
        states(run(route), {'out.value': '41', 'out.error': 'AIL-RUNTIME-VALUE'}, route[0])
    steps = [construct('Tree', 'Empty', '', 'data.empty'),
             construct('Tree', 'Node', 'left=f:data.empty;right=f:data.empty;value=i:7', 'data.tree'),
             test('Tree', 'Node', 'data.tree'), get(source='data.tree'), output('out.test'), output('out.value')]
    for route in compiled(root, 'c-interop-function-free', TREE_SCHEMA, steps, profile='C interop'):
        states(run(route), {'out.test': 'true', 'out.value': '7'}, route[0])
    # Encoded input can carry NUL/newlines, unlike a bare process argument.
    # Format the extracted Text before observing it so line-oriented output does
    # not hide or split a scalar. This uses explicit Python quoting expectations.
    schema = {'Box': {'Full': {'value': 'Text'}}}
    steps = [get('Box', 'Full', 'value', key='data.raw'),
             construct('Box', 'Full', 'value=f:data.raw', 'out.box'),
             op('JSON_FORMAT', key='out.text', source='data.raw', type='Text'),
             output('out.box'), output('out.text')]
    for route in compiled(root, 'text-controls', schema, steps):
        for text in ['\0', '\x01\b\f\n\r\t\x1f\x7f\x85', '\U0010ffff', '\\u0000"/\\']:
            states(run(route, arguments=['data.value=' + alternate(['Full', text])]),
                   {'out.box': transport(['Full', text]), 'out.text': quoted(text)}, route[0])
    print('PASS: full payload validation, unused arguments, sorted errors, wrong variants, aliasing and atomic recovery', flush=True)


def reject_protocol(root, name, program):
    artifact = root / (name + '.json')
    artifact.write_text(json.dumps(program))
    for command in ([AIL, 'ail-vm', artifact, '--action', 'Run'],
                    [AIL, 'ail-compile', artifact, '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', root / (name + '.elf')]):
        result = harness.invoke(command, timeout=60)
        COUNTS['checks'] += 1
        assert result.returncode > 0 and b'AILBC024' in result.stdout + result.stderr, (
            name, result.returncode, result.stdout[:500], result.stderr[-2000:])


def reject_source(root, name, source):
    location = package(root, name, source)
    result = harness.invoke([AIL, 'ail-lower', location], timeout=60)
    COUNTS['checks'] += 1
    assert result.returncode > 0, (name, 'invalid source accepted', result.stdout[:300])
    assert b'AIL-VALUE-001' in result.stdout + result.stderr or b'AIL-FUNCTION-001' in result.stdout + result.stderr, (
        name, result.stdout[:500], result.stderr[-2000:])


def protocol(root):
    base = document(TREE_SCHEMA, [test(), output('out.test')])
    mutations = []
    for name, schemas in [('unknown-payload', {'Tree': {'Empty': {}, 'Node': {'left': 'Unknown'}}}),
                          ('free-variable', {'Tree': {'Node': {'value': 'T'}}}),
                          ('bad-type-expression', {'Tree<Int,,Text>': {'Empty': {}}}),
                          ('bad-variant-name', {'Tree': {'bad-name': {}}}),
                          ('bad-payload-name', {'Tree': {'Empty': {'bad-name': 'Int'}}})]:
        modified = copy.deepcopy(base)
        modified['type_schemas'] = schemas
        mutations.append((name, modified))
    invalid_operations = [test(type_name='Unknown'), test(variant='Unknown'),
                          get(field='absent'), get(variant='Empty'),
                          construct('Tree', 'Node', 'left=f:data.value;right=f:data.value'),
                          construct('Tree', 'Node', 'left=f:data.value;right=f:data.value;value=i:1;extra=i:2'),
                          construct('Tree', 'Node', 'left=f:data.value;right=f:data.value;value=i:1;value=i:2'),
                          construct('Tree', 'Node', 'left=i:1;right=f:data.value;value=i:1'),
                          construct('Tree', 'Empty', 'extra=i:2')]
    for index, operation in enumerate(invalid_operations):
        # Invalid operations remain illegal behind an unconditional return.
        mutations.append((f'unreachable-operation-{index}', document(TREE_SCHEMA, [op('RETURN_SUCCESS'), operation])))
    modified = copy.deepcopy(base)
    modified['actions'][0]['instructions'][0]['operands']['unchecked'] = 'packed'
    mutations.append(('extra-operand', modified))
    for name, program in mutations:
        reject_protocol(root, name, program)
    # External untyped bytes may be interpreted against either nominal schema,
    # but a value constructed by checked bytecode already has an exact type.
    nominal_schemas = {'First': {'Empty': {}}, 'Second': {'Empty': {}}}
    reject_protocol(root, 'action-bytecode-nominal-mismatch', document(nominal_schemas, [
        construct('First', 'Empty', '', 'data.value'), test('Second', 'Empty', 'data.value')]))

    prefix = '''The application Rejection manages values.
Type: First.
First has variants:
- Empty
- Full(value: Int)
Type: Second.
Second has variants:
- Empty
- Full(value: Int)
A Data has:
- first: First
- second: Second
- number: Int
- flag: Bool
'''
    action = '\nAction: Run.\nWhen Run happens:\n'
    for name, statement in [
            ('nominal-get', '- the system gets field value of First variant Full from data.second into data.number'),
            ('nominal-construct', '- the system constructs First variant Empty with none into data.second'),
            ('nominal-test', '- the system tests First variant Empty of data.second into data.flag'),
            ('wrong-test-destination', '- the system tests First variant Empty of data.first into data.number'),
            ('wrong-get-destination', '- the system gets field value of First variant Full from data.first into data.flag'),
            ('unknown-field', '- the system gets field missing of First variant Full from data.first into data.number'),
            ('missing-constructor-field', '- the system constructs First variant Full with none into data.first'),
            ('duplicate-constructor-field', '- the system constructs First variant Full with value = 1; value = 2 into data.first')]:
        reject_source(root, name, prefix + action + statement + '\n')
    reject_source(root, 'nominal-function', prefix + identity('copy', 'First') + action +
                  '- the system calls copy with value = data.second into data.first\n')
    reject_source(root, 'nominal-return', prefix + '''Function: wrong.
The function needs:
- value: Second
The function produces:
- result: First
The function executes:
- the function returns local.value
''' + action + '- the system calls wrong with value = data.second into data.first\n')
    reject_source(root, 'wrong-generic-arity', '''The application Rejection manages values.
Type: Box<T>.
Box has variants:
- Empty
A Data has:
- value: Box<Int,Text>
Action: Run.
When Run happens:
- the system constructs Box<Int,Text> variant Empty with none into data.value
''')
    for name, declaration in [('duplicate-variant', 'Type: First.\nFirst has variants:\n- Empty\n- Empty\n'),
                              ('duplicate-payload', 'Type: First.\nFirst has variants:\n- Empty\n- Full(value: Int, value: Int)\n'),
                              ('duplicate-type', 'Type: First.\nFirst has variants:\n- Empty\nType: First.\nFirst has variants:\n- Empty\n')]:
        reject_source(root, name, 'The application Rejection manages values.\n' + declaration +
                      'A Data has:\n- first: First\n' + action +
                      '- the system constructs First variant Empty with none into data.first\n')
    reject_source(root, 'unbounded-instantiation', '''The application Rejection manages values.
Type: Grow<T>.
Grow has variants:
- End
- Next(value: Grow<Grow<T>>)
A Data has:
- value: Grow<Int>
Action: Run.
When Run happens:
- the system constructs Grow<Int> variant End with none into data.value
''')
    # Recursive schema cycles are legal even when they have no finite value.
    for route in compiled(root, 'nonproductive-recursion', {'Loop': {'Next': {'next': 'Loop'}}}, []):
        states(run(route), {}, route[0])

    # Existing descriptive declarations are inert outside executable roots.
    # Head ambiguity must not reject an ordinary integer-only application.
    descriptive = package(root, 'unused-descriptive-overloads', '''The application Descriptive manages values.
Type: Box<T>.
Box has variants:
- One
Type: Box<X,Y>.
Box has variants:
- Two
A Data has:
- number: Int
Action: Run.
When Run happens:
- the system assigns 7 to data.number
- the system returns data.number
- the system records a trace event named Done
''')
    artifact = root / 'unused-descriptive-overloads.json'
    artifact.write_bytes(checked([AIL, 'ail-lower', descriptive]))
    assert 'type_schemas' not in json.loads(artifact.read_bytes()), 'empty schema metadata must be omitted'
    binary = root / 'unused-descriptive-overloads.elf'
    checked([AIL, 'ail-compile', artifact, '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', binary])
    for backend in ('VM', 'native'):
        route = ('unused-descriptive-overloads/' + backend, artifact, {'Run': binary}, backend)
        states(run(route), {'data.number': '7'}, route[0])

    # Exercise the public Core loader with declaration ownership/provenance
    # damage. These are structural graph checks, independent of source syntax.
    valid = package(root, 'core-ownership', prefix + action +
                    '- the system constructs First variant Full with value = 7 into data.first\n'
                    '- the system gets field value of First variant Full from data.first into data.number\n'
                    '- the system returns data.number\n'
                    '- the system records a trace event named Constructed\n')
    core = checked([AIL, 'ail-core', valid]).decode()
    anchors = {
        'core-missing-field-owner': 'edge has_field Variant:First.Full -> Field:First.Full.value',
        'core-missing-variant-owner': 'edge contains Type:First -> Variant:First.Full',
        'core-missing-field-provenance': 'edge has_provenance Field:First.Full.value -> ',
        'core-missing-argument': 'edge variant_argument VariantOperation:Run.variant0 -> ',
        'core-missing-operation-owner': 'edge variant_step Action:Run -> VariantOperation:Run.variant0 ',
        'core-missing-argument-provenance': 'edge has_provenance IntegerExpression:Run.variant0.argument.value -> ',
    }
    damages = {}
    for name, anchor in anchors.items():
        matching = [line for line in core.splitlines() if line.startswith(anchor)]
        assert len(matching) == 1, (name, 'Core mutation anchor changed', matching)
        damages[name] = core.replace(matching[0] + '\n', '', 1)
    argument = next(line for line in core.splitlines() if line.startswith('edge variant_argument '))
    damages['core-duplicate-argument'] = core + argument + '\n'
    damages['core-wrong-argument-name'] = core.replace(argument, argument.replace('[name=value]', '[name=unknown]'), 1)
    damages['core-wrong-argument-type'] = core.replace(
        'node IntegerExpression Run.variant0.argument.value : Int ',
        'node IntegerExpression Run.variant0.argument.value : Bool ', 1)
    for name, contents in damages.items():
        assert contents != core, (name, 'ineffective Core mutation')
        damaged = root / (name + '.core')
        damaged.write_text(contents)
        result = harness.invoke([AIL, 'ail-lower', '--core-file', damaged], timeout=60)
        COUNTS['checks'] += 1
        assert result.returncode > 0 and b'AIL-VALUE-001' in result.stdout + result.stderr, (
            name, result.returncode, result.stdout[:500], result.stderr[-2000:])
    print('PASS: malformed schemas/operands, unreachable bad operations, nominal typing and declaration closure rejection', flush=True)


def boundary(root, name, schemas, value, operation, expected=None, category=None):
    instructions = [seed('data.value', value)] + caught(operation, 'any')
    instructions += [output('out.value' if operation['opcode'] != 'VALUE_TEST' else 'out.test')]
    for route in compiled(root, name, schemas, instructions):
        result = run(route)
        if category:
            fatal(result, category, route[0])
        else:
            states(result, expected, route[0])


def limits(root):
    chain = {'Chain': {'End': {}, 'Next': {'next': 'Chain'}}}
    value = ['End']
    for depth in range(1, 66):
        if depth in (1, 63, 64, 65):
            boundary(root, f'custom-depth-{depth}', chain, transport(value), test('Chain', 'End'),
                     expected={'out.test': str(depth == 1).lower()} if depth <= 64 else None,
                     category='VALUE-LIMIT' if depth > 64 else None)
        value = ['Next', value]

    # 1 root + 255 bags * (1 bag + 256 Int leaves) = exactly 65,536 values.
    bags = {f'v{i:03}': 'Int' for i in range(256)}
    schemas = {'Bag': {'Full': bags}, 'Root': {'Full': {f'b{i:03}': 'Bag' for i in range(255)}}}
    bag = ['Full'] + ['0'] * 256
    value = ['Full'] + [bag] * 255
    boundary(root, 'value-count-exact', schemas, transport(value), test('Root', 'Full'), {'out.test': 'true'})
    schemas['Root']['Full']['z'] = 'Int'
    value.append('0')
    boundary(root, 'value-count-over', schemas, transport(value), test('Root', 'Full'), category='VALUE-LIMIT')

    # Whitespace counts toward the input bound, without inventing a smaller tree limit.
    empty = '["End"]'
    for extra in (0, 1):
        raw = ' ' * (TRANSPORT_LIMIT - len(empty) + extra) + empty
        boundary(root, 'raw-transport-' + str(extra), chain, raw, test('Chain', 'End'),
                 {'out.test': 'true'} if not extra else None, 'VALUE-LIMIT' if extra else None)

    # Canonical escaping expands a valid borrowed input beyond 8 MiB. Testing
    # and extracting a small field still work; publishing a custom copy fails.
    payloads = {f'p{i}': 'Text' for i in range(9)}
    payloads['z'] = 'Int'
    large_schema = {'Wide': {'Full': payloads}}
    # C1 controls are legal literal UTF-8 input with six-byte canonical escaping.
    payload = '\x85' * 160_000
    raw = json.dumps(['Full'] + [payload] * 9 + ['7'], ensure_ascii=False)
    assert len(raw.encode()) < TRANSPORT_LIMIT
    assert len(transport(['Full'] + [payload] * 9 + ['7']).encode()) > TRANSPORT_LIMIT
    boundary(root, 'borrowed-expansion-test', large_schema, raw, test('Wide', 'Full'), {'out.test': 'true'})
    boundary(root, 'borrowed-expansion-get', large_schema, raw,
             get('Wide', 'Full', 'z'), {'out.value': '7'})
    identity_ = routine('copy', 'value:Wide', 'Wide', [op('RETURN_VALUE', type='Wide', expression='f:local.value')])
    steps = [seed('data.value', raw)] + caught(op('CALL_VALUE', target='copy', arguments='value=f:data.value', key='out.value'), 'any') + [output('out.value')]
    for route in compiled(root, 'owned-expansion-limit', large_schema, steps, [identity_]):
        fatal(run(route), 'VALUE-LIMIT', route[0])

    # A borrowed embedded builtin can likewise expand beyond its own canonical
    # output bound. Validation may inspect it; extracting/publishing may not.
    for type_name, leaf in [('Json', '"' + '\x85' * 200_000 + '"'),
                           ('List<Text>', '["' + '\x85' * 200_000 + '"]')]:
        schema = {'Box': {'Full': {'value': type_name}}}
        raw = json.dumps(['Full', leaf], ensure_ascii=False)
        stem = 'builtin-canonical-' + type_name.split('<')[0]
        boundary(root, stem + '-test', schema, raw, test('Box', 'Full'), {'out.test': 'true'})
        boundary(root, stem + '-get', schema, raw, get('Box', 'Full', 'value'), category='VALUE-LIMIT')

    # Full leaf limits remain independent of the custom-value count/depth.
    for type_name, good, bad in [('Bytes', 'hex:' + '00' * LEAF_LIMIT, 'hex:' + '00' * (LEAF_LIMIT + 1)),
                               ('List<Int>', '[' + ','.join(['0'] * 65536) + ']', '[' + ','.join(['0'] * 65537) + ']')]:
        schema = {'Box': {'Full': {'value': type_name}}}
        for label, raw_leaf in [('exact', good), ('over', bad)]:
            boundary(root, 'leaf-limit-' + type_name.split('<')[0] + '-' + label, schema,
                     transport(['Full', raw_leaf]), test('Box', 'Full'),
                     {'out.test': 'true'} if label == 'exact' else None,
                     'VALUE-LIMIT' if label == 'over' else None)

    # Schema limits must reject statically even when no value reaches that branch.
    for count in (256, 257):
        schema = {'V': {f'V{i}': {} for i in range(count)}}
        if count == 256:
            boundary(root, 'variants-exact', schema, '["V0"]', test('V', 'V0'), {'out.test': 'true'})
        else:
            reject_protocol(root, 'variants-over', document(schema, [test('V', 'V0')]))
    reject_protocol(root, 'fields-over', document({'F': {'Full': {f'f{i}': 'Int' for i in range(257)}}}, []))
    for length in (128, 129):
        name = 'T' * length
        schema = {name: {'End': {}}}
        if length == 128:
            boundary(root, 'identifier-exact', schema, '["End"]', test(name, 'End'), {'out.test': 'true'})
        else:
            reject_protocol(root, 'identifier-over', document(schema, []))
    for count in (4096, 4097):
        schema = {f'T{i}': {'End': {}, 'Next': {'value': f'T{i + 1}'}} for i in range(count - 1)}
        schema[f'T{count - 1}'] = {'End': {}}
        if count == 4096:
            boundary(root, 'concrete-types-exact', schema, '["End"]', test('T0', 'End'), {'out.test': 'true'})
        else:
            reject_protocol(root, 'concrete-types-over', document(schema, []))

    # Both independent syntax bounds are inclusive; a long but valid name is
    # not constrained by a new small implementation-specific scratch buffer.
    def closed_chain(name):
        schemas = {}
        while name != 'Int':
            schemas[name] = {'End': {}}
            if '<' not in name:
                break
            name = name.split('<', 1)[1][:-1]
        return schemas

    for depth in (32, 33):
        name = 'Box<' * (depth - 1) + 'Int' + '>' * (depth - 1)
        schema = closed_chain(name)
        if depth == 32:
            boundary(root, 'type-depth-exact', schema, '["End"]', test(name, 'End'), {'out.test': 'true'})
        else:
            reject_protocol(root, 'type-depth-over', document(schema, []))
    long_head = 'T' * 128
    name = (long_head + '<') * 31 + 'I' * 66 + '>' * 31
    assert len(name) == 4096
    boundary(root, 'type-spelling-exact', closed_chain(name), '["End"]', test(name, 'End'), {'out.test': 'true'})
    longer = (long_head + '<') * 31 + 'I' * 67 + '>' * 31
    reject_protocol(root, 'type-spelling-over', document(closed_chain(longer), []))

    field_schema = {f'F{i}': {'Empty': {}, 'Full': {f'f{j}': 'Int' for j in range(256)}} for i in range(256)}
    for index in range(255):
        field_schema[f'F{index}']['Full']['f0'] = f'F{index + 1}'
    boundary(root, 'aggregate-fields-exact', field_schema, '["Empty"]', test('F0', 'Empty'), {'out.test': 'true'})
    field_schema['Overflow'] = {'One': {'value': 'Int'}}
    reject_protocol(root, 'aggregate-fields-over', document(field_schema, []))

    # Frozen ABI v1: 32-byte header, 8/type, 16/variant, 4/field and
    # concatenated variant names. Set total encoded length to exactly 4 MiB
    # while simultaneously reaching the aggregate 65,536-variant boundary.
    variants_total = 256 * 256
    base_bytes = 32 + 256 * 8 + variants_total * 16 + 255 * 4
    names_bytes = 4 * 1024 * 1024 - base_bytes
    width, extra = divmod(names_bytes, variants_total)
    schema = {}
    for i in range(256):
        variants = {}
        for j in range(256):
            n = i * 256 + j
            label = f'V{j:03}' + 'x' * (width - 4 + (n < extra))
            variants[label] = {'next': f'T{i + 1}'} if j == 0 and i < 255 else {}
        schema[f'T{i}'] = variants
    empty_variant = sorted(schema['T0'])[1]
    boundary(root, 'encoded-schema-exact', schema, transport([empty_variant]), test('T0', empty_variant), {'out.test': 'true'})
    label = next(iter(schema['T255']))
    schema['T255'][label + 'x'] = schema['T255'].pop(label)
    reject_protocol(root, 'encoded-schema-over', document(schema, []))
    # Restore the byte budget before independently overflowing the variant count.
    schema['T255'][label] = schema['T255'].pop(label + 'x')
    schema['Overflow'] = {'One': {}}
    reject_protocol(root, 'aggregate-variants-over', document(schema, []))

    # Produce an exact 8 MiB canonical value from individually valid Text leaves.
    fields_ = {f'p{i}': 'Text' for i in range(8)}
    overhead = len(transport(['Full'] + [''] * 8))
    last = TRANSPORT_LIMIT - overhead - 7 * LEAF_LIMIT
    payloads = ['x' * LEAF_LIMIT] * 7 + ['y' * last]
    canonical = transport(['Full'] + payloads)
    assert len(canonical) == TRANSPORT_LIMIT
    steps = [seed(f'data.p{i}', value) for i, value in enumerate(payloads)]
    steps += [construct('Wide', 'Full', ';'.join(f'p{i}=f:data.p{i}' for i in range(8))), output('out.value')]
    for route in compiled(root, 'produced-transport-exact', {'Wide': {'Full': fields_}}, steps):
        states(run(route), {'out.value': canonical}, route[0])

    # Exactly one successful custom constructor owns exactly one canonical copy.
    # A 1 MiB custom value permits 64 copies in the shared 64 MiB arena.
    large = 'x' * (LEAF_LIMIT - len('["Full",""]'))
    schemas = TREE_SCHEMA | {'Box': {'Full': {'value': 'Text'}}}
    one = construct('Box', 'Full', 'value=f:data.large', 'data.copy')
    ignore = routine('ignore', 'value:Box', 'Int', [op('RETURN_VALUE', type='Int', expression='i:7')])
    for copies in (64, 65):
        steps = [seed('data.large', large)]
        for _ in range(copies):
            # Recovery does not refund a successful earlier allocation.
            steps += [op('TRY_BEGIN', errors='INT', key='data.error'), one,
                      op('EVAL_INT', expression='f:missing.int', key='data.int'), op('RECOVER'), op('TRY_END')]
        for _ in range(80):
            steps += [op('CALL_VALUE', target='ignore', arguments='value=f:data.copy', key='out.number')]
        steps += [test('Box', 'Full', 'data.copy'), output('out.test'), output('out.number')]
        for route in compiled(root, f'shared-arena-{copies}', schemas, steps, [ignore]):
            if copies == 64:
                states(run(route), {'out.test': 'true', 'out.number': '7'}, route[0])
            else:
                fatal(run(route), 'VALUE-LIMIT', route[0])
    # The final MiB after 63 custom copies belongs to the same budget used by
    # ordinary Json. Its next allocation keeps Json's existing fatal category.
    steps = [seed('data.large', large), seed('data.json', '"' + 'j' * (LEAF_LIMIT - 2) + '"')]
    steps += [one] * 63
    steps += [op('JSON_PARSE', key='data.parsed', source='data.json', type='Json')]
    steps += caught(op('JSON_PARSE', key='data.parsed', source='data.json', type='Json'), 'any')
    steps += [output('data.parsed')]
    for route in compiled(root, 'shared-custom-json-arena', schemas, steps):
        fatal(run(route), 'JSON-LIMIT', route[0])
    print('PASS: runtime depth/count/transport/leaf boundaries, schema limits and shared allocation recovery', flush=True)


def mutation(root, routes):
    base = json.loads(routes[0][1].read_bytes())
    forbidden = ('PROCESS_', 'FILE_', 'DIRECTORY_', 'SORT', 'TREE_', 'HOST_', 'RLE_')
    for action in base['actions']:
        for instruction in action['instructions']:
            opcode = instruction['opcode']
            assert not any(word in opcode for word in forbidden), ('forbidden tree implementation', opcode)
    constructors = [instruction for action in base['actions'] for instruction in action['instructions']
                    if instruction['opcode'] == 'VALUE_CONSTRUCT' and 'value=' in instruction['operands']['arguments']]
    assert constructors, 'tree constructor mutation anchor missing'
    for instruction in constructors:
        parts = instruction['operands']['arguments'].split(';')
        instruction['operands']['arguments'] = ';'.join('value=i:0' if part.startswith('value=') else part for part in parts)
    artifact = root / 'mutant-tree-key.json'
    artifact.write_text(json.dumps(base))
    binary = root / 'mutant-tree-key.elf'
    checked([AIL, 'ail-compile', artifact, '--action', 'Analyze', '--target', 'linux-x86_64-elf', '--out', binary])
    wanted = REFERENCE['analyze']([4, 2, 6, 1, 3, 5, 7, 3], 3)
    for backend in ('VM', 'native'):
        route = ('mutant/' + backend, artifact, {'Analyze': binary}, backend)
        result = run(route, 'Analyze', ['data.values=[4,2,6,1,3,5,7,3]', 'data.query=3'])
        COUNTS['checks'] += 1
        assert result.returncode == 0, (route[0], result.stderr[-1000:])
        assert fields(result, route[0]) != wanted, (route[0], 'independent oracle missed corrupted keys')
    # Mutation of a typed operation cannot bypass validation with an early return.
    invalid = document(TREE_SCHEMA, [op('RETURN_SUCCESS'), construct('Tree', 'Node', 'value=i:0')])
    reject_protocol(root, 'mutation-bypass-validation', invalid)
    print('PASS: corrupted tree-key result detected and typed-validation bypass rejected on both backends', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--section', default='all', choices=['all', 'fixed', 'generated', 'leaves',
                        'generics', 'recovery', 'protocol', 'limits', 'mutation'])
    args = parser.parse_args()
    frozen()
    assert categories(b'AIL-RUNTIME-VALUE-LIMIT: bound') == ['VALUE-LIMIT']
    assert categories(b'AIL-RUNTIME-VALUE: bad') == ['VALUE']
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed'
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix='ail-adt-oracle-') as temporary:
        root = Path(temporary)
        if args.section in ('all', 'fixed', 'generated', 'mutation'):
            routes = build(root, RUNG / 'program.ail', 'tree', ('Analyze',))
            if args.section in ('all', 'fixed'):
                fixed(routes)
            if args.section in ('all', 'generated'):
                generated(routes)
            if args.section in ('all', 'mutation'):
                mutation(root, routes)
        for name, operation in [('leaves', leaves), ('generics', generics), ('recovery', recovery),
                                ('protocol', protocol), ('limits', limits)]:
            if args.section in ('all', name):
                operation(root)
    frozen()
    print(json.dumps({'seed': hex(SEED), 'checks': COUNTS['checks'], 'commands': harness.COUNTS['executions'],
                      'seconds': round(time.monotonic() - started, 2), 'frozen_fingerprint': FINGERPRINT}, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
