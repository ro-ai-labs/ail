#!/usr/bin/env python3
"""Independent caller-directory oracle. Build target/debug/ail first.

Only harness/compilation plumbing is reused from the process oracle. Directory
expectations are computed from Python fixture data, not either AIL backend.
Every subprocess has a finite deadline; race workers also have bounded lifetimes.
"""
import argparse
import ctypes
import json
import os
from pathlib import Path
import random
import resource
import select
import shutil
import signal
import socket
import subprocess
import tempfile
import threading
import time
import ail_process_differential as h

REPO, AIL, LIMIT = h.REPO, h.AIL, h.LIMIT
SEED = 0xD1EC701
COUNTS = h.COUNTS
op, seed, write = h.op, h.seed, h.write
invoke, exact, fatal = h.invoke, h.exact, h.fatal
program, package, routes = h.program, h.package, h.routes
BACKENDS = ['VM', 'native']


def canonical(value):
    if value is True:
        return 'true'
    if value is False:
        return 'false'
    if isinstance(value, str):
        return h.quoted(value)
    if isinstance(value, list):
        return '[' + ','.join(map(canonical, value)) + ']'
    if isinstance(value, dict):
        return '{' + ','.join(h.quoted(k) + ':' + canonical(v) for k, v in value.items()) + '}'
    return 'null' if value is None else str(value)


def directory(kind, **changes):
    operands = dict(resource='workspace', path='data.path')
    operands['source' if kind == 'WRITE_TEXT' else 'key'] = 'data.body' if kind == 'WRITE_TEXT' else 'data.result'
    return op('DIRECTORY_' + kind, **(operands | changes))


def output(key='data.result', type='Text'):
    return [op('JSON_FORMAT', key='data.encoded', source=key, type=type), write('data.encoded')]


def run(artifacts, backend, root=None, args=(), *, writes=None, input=b'', **kwargs):
    action = kwargs.pop('action', 'Run')
    prefix = [AIL, 'ail-vm', artifacts[0], '--action', action] if backend == 'VM' else [artifacts[1]]
    grants = [] if root is None else ['--read-directory=workspace=' + str(root)]
    if writes is not None:
        grants += ['--write-directory=workspace=' + str(writes)]
    return invoke(prefix + ['--stdio'] + grants + list(args), input, **kwargs)


def primitives(root, stem):
    return (
        program(root, stem + '-list', [directory('LIST_NAMES')] + output(type='List<Text>')),
        program(root, stem + '-read', [directory('READ_TEXT')] + output()),
        program(root, stem + '-write', [directory('WRITE_TEXT')]),
    )


SOURCE = r'''The application DirectoryTransport manages files.
A Data has:
- path: Text
- file: Text
- output: Text
- body: Text
- names: List<Text>
- encoded: Text
- separator: Text
- suffix: Text
Action: Run.
When Run happens:
- the system lists names in directory resource workspace at data.path into data.names
- the system formats data.names as JSON text into data.encoded
- the system writes text data.encoded to standard output
- the system assigns "\n" to data.separator
- the system writes text data.separator to standard output
- the system reads text from directory resource workspace at data.file into data.body
- the system assigns "\u0000λ\n" to data.suffix
- the system joins text data.body and data.suffix into data.body
- the system writes text data.body to directory resource reports at data.output
- the system formats data.body as JSON text into data.encoded
- the system writes text data.encoded to standard output
- the system records a trace event named Completed
'''


def transport(root, rng):
    artifacts = routes(root, package(root, 'directory-transport', SOURCE), 'directory-transport')
    names = ['z', 'A', '.hidden', 'a b=c', 'quote"', 'back\\slash', 'line\nbreak', '\x7f', '\x85', 'é', 'e\u0301', 'λ🙂', 'x' * 255]
    folder = root / 'transport input = λ'
    folder.mkdir()
    rng.shuffle(names)
    alphabet = ['a', 'Z', '\0', '\n', '\r', '\t', 'λ', '🙂', '漢', '\x7f', '\x85', '"', '\\', '=']
    payloads = {name: ''.join(rng.choice(alphabet) for _ in range(rng.randrange(70))) for name in names}
    for name, value in payloads.items():
        (folder / name).write_bytes(value.encode())
    (folder / 'sub').mkdir()
    (folder / 'link').symlink_to(folder / names[0])
    os.mkfifo(folder / 'fifo')
    ordered = sorted(names + ['sub', 'link', 'fifo'], key=lambda s: s.encode())
    for route_index, artifacts_pair in enumerate(artifacts):
        for backend in BACKENDS:
            destination = root / f'transport-output-{route_index}-{backend}'
            destination.mkdir()
            for name in names:
                transformed = payloads[name] + '\0λ\n'
                args = ['data.path=', 'data.file=' + name, 'data.output=computed report.json', '--write-directory=reports=' + str(destination)]
                wanted = (canonical(ordered) + '\n' + canonical(transformed)).encode()
                exact(run(artifacts_pair, backend, folder, args), wanted)
                assert (destination / 'computed report.json').read_bytes() == transformed.encode()
                assert (folder / name).read_bytes() == payloads[name].encode()
    print('PASS: exact sorted all-entry Unicode/control listing and discovered-file transform across 12 routes', flush=True)


def paths(root):
    listing, reading, writing = primitives(root, 'paths')
    folder, outside = root / 'paths-root', root / 'outside'
    folder.mkdir(); outside.mkdir()
    (outside / 'sentinel').write_bytes(b'OUTSIDE MUST SURVIVE')
    (folder / 'sub').mkdir(); (folder / 'sub' / 'λ').mkdir()
    (folder / 'sub' / 'λ' / 'value').write_bytes(b'inside\0\r\n')
    for name in ['..part', '%2e%2e', '~', 'a\\b', 'C:part', 'full／slash']:
        (folder / name).write_bytes(name.encode())
    (folder / 'out').symlink_to(outside, target_is_directory=True)
    (folder / 'in').symlink_to(folder / 'sub', target_is_directory=True)
    (folder / 'final').symlink_to(outside / 'sentinel')
    (folder / 'dangling').symlink_to(folder / 'absent')
    (folder / 'cycle').symlink_to('cycle')
    os.mkfifo(folder / 'fifo')
    # Reach the exact relative-path bound even though the absolute joined path
    # would exceed PATH_MAX: create using dirfds, as callers are allowed to do.
    segment = 'p' * 255
    long_path = '/'.join([segment] * 16)
    assert len(long_path.encode()) == 4095
    parent_fd = os.open(folder, os.O_RDONLY | os.O_DIRECTORY)
    try:
        for _ in range(15):
            os.mkdir(segment, dir_fd=parent_fd)
            child_fd = os.open(segment, os.O_RDONLY | os.O_DIRECTORY, dir_fd=parent_fd)
            os.close(parent_fd); parent_fd = child_fd
        leaf_fd = os.open(segment, os.O_CREAT | os.O_WRONLY, 0o600, dir_fd=parent_fd)
        try:
            os.write(leaf_fd, b'boundary')
        finally:
            os.close(leaf_fd)
    finally:
        os.close(parent_fd)
    sock = socket.socket(socket.AF_UNIX)
    try:
        sock.bind(str(folder / 'socket'))
        for backend in BACKENDS:
            exact(run(reading, backend, folder, ['data.path=' + long_path]), b'"boundary"')
            exact(run(writing, backend, args=['data.path=' + long_path, 'data.body=updated'], writes=folder))
            exact(run(reading, backend, folder, ['data.path=' + long_path]), b'"updated"')
            exact(run(writing, backend, args=['data.path=' + long_path, 'data.body=boundary'], writes=folder))
            exact(run(listing, backend, folder, ['data.path=sub/λ']), b'["value"]')
            exact(run(reading, backend, folder, ['data.path=sub/λ/value']), canonical('inside\0\r\n').encode())
            exact(run(writing, backend, args=['data.path=sub/λ/new', 'data.body=new'], writes=folder))
            assert (folder / 'sub' / 'λ' / 'new').read_bytes() == b'new'
            (folder / 'sub' / 'λ' / 'new').unlink()
            for name in ['..part', '%2e%2e', '~', 'a\\b', 'C:part', 'full／slash']:
                exact(run(reading, backend, folder, ['data.path=' + name], cwd=outside), canonical(name).encode())
            invalid = ['', '/sentinel', '../outside/sentinel', 'sub/../final', './final', 'sub//λ/value', 'sub/λ/',
                       'out/sentinel', 'in/λ/value', 'final', 'dangling', 'cycle', 'fifo', 'socket', 'sub', 'absent', 'x' * 4096]
            for value in invalid:
                fatal(run(reading, backend, folder, ['data.path=' + value]), 'PATH')
                if value != 'absent':
                    fatal(run(writing, backend, args=['data.path=' + value, 'data.body=BAD'], writes=folder), 'PATH')
            assert (outside / 'sentinel').read_bytes() == b'OUTSIDE MUST SURVIVE'
            for absent_root in [folder / 'missing', folder / '..part']:
                fatal(run(listing, backend, absent_root, ['data.path=']), 'PATH')
        rootlink = root / 'caller-root-link'
        rootlink.symlink_to(folder, target_is_directory=True)
        os.link(outside / 'sentinel', folder / 'hard')
        fixed = program(root, 'fixed-symlink-compatibility', [op('FILE_READ_TEXT', resource='file', key='data.result')] + output())
        for backend in BACKENDS:
            exact(run(reading, backend, rootlink, ['data.path=..part']), canonical('..part').encode())
            exact(run(reading, backend, folder, ['data.path=hard']), canonical('OUTSIDE MUST SURVIVE').encode())
            exact(run(writing, backend, args=['data.path=hard', 'data.body=hardlinked'], writes=folder))
            assert (outside / 'sentinel').read_bytes() == b'hardlinked'
            exact(run(fixed, backend, args=['--read-file=file=' + str(folder / 'final')]), canonical('hardlinked').encode())
            (outside / 'sentinel').write_bytes(b'OUTSIDE MUST SURVIVE')
    finally:
        sock.close()
    nul = program(root, 'nul-path', [seed('data.path', 'bad\0path'), directory('READ_TEXT')])
    for backend in BACKENDS:
        fatal(run(nul, backend, folder), 'PATH')
    fatal(run(reading, 'native', folder, [b'data.path=\xff']), 'TEXT')
    fatal(run(writing, 'native', args=[b'data.path=\xff', b'data.body=\xff']), 'CAPABILITY')
    fatal(run(writing, 'native', args=['data.path=../outside', b'data.body=\xff'], writes=folder), 'PATH')
    fatal(run(writing, 'native', args=['data.path=missing-parent/file', b'data.body=\xff'], writes=folder), 'TEXT')
    for backend in BACKENDS:
        fatal(run(reading, backend, folder / 'missing-root'), 'TEXT')
        fatal(run(writing, backend, args=['data.path=file'], writes=folder / 'missing-root'), 'TEXT')
    print('PASS: nested/literal paths, traversal and every relative symlink class, nonregular targets, root symlink and hard-link boundary, fixed-file compatibility', flush=True)


def authority(root):
    listing, reading, writing = primitives(root, 'authority')
    first, second = root / 'authority-first', root / 'authority second=λ'
    first.mkdir(); second.mkdir()
    (first / 'value').write_text('first'); (second / 'value').write_text('second')
    sideeffect = program(root, 'authority-before-effects', [seed('data.body', 'bad'), seed('data.path', 'marker'), directory('WRITE_TEXT')])
    silent_variants = [program(root, 'unused-legacy', [seed('data.status', 'ok')]), program(root, 'unused-scalar', [op('EVAL_INT', key='data.status', expression='i:1')])]
    for backend in BACKENDS:
        for prefix in ['--read-directory', '--write-directory']:
            malformed = [prefix, prefix + '=', prefix + '=workspace', prefix + '==/tmp', prefix + '=workspace=',
                         prefix + '=workspace=relative', prefix + '=bad-name=/tmp', prefix + '=bad..name=/tmp',
                         prefix + '-extra=workspace=/tmp', prefix + '=workspace=/' + 'x' * 4095, prefix + '=' + 'x' * 257 + '=/tmp']
            for flag in malformed:
                fatal(run(sideeffect, backend, args=[flag], writes=first), 'CAPABILITY')
            fatal(run(sideeffect, backend, args=[prefix + '=workspace=relative', prefix + '=workspace=' + str(first)], writes=first), 'CAPABILITY')
            for silent in silent_variants:
                binary_prefix = [AIL, 'ail-vm', silent[0], '--action', 'Run'] if backend == 'VM' else [silent[1]]
                result = invoke(binary_prefix + [prefix + '=unused=/' + 'x' * 4094])
                assert result.returncode == 0, result.stderr
                assert b'x' * 128 not in result.stdout + result.stderr
                fatal(run(silent, backend, args=[prefix + '=unused=relative']), 'CAPABILITY')
        assert not (first / 'marker').exists()
        fatal(run(reading, backend, args=['data.path=value'], writes=first), 'CAPABILITY')
        fatal(run(writing, backend, first, ['data.path=value', 'data.body=BAD']), 'CAPABILITY')
        for spoof in ['workspace=' + str(first), 'read-directory=workspace=' + str(first), '--read-file=workspace=' + str(first), '--run-process=workspace=/bin/true']:
            fatal(run(listing, backend, args=['data.path=', spoof]), 'CAPABILITY')
        exact(run(reading, backend, first, ['data.path=value', '--read-directory=workspace=' + str(second)]), b'"second"')
        exact(run(reading, backend, args=['data.path=value', '--read-directory=unused=/missing', '--read-directory=workspace=/missing', '--read-directory=workspace=' + str(first)]), b'"first"')
        exact(run(writing, backend, first, ['data.path=value', 'data.body=changed'], writes=second))
        assert (first / 'value').read_text() == 'first'
        assert (second / 'value').read_text() == 'changed'
        (second / 'value').write_text('second')
    prohibited = root / 'unsupported-grant.elf'
    result = invoke([AIL, 'ail-compile', listing[0], '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', prohibited,
                     '--read-directory=workspace=' + str(first)])
    COUNTS['cases'] += 1
    assert result.returncode != 0 and b'AIL-RUNTIME-CAPABILITY' in result.stdout + result.stderr
    assert not prohibited.exists()
    print('PASS: directional authority, all malformed/unused/overridden bindings, independent read/write roots and legacy/scalar grant privacy', flush=True)


def limits(root):
    listing, reading, writing = primitives(root, 'limits')
    reading = program(root, 'limit-raw-read', [directory('READ_TEXT'), write('data.result')])
    folder = root / 'text-limits'; folder.mkdir()
    for name, data in [('exact', b'x' * LIMIT), ('over', b'\xff' + b'x' * LIMIT), ('bad', b'a\xff'), ('empty', b'')]:
        (folder / name).write_bytes(data)
    (folder / 'sentinel').write_bytes(b'unchanged')
    oversized_write = program(root, 'oversized-write', [seed('data.path', 'sentinel'), seed('data.body', 'x' * (LIMIT + 1)), directory('WRITE_TEXT')])
    oversized_path = program(root, 'oversized-path', [seed('data.path', 'x' * (LIMIT + 1)), directory('READ_TEXT')])
    for backend in BACKENDS:
        exact(run(reading, backend, folder, ['data.path=exact']), b'x' * LIMIT)
        fatal(run(reading, backend, folder, ['data.path=over']), 'TEXT-LIMIT')
        fatal(run(reading, backend, folder, ['data.path=bad']), 'TEXT')
        fatal(run(oversized_write, backend, writes=folder), 'TEXT-LIMIT')
        fatal(run(oversized_path, backend, folder), 'TEXT-LIMIT')
        assert (folder / 'sentinel').read_bytes() == b'unchanged'
    fatal(run(writing, 'native', args=['data.path=sentinel', b'data.body=\xff'], writes=folder), 'TEXT')
    assert (folder / 'sentinel').read_bytes() == b'unchanged'
    names_root = root / 'canonical-bound'; names_root.mkdir()
    names = [f'{i:04x}-' + 'x' * 248 for i in range(4096)]
    names[0] = names[0][:-1]
    for name in names:
        (names_root / name).touch()
    encoded = canonical(sorted(names)).encode(); assert len(encoded) == LIMIT
    for backend in BACKENDS:
        exact(run(listing, backend, names_root, ['data.path=']), encoded)
    (names_root / names[0]).rename(names_root / (names[0] + 'x'))
    for backend in BACKENDS:
        fatal(run(listing, backend, names_root, ['data.path=']), 'LIST-LIMIT')
    (names_root / (names[0] + 'x')).rename(names_root / names[0])
    raw_root = root / 'raw-bound'; raw_root.mkdir()
    for i in range(4112):
        name = f'{i:04x}-'.encode() + b'x' * 250
        if i == 0:
            name = name[:-1] + b'\xff'
        os.close(os.open(os.fsencode(raw_root) + b'/' + name, os.O_CREAT | os.O_WRONLY, 0o600))
    (raw_root / ('r' * 16)).touch()
    assert 4112 * 255 + 16 == LIMIT
    for backend in BACKENDS:
        fatal(run(listing, backend, raw_root, ['data.path=']), 'TEXT')
    (raw_root / ('r' * 16)).rename(raw_root / ('r' * 17))
    for backend in BACKENDS:
        fatal(run(listing, backend, raw_root, ['data.path=']), 'LIST-LIMIT')
    count_root = root / 'count-bound'; count_root.mkdir()
    for i in range(65536):
        (count_root / f'{i:05x}').touch()
    expected = canonical([f'{i:05x}' for i in range(65536)]).encode()
    for backend in BACKENDS:
        exact(run(listing, backend, count_root, ['data.path=']), expected)
    badname = os.fsencode(count_root) + b'/invalid\xff'
    os.close(os.open(badname, os.O_CREAT | os.O_WRONLY, 0o600))
    for backend in BACKENDS:
        fatal(run(listing, backend, count_root, ['data.path=']), 'LIST-LIMIT')
    (count_root / '00000').unlink()
    for backend in BACKENDS:
        fatal(run(listing, backend, count_root, ['data.path=']), 'TEXT')
    expansion = root / 'escaped-bound'; expansion.mkdir()
    for i in range(900):
        (expansion / (f'{i:04x}' + '\x7f' * 250)).touch()
    for backend in BACKENDS:
        fatal(run(listing, backend, expansion, ['data.path=']), 'LIST-LIMIT')
    print('PASS: exact Text, canonical/raw-name/item listing limits, invalid UTF-8 ordering and control-escape expansion', flush=True)
    return names_root, folder


def recovery(root, names_root, text_root):
    # No formatting or temporary result construction is hidden in these fills.
    fill_list = [seed('data.path', '')] + [directory('LIST_NAMES')] * 64
    fill_text = [seed('data.path', 'exact')] + [directory('READ_TEXT')] * 64
    success_list = program(root, 'exact-list-arena', fill_list + [seed('data.done', 'exact'), write('data.done')])
    excess_list = program(root, 'excess-list-arena', fill_list + [directory('LIST_NAMES'), seed('data.done', 'wrong'), write('data.done')])
    success_text = program(root, 'exact-text-arena', fill_text + [seed('data.path', 'copied'), directory('WRITE_TEXT', source='data.result'), seed('data.done', 'exact'), write('data.done')])
    excess_text = program(root, 'excess-text-arena', fill_text + [seed('data.path', 'empty'), directory('READ_TEXT')])
    for backend in BACKENDS:
        exact(run(success_list, backend, names_root, timeout=30), b'exact')
        fatal(run(excess_list, backend, names_root, timeout=30), 'LIST-LIMIT')
        exact(run(success_text, backend, text_root, writes=text_root, timeout=30), b'exact')
        assert (text_root / 'copied').stat().st_size == LIMIT
        fatal(run(excess_text, backend, text_root, timeout=30), 'TEXT-LIMIT')
    alias = program(root, 'aliases', [seed('data.path', 'empty'), op('COPY_FIELD', key='data.old', source='data.path', text='borrow path'),
        directory('READ_TEXT', key='data.path'), *output('data.path'), *output('data.old'), seed('data.path', 'same'), directory('WRITE_TEXT', source='data.path')])
    for backend in BACKENDS:
        exact(run(alias, backend, text_root, writes=text_root), b'"""empty"')
        assert (text_root / 'same').read_bytes() == b'same'
    for category, path in [('PATH', 'missing'), ('TEXT', 'bad')]:
        child = {'action': 'Child', 'instructions': [directory('READ_TEXT'), op('RETURN_SUCCESS')]}
        body = [seed('data.path', path), seed('data.result', 'retained'), op('TRY_BEGIN', errors=category, key='data.failure'),
                op('CALL_ACTION', target='Child'), op('RECOVER'), *output(), *output('data.failure'), op('TRY_END')]
        artifact = program(root, 'recover-' + category, body, [child])
        for backend in BACKENDS:
            exact(run(artifact, backend, text_root), ('"retained"' + canonical('AIL-RUNTIME-' + category)).encode())
    recovery_fill = program(root, 'failure-no-charge', [seed('data.path', 'bad'), op('TRY_BEGIN', errors='any', key='data.failure'), directory('READ_TEXT'),
        op('RECOVER'), op('TRY_END')] + fill_text + [seed('data.done', 'uncharged'), write('data.done')])
    for backend in BACKENDS:
        exact(run(recovery_fill, backend, text_root, timeout=30), b'uncharged')
    invalid_names = root / 'invalid-list-recovery'; invalid_names.mkdir()
    os.close(os.open(os.fsencode(invalid_names) + b'/bad\xff', os.O_CREAT | os.O_WRONLY, 0o600))
    failed_list_fill = program(root, 'list-failure-no-charge', [seed('data.path', ''), seed('data.result', '["retained"]'),
        op('TRY_BEGIN', errors='TEXT', key='data.failure'), directory('LIST_NAMES', resource='bad'), op('RECOVER'),
        *output(type='List<Text>'), op('TRY_END')] + fill_list + [seed('data.done', 'uncharged'), write('data.done')])
    child = {'action': 'Child', 'instructions': [directory('READ_TEXT'), op('RETURN_SUCCESS')]}
    called_fill = [seed('data.path', 'exact')] + [op('CALL_ACTION', target='Child')] * 64
    called_exact = program(root, 'called-arena-exact', called_fill + [seed('data.done', 'shared'), write('data.done')], [child])
    called_excess = program(root, 'called-arena-excess', called_fill + [seed('data.path', 'empty'), directory('READ_TEXT')], [child])
    mixed_fill = [seed('data.path', '')] + [directory('LIST_NAMES')] * 63
    mixed_fill += [seed('data.empty', '[]'), seed('data.item', 'x' * (LIMIT - 4)),
                   op('LIST_APPEND', key='data.result', source='data.empty', type='Text', value='data.item')]
    mixed_exact = program(root, 'mixed-list-arena-exact', mixed_fill + [seed('data.done', 'mixed'), write('data.done')])
    mixed_excess = program(root, 'mixed-list-arena-excess', mixed_fill + [directory('LIST_NAMES')])
    for backend in BACKENDS:
        exact(run(failed_list_fill, backend, names_root, ['--read-directory=bad=' + str(invalid_names)], timeout=30), b'["retained"]uncharged')
        exact(run(called_exact, backend, text_root, timeout=30), b'shared')
        fatal(run(called_excess, backend, text_root, timeout=30), 'TEXT-LIMIT')
        exact(run(mixed_exact, backend, names_root, timeout=30), b'mixed')
        fatal(run(mixed_excess, backend, names_root, timeout=30), 'LIST-LIMIT')
    repeat = [seed('data.path', 'bad')]
    repeat += [op('TRY_BEGIN', errors='TEXT', key='data.failure'), directory('READ_TEXT'), op('RECOVER'), op('TRY_END')] * 200
    repeat += [seed('data.path', 'empty')] + [directory('READ_TEXT')] * 200 + [seed('data.done', 'closed'), write('data.done')]
    constrained = program(root, 'fd-cleanup', repeat)
    def restrict():
        resource.setrlimit(resource.RLIMIT_NOFILE, (16, 16))
        resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024, 256 * 1024 * 1024))
    for backend in BACKENDS:
        exact(run(constrained, backend, text_root, prepare=restrict, timeout=30), b'closed')
    print('PASS: exact shared arenas, failed-operation non-charge/atomic state, aliases, caller recovery and repeated cleanup under limits', flush=True)


def lifecycle(root):
    folder = root / 'lifecycle'; folder.mkdir()
    (folder / 'value').write_text('before')
    artifact = program(root, 'reopen-root', [seed('data.path', 'value'), directory('READ_TEXT'), seed('data.ready', 'READY\n'), write('data.ready'),
        op('STDIN_READ_TEXT', key='data.barrier'), directory('READ_TEXT'), write('data.result')])
    for backend in BACKENDS:
        prefix = [AIL, 'ail-vm', artifact[0], '--action', 'Run'] if backend == 'VM' else [artifact[1]]
        COUNTS['executions'] += 1
        child = subprocess.Popen(prefix + ['--stdio', '--read-directory=workspace=' + str(folder)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        try:
            deadline = time.monotonic() + 5
            marker = b''
            while len(marker) < 6:
                remaining = deadline - time.monotonic()
                assert remaining > 0 and select.select([child.stdout], [], [], remaining)[0], 'root-reopen barrier timed out'
                chunk = os.read(child.stdout.fileno(), 6 - len(marker))
                assert chunk, 'root-reopen child exited before barrier'
                marker += chunk
            assert marker == b'READY\n'
            parked = root / ('parked-' + backend)
            folder.rename(parked); folder.mkdir(); (folder / 'value').write_text('after')
            out, err = child.communicate(b'continue', timeout=5)
            exact(subprocess.CompletedProcess(prefix, child.returncode, out, err), b'after')
        finally:
            if child.poll() is None:
                h.terminate_session(child.pid); h.terminate_tree(child.pid); child.communicate(timeout=3)
            h.reap_adopted(False)
    partial = program(root, 'partial-write', [seed('data.path', 'partial'), seed('data.body', 'abcdefghij'), directory('WRITE_TEXT')])
    def short_file():
        signal.signal(signal.SIGXFSZ, signal.SIG_IGN)
        resource.setrlimit(resource.RLIMIT_FSIZE, (3, 3))
    for backend in BACKENDS:
        fatal(run(partial, backend, writes=folder, prepare=short_file), 'IO')
        assert (folder / 'partial').read_bytes() == b'abc'
    prior = program(root, 'prior-effect', [seed('data.path', 'first'), seed('data.body', 'persistent'), directory('WRITE_TEXT'), seed('data.path', '../outside'), directory('WRITE_TEXT')])
    for backend in BACKENDS:
        fatal(run(prior, backend, writes=folder), 'PATH')
        assert (folder / 'first').read_bytes() == b'persistent'
    outside, inside = root / 'race-outside', folder / 'moving'
    outside.mkdir(); inside.mkdir()
    (outside / 'value').write_text('OUTSIDE'); (inside / 'value').write_text('inside')
    race = program(root, 'bounded-race', [seed('data.path', 'moving/value'), seed('data.body', 'safe')] +
        [op('TRY_BEGIN', errors='PATH', key='data.failure'), directory('WRITE_TEXT'), op('RECOVER'), op('TRY_END')] * 250)
    for backend in BACKENDS:
        stop = threading.Event()
        failures = []
        parked = folder / 'parked'
        def swap():
            try:
                for _ in range(4000):
                    if stop.is_set():
                        break
                    inside.rename(parked)
                    inside.symlink_to(outside, target_is_directory=True)
                    inside.unlink()
                    parked.rename(inside)
            except BaseException as error:
                failures.append(error)
            finally:
                if inside.is_symlink():
                    inside.unlink()
                if parked.exists() and not inside.exists():
                    parked.rename(inside)
        worker = threading.Thread(target=swap, daemon=True); worker.start()
        try:
            result = run(race, backend, writes=folder, timeout=15)
            if result.returncode == 0:
                exact(result)
            else:
                fatal(result, 'IO')  # openat2 EAGAIN is deliberately fatal.
        finally:
            stop.set(); worker.join(timeout=3)
        assert not worker.is_alive() and not failures, failures
        assert (outside / 'value').read_bytes() == b'OUTSIDE'
    print('PASS: per-operation root reopening, real partial write, earlier effects retained and bounded relative-component race stress', flush=True)


TRIM = ''.join(chr(i) for i in list(range(9, 14)) + list(range(28, 33)) + [0x85, 0xa0, 0x1680] + list(range(0x2000, 0x200b)) + [0x2028, 0x2029, 0x202f, 0x205f, 0x3000])
class Pairs(list):
    pass
class Number(str):
    pass


def report_for(text):
    errors = {f'AIL-RUNTIME-{key}': 0 for key in ['DIVZERO', 'JSON', 'JSON-PATH', 'OVERFLOW']}
    values = []; count = 0
    def reject_constant(_):
        raise ValueError('non-JSON number')
    def validate(value):
        if isinstance(value, str):
            value.encode('utf-8')
        elif isinstance(value, Pairs):
            for key, child in value:
                key.encode('utf-8'); validate(child)
        elif isinstance(value, list):
            for child in value:
                validate(child)
    for line in text.split('\n'):
        line = line.strip(TRIM)
        if not line:
            continue
        count += 1; error = None; operands = []
        try:
            value = json.loads(line, object_pairs_hook=Pairs, parse_int=Number, parse_float=Number, parse_constant=reject_constant)
            validate(value)
        except (ValueError, UnicodeError):
            error = 'JSON'
        if error is None:
            for key in ['value', 'divisor']:
                selected = [v for k, v in value if k == key] if isinstance(value, Pairs) else []
                if len(selected) != 1:
                    error = 'JSON-PATH'; break
                item = selected[0]
                if not isinstance(item, Number) or any(c in item for c in '.eE') or not -(1 << 63) <= int(item) < (1 << 63):
                    error = 'JSON'; break
                operands.append(int(item))
        if error is None:
            left, right = operands
            if right == 0:
                error = 'DIVZERO'
            elif left == -(1 << 63) and right == -1:
                error = 'OVERFLOW'
            else:
                values.append(abs(left) // abs(right) * (-1 if (left < 0) != (right < 0) else 1))
        if error:
            errors['AIL-RUNTIME-' + error] += 1
    return {'rows': count, 'accepted': len(values), 'rejected': count - len(values), 'values': sorted(values), 'errors': errors}


def workload(root, rng):
    artifacts = routes(root, REPO / 'examples/directory-report', 'directory-report', 'Generate')
    entries = {'.hidden': b'', 'bad-utf8': b'\xff', 'z-good': b'{"value":9,"divisor":2}\n', 'λ\n=quoted"': b'{"value":-7,"divisor":3}\r\n'}
    for i in range(24):
        rows = [canonical({'value': rng.randrange(-10000, 10000), 'divisor': rng.choice([-7, -1, 1, 3, 19])}) for _ in range(rng.randrange(1, 12))]
        rows += ['{', '{}', '{"value":4,"divisor":0}', '{"value":-9223372036854775808,"divisor":-1}', '{"value":1,"value":2,"divisor":1}', '\x85', '{"value":true,"divisor":1}', '{"value":1.0,"divisor":1}']
        rng.shuffle(rows)
        entries[f'item-{i:02}-é'] = '\n'.join(rows).encode()
    entries['long-' + 'x' * 250] = b'{"value":-9223372036854775808,"divisor":1}'
    names = sorted(list(entries) + ['directory', 'symlink', 'fifo'], key=lambda s: s.encode())
    wanted_files = {}; manifest = {'entries': []}
    for i, name in enumerate(names):
        report_name, error = '', ''
        if name not in entries:
            error = 'AIL-RUNTIME-PATH'
        else:
            try:
                value = entries[name].decode('utf-8')
            except UnicodeError:
                error = 'AIL-RUNTIME-TEXT'
            else:
                report_name = f'result-{i}.json'
                wanted_files[report_name] = canonical(report_for(value)).encode()
        manifest['entries'].append({'name': name, 'report': report_name, 'error': error})
    wanted_files['manifest.json'] = canonical(manifest).encode()
    for route_index, artifact in enumerate(artifacts):
        for backend in BACKENDS:
            folder, destination = root / f'batch-in-{route_index}-{backend}', root / f'batch-out-{route_index}-{backend}'
            folder.mkdir(); destination.mkdir()
            for name, value in reversed(list(entries.items())):
                (folder / name).write_bytes(value)
            (folder / 'directory').mkdir(); (folder / 'symlink').symlink_to(folder / 'z-good'); os.mkfifo(folder / 'fifo')
            result = run(artifact, backend, folder, ['--write-directory=reports=' + str(destination)], action='Generate', timeout=30)
            exact(result)
            actual = {path.name: path.read_bytes() for path in destination.iterdir()}
            assert actual == wanted_files, (backend, route_index, actual.keys(), wanted_files.keys(), [(name, actual.get(name, b'')[:300], value[:300]) for name, value in wanted_files.items() if actual.get(name) != value][:3])
            assert all((folder / name).read_bytes() == value for name, value in entries.items())
    supplied = REPO / 'examples/directory-report/input'
    supplied_names = sorted((path.name for path in supplied.iterdir()), key=lambda name: name.encode())
    supplied_files = {}; supplied_manifest = {'entries': []}
    for i, name in enumerate(supplied_names):
        filename, error = '', ''
        path = supplied / name
        if not path.is_file():
            error = 'AIL-RUNTIME-PATH'
        else:
            try:
                content = path.read_bytes().decode()
            except UnicodeError:
                error = 'AIL-RUNTIME-TEXT'
            else:
                filename = f'result-{i}.json'
                supplied_files[filename] = canonical(report_for(content)).encode()
        supplied_manifest['entries'].append({'name': name, 'report': filename, 'error': error})
    assert len(supplied_names) == 5 and len(supplied_files) == 3
    supplied_files['manifest.json'] = canonical(supplied_manifest).encode()
    for route_index, artifact in enumerate(artifacts):
        for backend in BACKENDS:
            folder, destination = root / f'documented-in-{route_index}-{backend}', root / f'documented-out-{route_index}-{backend}'
            shutil.copytree(supplied, folder, symlinks=True); destination.mkdir()
            exact(run(artifact, backend, folder, ['--write-directory=reports=' + str(destination)], action='Generate', timeout=30))
            assert {path.name: path.read_bytes() for path in destination.iterdir()} == supplied_files
    failed_manifest = json.loads(canonical(supplied_manifest))
    failed_manifest['entries'][0]['report'] = ''
    failed_manifest['entries'][0]['error'] = 'AIL-RUNTIME-PATH'
    failed_files = supplied_files | {'manifest.json': canonical(failed_manifest).encode()}
    del failed_files['result-0.json']
    for backend in BACKENDS:
        destination = root / ('failed-report-out-' + backend); destination.mkdir()
        blocked = destination / 'result-0.json'; blocked.mkdir()
        exact(run(artifacts[0], backend, supplied, ['--write-directory=reports=' + str(destination)], action='Generate', timeout=30))
        assert blocked.is_dir() and not list(blocked.iterdir())
        assert {path.name: path.read_bytes() for path in destination.iterdir() if path.is_file()} == failed_files
    print('PASS: seeded and documented discovered-workspace reports/manifests across 12 routes, plus per-entry failed-write continuation', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--section', choices=['all', 'transport', 'paths', 'authority', 'limits', 'recovery', 'lifecycle', 'workload'], default='all')
    args = parser.parse_args()
    assert ctypes.CDLL(None).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup failed'
    try:
        with tempfile.TemporaryDirectory(prefix='ail-directory-oracle-') as temporary:
            root = Path(temporary); rng = random.Random(SEED)
            if args.section in ['all', 'transport']: transport(root, rng)
            if args.section in ['all', 'paths']: paths(root)
            if args.section in ['all', 'authority']: authority(root)
            if args.section in ['all', 'limits', 'recovery']:
                listing_root, text_root = limits(root)
                if args.section in ['all', 'recovery']: recovery(root, listing_root, text_root)
            if args.section in ['all', 'lifecycle']: lifecycle(root)
            if args.section in ['all', 'workload']: workload(root, rng)
    finally:
        h.reap_adopted(False)
    print(f"PASS: directory oracle seed={SEED:#x}; {COUNTS['cases']} assertions, {COUNTS['executions']} bounded commands", flush=True)


if __name__ == '__main__':
    main()
