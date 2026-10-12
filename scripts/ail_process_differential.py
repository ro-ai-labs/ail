#!/usr/bin/env python3
"""Independent Linux process-resource oracle; build target/debug/ail first.

The C helper is a test fixture, never a generated-program dependency. Expectations
come from Python and the fixture contract, not from either AIL backend. Every
command has an outer deadline. A subreaper and /proc descendant cleanup cover
private process groups as well as the immediate command's session.
"""
import argparse
import ctypes
import json
import os
from pathlib import Path
import random
import resource
import shutil
import signal
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576
SEED = 0xA11C011
COUNTS = {"executions": 0, "cases": 0}


def quoted(value):
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + ''.join(escapes[c] if c in escapes else f'\\u{ord(c):04x}' if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value) + '"'


def canonical(value):
    if isinstance(value, str):
        return quoted(value)
    if isinstance(value, list):
        return '[' + ','.join(map(canonical, value)) + ']'
    if isinstance(value, dict):
        return '{' + ','.join(quoted(k) + ':' + canonical(v) for k, v in value.items()) + '}'
    if value is None:
        return 'null'
    return str(value)


def children(pid):
    try:
        return [int(p) for p in Path(f'/proc/{pid}/task/{pid}/children').read_text().split()]
    except FileNotFoundError:
        return []


def terminate_tree(pid):
    # The language engine deliberately gives each child its own process group.
    # Enumerate before killing its parent, then kill group and individual IDs.
    for child in children(pid):
        terminate_tree(child)
    try:
        os.killpg(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def terminate_session(session):
    # The session survives its leader. Find private child groups even after the
    # AIL parent exited and its descendants were adopted by this subreaper.
    members = []
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / 'stat').read_text().rsplit(')', 1)[1].split()
            if int(fields[3]) == session:
                members.append((int(entry.name), int(fields[2])))
        except (FileNotFoundError, ProcessLookupError):
            continue
    for group in {group for _, group in members}:
        try:
            os.killpg(group, signal.SIGKILL)
        except ProcessLookupError:
            pass
    for pid, _ in members:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def reap_adopted(check=True):
    deadline = time.monotonic() + .25
    while children(os.getpid()) and time.monotonic() < deadline:
        for pid in children(os.getpid()):
            try:
                os.waitpid(pid, os.WNOHANG)
            except ChildProcessError:
                pass
        time.sleep(.002)
    survivors = children(os.getpid())
    for pid in survivors:
        terminate_tree(pid)
    for pid in survivors:
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass
    if check:
        assert not survivors, f'AIL left live child processes after returning: {survivors}'


def invoke(arguments, input=b'', *, cwd=REPO, pass_fds=(), prepare=None, timeout=15):
    COUNTS['executions'] += 1
    process = subprocess.Popen(arguments, cwd=cwd, stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               start_new_session=True, pass_fds=pass_fds, preexec_fn=prepare)
    try:
        out, err = process.communicate(input, timeout=timeout)
    except BaseException:
        terminate_session(process.pid)
        terminate_tree(process.pid)
        try:
            process.communicate(timeout=3)
        finally:
            reap_adopted(False)
        raise
    reap_adopted()
    return subprocess.CompletedProcess(arguments, process.returncode, out, err)


def checked(arguments):
    result = invoke(arguments, timeout=45)
    assert result.returncode == 0, (arguments[:5], result.stdout[:1000], result.stderr[-3000:])
    return result.stdout


def exact(result, stdout=b'', stderr=b''):
    COUNTS['cases'] += 1
    assert result.returncode == 0, (result.returncode, result.stdout[:500], result.stderr[-2500:])
    assert result.stdout == stdout, (len(result.stdout), result.stdout[:1000], len(stdout), stdout[:1000])
    assert result.stderr == stderr, (result.stderr[:2000], stderr[:2000])


def fatal(result, code):
    COUNTS['cases'] += 1
    assert result.returncode > 0, (result.returncode, result.stdout[:500], result.stderr[-2000:])
    assert result.stdout == b'', result.stdout[:1000]
    assert f'AIL-RUNTIME-{code}:'.encode() in result.stderr, result.stderr[-2000:]


def op(opcode, **operands):
    return {'opcode': opcode, 'operands': operands}


def seed(key, value):
    return op('SET_FIELD', key=key, value=value, text='independent process fixture')


def write(source):
    return op('STDOUT_WRITE_TEXT', source=source)


def process_op(**overrides):
    fields = dict(resource='tool', arguments='data.arguments', input='data.input',
                  timeout_ms='data.timeout', stdout='data.output', stderr='data.errors', status='data.status')
    return op('PROCESS_RUN', **(fields | overrides))


def program(root, name, instructions, routines=()):
    artifact = root / f'{name}.json'
    artifact.write_text(json.dumps({'kind': 'AIL-Bytecode', 'package': 'process-oracle', 'version': '1',
        'profile': 'Application', 'capability_grants': [], 'target_support': {}, 'external_bindings': [],
        'failures': [], 'actions': [{'action': 'Run', 'instructions': instructions + [op('RETURN_SUCCESS')]}] + list(routines)}))
    binary = root / f'{name}.elf'
    checked([AIL, 'ail-compile', artifact, '--action', 'Run', '--target', 'linux-x86_64-elf', '--out', binary])
    return artifact, binary


def run(artifacts, backend, helper=None, arguments=(), input=b'', *, action='Run', **kwargs):
    prefix = [AIL, 'ail-vm', artifacts[0], '--action', action] if backend == 'VM' else [artifacts[1]]
    grants = [] if helper is None else ['--run-process=tool=' + str(helper)]
    return invoke(prefix + ['--stdio'] + grants + list(arguments), input, **kwargs)


def initial(arguments=(), input='', timeout='2000'):
    return [seed('data.arguments', canonical(list(arguments))), seed('data.input', input), seed('data.timeout', timeout)]


def report():
    return [op('JSON_FORMAT', key='data.encoded', source='data.output', type='Text'), write('data.encoded'),
            seed('data.separator', '|'), write('data.separator'),
            op('JSON_FORMAT', key='data.encoded', source='data.errors', type='Text'), write('data.encoded'),
            write('data.separator'), op('JSON_FORMAT', key='data.encoded', source='data.status', type='Int'), write('data.encoded')]


def expected(out='', err='', status=0):
    return (quoted(out) + '|' + quoted(err) + '|' + str(status)).encode()


FIXTURE = r'''
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
extern char **environ;
static void put(int fd, const void *data, size_t len) {
    const char *p=data;
    while(len) { ssize_t n=write(fd,p,len); if(n<0&&errno==EINTR)continue; if(n<=0)_exit(90); p+=n;len-=n; }
}
static void fill(int fd, char c, size_t len) {
    char b[4096];memset(b,c,sizeof b);while(len){size_t n=len>sizeof b?sizeof b:len;put(fd,b,n);len-=n;}
}
static void echo(void) {
    char b[4096];for(;;){ssize_t n=read(0,b,sizeof b);if(n<0&&errno==EINTR)continue;if(n<0)_exit(91);if(!n)break;put(1,b,n);}
}
static void ms(unsigned n) {struct timespec t={n/1000,(n%1000)*1000000};while(nanosleep(&t,&t)&&errno==EINTR){}}
static void marker(const char *p,const char *s){int fd=open(p,O_WRONLY|O_CREAT|O_TRUNC,0600);if(fd<0)_exit(92);put(fd,s,strlen(s));close(fd);}
int main(int argc,char **argv) {
    alarm(8);const char *m=argc>1?argv[1]:"empty";
    if(!strcmp(m,"echo")){echo();put(2,"err\0\316\273\n",7);return argc>2?atoi(argv[2]):0;}
    if(!strcmp(m,"empty"))return argc>2?atoi(argv[2]):0;
    if(!strcmp(m,"args")){for(int i=0;i<argc;i++){for(unsigned char *p=(void*)argv[i];*p;p++)dprintf(1,"%02x",*p);put(1,"\n",1);}return 0;}
    if(!strcmp(m,"observe")){
        int n=0;while(environ[n])n++;char cwd[4096];if(!getcwd(cwd,sizeof cwd))return 93;
        dprintf(1,"env=%d\ncwd=%s\nfds=",n,cwd);for(int fd=3;fd<4096;fd++)if(fcntl(fd,F_GETFD)>=0)dprintf(1,"%d,",fd);
        sigset_t mask;sigprocmask(SIG_SETMASK,NULL,&mask);struct sigaction a,b;sigaction(SIGUSR1,NULL,&a);sigaction(SIGPIPE,NULL,&b);
        dprintf(1,"\nusr1=%d\npipe=%d\nmask=%d,%d\n",a.sa_handler==SIG_DFL,b.sa_handler==SIG_DFL,sigismember(&mask,SIGUSR1),sigismember(&mask,SIGPIPE));return 0;
    }
    if(!strcmp(m,"signal")){raise(atoi(argv[2]));return 94;}
    if(!strcmp(m,"fill")){size_t n=strtoull(argv[2],NULL,10);fill(1,'a',n);fill(2,'b',n);return 0;}
    if(!strcmp(m,"out")){fill(atoi(argv[2]),'z',strtoull(argv[3],NULL,10));return 0;}
    if(!strcmp(m,"duplex")){fill(1,'a',131072);fill(2,'b',131072);echo();return 0;}
    if(!strcmp(m,"interleave")){char b[997];for(;;){ssize_t n=read(0,b,sizeof b);if(n<0&&errno==EINTR)continue;if(n<=0)break;put(1,b,n);put(2,b,n);}return 0;}
    if(!strcmp(m,"early")){close(0);put(1,"done",4);put(2,"closed",6);return 7;}
    if(!strcmp(m,"one")){close(1);char b[4096];while(read(0,b,sizeof b)>0){}put(2,"only stderr",11);return 0;}
    if(!strcmp(m,"split")){const char *s="\360\237\231\202\316\273\0\n";for(int i=0;i<8;i++){put(1,s+i,1);put(2,s+i,1);}return 0;}
    if(!strcmp(m,"bad")){if(argc>3)marker(argv[3],"child effect");put(atoi(argv[2]),"\xff",1);return 0;}
    if(!strcmp(m,"bad-over")){put(1,"\xff",1);fill(1,'x',1048576);return 0;}
    if(!strcmp(m,"sleep")){ms(5000);return 0;}
    if(!strcmp(m,"steady")){for(;;){put(1,"x",1);ms(2);}}
    if(!strcmp(m,"descendant")){
        pid_t child=fork();if(child<0)return 95;
        if(!child){if(argc>3){char s[32];snprintf(s,sizeof s,"%ld",(long)getpid());marker(argv[3],s);}ms((unsigned)atoi(argv[2]));put(1,"descendant",10);_exit(0);}return 12;
    }
    if(!strcmp(m,"closed-descendant")){
        pid_t child=fork();if(child<0)return 95;
        if(!child){char s[32];snprintf(s,sizeof s,"%ld",(long)getpid());marker(argv[2],s);close(0);close(1);close(2);ms(5000);_exit(0);}return 12;
    }
    if(!strcmp(m,"parent-children")){
        char path[128],line[16384];snprintf(path,sizeof path,"/proc/%ld/task/%ld/children",(long)getppid(),(long)getppid());
        FILE *f=fopen(path,"r");if(!f||!fgets(line,sizeof line,f))return 96;fclose(f);int n=0;char *p=line;
        while(*p){while(*p==' ')p++;if(!*p)break;strtol(p,&p,10);n++;}dprintf(1,"%d",n);return 0;
    }
    if(!strcmp(m,"mark")){marker(argv[2],"executed");return 0;}
    if(!strcmp(m,"checkdead")){
        FILE *f=fopen(argv[2],"r");long pid=0;if(!f||fscanf(f,"%ld",&pid)!=1)return 96;if(f)fclose(f);
        char path[80],line[512];snprintf(path,sizeof path,"/proc/%ld/stat",pid);f=fopen(path,"r");
        if(f){if(!fgets(line,sizeof line,f))return 97;fclose(f);char *p=strrchr(line,')');if(!p||p[2]!='Z')return 98;}put(1,"dead",4);return 0;
    }
    return 99;
}
'''


def fixture(root):
    source = root / 'fixture.c'
    source.write_text(FIXTURE)
    binary = root / 'tool with spaces=literal'
    compiler = shutil.which('cc')
    assert compiler, 'the independent test fixture needs a C compiler'
    checked([compiler, '-std=c11', '-O2', '-Wall', '-Wextra', source, '-o', binary])
    return binary


def check_harness(root, helper):
    # Deliberately let the direct leader exit while its descendant keeps pipes.
    # The outer deadline must clean that group without waiting its five seconds.
    marker = root / 'harness-descendant'
    start = time.monotonic()
    try:
        invoke([helper, 'descendant', '5000', marker], timeout=.1)
    except subprocess.TimeoutExpired:
        pass
    else:
        raise AssertionError('harness failed to impose its own deadline')
    assert time.monotonic() - start < 2
    assert marker.exists(), 'harness fixture did not reach its descendant'
    assert not Path('/proc/' + marker.read_text()).exists(), 'outer timeout leaked a descendant'
    COUNTS['cases'] += 1


SOURCE = '''The application ProcessOracle manages tools.
A Data has:
- arguments: List<Text>
- input: Text
- timeout: Int
- output: Text
- errors: Text
- status: Int
- encoded: Text
- separator: Text
Action: Run.
When Run happens:
- the system reads text from standard input into data.input
- the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status
- the system formats data.output as JSON text into data.encoded
- the system writes text data.encoded to standard output
- the system assigns "|" to data.separator
- the system writes text data.separator to standard output
- the system formats data.errors as JSON text into data.encoded
- the system writes text data.encoded to standard output
- the system writes text data.separator to standard output
- the system formats data.status as JSON text into data.encoded
- the system writes text data.encoded to standard output
- the system records a trace event named Completed
'''


def package(root, name, source):
    path = root / name
    path.mkdir()
    (path / 'ail-package.md').write_text('# AIL Package\nname: '+name+'\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, functions, traces\nconformance: first-slice\n')
    (path / 'spec.ail-spec.md').write_text(source)
    return path


def routes(root, package, stem, action='Run'):
    core = root / f'{stem}.core'
    core.write_bytes(checked([AIL, 'ail-core', package]))
    rendered = root / f'{stem}-rendered'
    rendered.mkdir()
    (rendered / 'ail-package.md').write_bytes((package / 'ail-package.md').read_bytes())
    (rendered / 'spec.ail-spec.md').write_bytes(checked([AIL, 'ail-spec', '--core-file', core]))
    result = []
    for name, arguments in [('source', [package]), ('core', ['--core-file', core]), ('rendered', [rendered])]:
        bytecode = root / f'{stem}-{name}.json'
        bytecode.write_bytes(checked([AIL, 'ail-lower'] + arguments))
        ste = root / f'{stem}-{name}.ail-ste'
        ste.write_bytes(checked([AIL, 'ail-ste'] + arguments))
        for form, artifact, compile_arguments in [('direct', bytecode, arguments), ('ste', ste, [ste])]:
            binary = root / f'{stem}-{name}-{form}.elf'
            checked([AIL, 'ail-compile'] + compile_arguments + ['--action', action, '--target', 'linux-x86_64-elf', '--out', binary])
            result.append((artifact, binary))
    return result


def transport(root, helper, rng):
    artifacts = routes(root, package(root, 'transport', SOURCE), 'transport')
    alphabet = ['a', 'Z', ' ', '\0', '\n', '\r', '\t', 'λ', '🙂', '漢', '\u0085', '\x7f', '"', '\\', '=', '$', ';']
    cases = ['', 'all\0bytes\r\nλ🙂', 'final line without newline']
    cases += [''.join(rng.choice(alphabet) for _ in range(rng.randrange(150))) for _ in range(29)]
    for index, value in enumerate(cases):
        status = [0, 1, 127, 255][index % 4]
        args = ['data.arguments=' + canonical(['echo', str(status)]), 'data.timeout=3000']
        for artifact in artifacts:
            for backend in ['VM', 'native']:
                exact(run(artifact, backend, helper, args, value.encode()), expected(value, 'err\0λ\n', status))
    marker = root / 'shell must not run'
    argv = ['', ';touch ' + str(marker), '$(touch ' + str(marker) + ')', '*', '$HOME', '"quoted"', '\n', 'λ🙂']
    expected_args = '\n'.join(s.encode().hex() for s in [str(helper), 'args'] + argv) + '\n'
    for artifact in artifacts:
        for backend in ['VM', 'native']:
            exact(run(artifact, backend, helper, ['data.arguments=' + canonical(['args'] + argv), 'data.timeout=3000']), expected(expected_args))
    assert not marker.exists()
    print('PASS: 32 exact Unicode/NUL/newline/status cases and literal argv across all 12 public routes', flush=True)
    return artifacts[0]


def raw_capture(root, name, extra=()):
    return program(root, name, list(extra) + [op('STDIN_READ_TEXT', key='data.input'), process_op(),
        write('data.output'), op('STDERR_WRITE_TEXT', source='data.errors'), seed('data.separator', '|'),
        write('data.separator'), op('JSON_FORMAT', key='data.encoded', source='data.status', type='Int'), write('data.encoded')])


def invocation(argv, timeout=3000):
    return ['data.arguments=' + canonical(argv), 'data.timeout=' + str(timeout)]


def limits(root, helper):
    capture = raw_capture(root, 'capture')
    for backend in ['VM', 'native']:
        exact(run(capture, backend, helper, invocation(['fill', str(LIMIT)])), b'a' * LIMIT + b'|0', b'b' * LIMIT)
        exact(run(capture, backend, helper, invocation(['echo']), b'x' * LIMIT), b'x' * LIMIT + b'|0', b'err\0\xce\xbb\n')
        for fd in ['1', '2']:
            fatal(run(capture, backend, helper, invocation(['out', fd, str(LIMIT + 1)])), 'TEXT-LIMIT')
            fatal(run(capture, backend, helper, invocation(['bad', fd])), 'TEXT')
        fatal(run(capture, backend, helper, invocation(['bad-over'])), 'TEXT-LIMIT')
        fatal(run(capture, backend, helper, invocation(['echo']), b'x' * (LIMIT + 1)), 'TEXT-LIMIT')
        exact(run(capture, backend, helper, invocation(['empty'] + [''] * 255)), b'|0')
        fatal(run(capture, backend, helper, invocation(['empty'] + [''] * 256)), 'PROCESS-LIMIT')
        # argv0, the mode argument, and every terminating NUL are included.
        remaining = 65536 - len(os.fsencode(helper)) - 1 - len('empty') - 1 - 1
        exact(run(capture, backend, helper, invocation(['empty', 'x' * remaining])), b'|0')
        fatal(run(capture, backend, helper, invocation(['empty', 'x' * (remaining + 1)])), 'PROCESS-LIMIT')
        # Argument count/aggregate limits win over embedded NUL.
        fatal(run(capture, backend, helper, invocation(['empty', '\0'] + [''] * 255)), 'PROCESS-LIMIT')
        fatal(run(capture, backend, helper, invocation(['empty', '\0'])), 'PROCESS')
        for timeout in ['0', '-1', '60001', '-9223372036854775808']:
            fatal(run(capture, backend, helper, ['data.arguments=[]', 'data.timeout=' + timeout]), 'PROCESS')
        for timeout in ['bad', '9223372036854775808']:
            fatal(run(capture, backend, helper, ['data.arguments=[]', 'data.timeout=' + timeout]), 'INT')
        for malformed in ['[1]', '["ok",]', '[]x', '["\\ud800"]', '[[]]', '[true]']:
            fatal(run(capture, backend, helper, ['data.arguments=' + malformed, 'data.timeout=0']), 'LIST')
    invalid_native = program(root, 'native-transport', [process_op()])
    for malformed in [b'\x80', b'\xc0\x80', b'\xed\xa0\x80', b'\xf4\x90\x80\x80']:
        fatal(run(invalid_native, 'native', helper, ['data.arguments=[]', b'data.input=' + malformed, 'data.timeout=0']), 'TEXT')
        fatal(run(invalid_native, 'native', helper, [b'data.arguments=["' + malformed + b'"]', 'data.timeout=0']), 'LIST')
    for backend in ['VM', 'native']:
        fatal(run(invalid_native, backend, helper, ['data.arguments=[]', 'data.timeout=0']), 'TEXT')
        fatal(run(invalid_native, backend, helper, ['data.arguments=[]', 'data.input=']), 'INT')
    # Read the list transport from a file so Linux's outer argv ceiling cannot
    # mask the language's independent encoded-size and decoded-size rules.
    from_file = raw_capture(root, 'arguments-file', [op('FILE_READ_TEXT', resource='args', key='data.arguments')])
    path = root / 'arguments.json'
    path.write_bytes(b' ' * (LIMIT - len(b'["empty"]')) + b'["empty"]')
    for backend in ['VM', 'native']:
        exact(run(from_file, backend, helper, ['--read-file=args=' + str(path), 'data.timeout=3000']), b'|0')
    # Raw DEL canonicalization would expand sixfold. This borrowed argument list
    # still needs no canonical result or List arena allocation.
    path.write_text('["empty","' + '\x7f' * 40000 + '"]')
    for backend in ['VM', 'native']:
        exact(run(from_file, backend, helper, ['--read-file=args=' + str(path), 'data.timeout=3000']), b'|0')
    print('PASS: exact/over Text and argv limits, UTF-8/argument grammar and deterministic validation priority', flush=True)


def lifecycle(root, helper):
    capture = raw_capture(root, 'lifecycle')
    for backend in ['VM', 'native']:
        payload = ('λ\0🙂\n' * 70000).encode()
        exact(run(capture, backend, helper, invocation(['duplex']), payload), b'a' * 131072 + payload + b'|0', b'b' * 131072)
        exact(run(capture, backend, helper, invocation(['interleave']), payload), payload + b'|0', payload)
        exact(run(capture, backend, helper, invocation(['early']), b'x' * LIMIT), b'done|7', b'closed')
        exact(run(capture, backend, helper, invocation(['one']), b'x' * 131072), b'|0', b'only stderr')
        exact(run(capture, backend, helper, invocation(['split'])), '🙂λ\0\n|0'.encode(), '🙂λ\0\n'.encode())
        for sig in [signal.SIGTERM, signal.SIGPIPE]:
            exact(run(capture, backend, helper, invocation(['signal', str(int(sig))])), f'|{-sig}'.encode())
        for mode in ['sleep', 'steady']:
            before = time.monotonic()
            fatal(run(capture, backend, helper, invocation([mode], 50)), 'TIMEOUT')
            assert time.monotonic() - before < 3, 'deadline was restarted or output starved the clock'
        exact(run(capture, backend, helper, invocation(['descendant', '30'])), b'descendant|12')
        marker = root / ('child-' + backend)
        fatal(run(capture, backend, helper, invocation(['descendant', '5000', str(marker)], 100)), 'TIMEOUT')
        assert marker.exists()
        pid = int(marker.read_text())
        assert not Path(f'/proc/{pid}').exists(), f'descendant {pid} survived timeout cleanup'
    # A high inherited non-CLOEXEC descriptor must be closed in the tool; its
    # owner's open-file description and original fd remain usable afterward.
    descriptor_path = root / 'sentinel'
    descriptor_path.write_bytes(b'parent sentinel')
    fd = os.open(descriptor_path, os.O_RDONLY)
    high = 300
    os.dup2(fd, high, inheritable=True)
    def altered_signals():
        signal.signal(signal.SIGUSR1, signal.SIG_IGN)
        signal.signal(signal.SIGPIPE, signal.SIG_IGN)
        signal.pthread_sigmask(signal.SIG_BLOCK, [signal.SIGUSR1, signal.SIGPIPE])
    try:
        for backend in ['VM', 'native']:
            observed = f'env=0\ncwd={root}\nfds=\nusr1=1\npipe=1\nmask=0,0\n|0'.encode()
            exact(run(capture, backend, helper, invocation(['observe']), cwd=root, pass_fds=(fd, high), prepare=altered_signals), observed)
            assert os.read(high, 6) == (b'parent' if backend == 'VM' else b' senti')
    finally:
        os.close(high)
        os.close(fd)
    # Native startup has no Rust descriptor normalization: actually enter with
    # each combination of closed stdio and still perform a captured child call.
    marker_program = program(root, 'closed-fds', initial(['mark', str(root / 'closed-marker')]) + [process_op()])
    for closed in [(0,), (1,), (2,), (0, 1, 2)]:
        marker = root / 'closed-marker'
        marker.unlink(missing_ok=True)
        def close_selected():
            for fd in closed:
                os.close(fd)
        exact(run(marker_program, 'native', helper, prepare=close_selected))
        assert marker.read_text() == 'executed'
    # An inherited SIGCHLD ignore must fail before any child effect.
    for backend in ['VM', 'native']:
        marker = root / 'closed-marker'
        marker.unlink(missing_ok=True)
        fatal(run(marker_program, backend, helper, prepare=lambda: signal.signal(signal.SIGCHLD, signal.SIG_IGN)), 'IO')
        assert not marker.exists()
    print('PASS: concurrent large streams, early close, signals, descendant EOF/deadlines, env/cwd/fd/mask isolation and closed native stdio', flush=True)


def authority(root, helper):
    marker = root / 'must-not-execute'
    effect = program(root, 'authority', initial(['mark', str(marker)]) + [process_op()])
    malformed = ['--run-process', '--run-process=', '--run-process=tool', '--run-process==/bin/true',
        '--run-process=tool=', '--run-process=tool=relative', '--run-process=bad-name=/bin/true',
        '--run-process-extra=tool=/bin/true', '--run-process=tool=/' + 'x' * 4095]
    for backend in ['VM', 'native']:
        fatal(run(effect, backend), 'CAPABILITY')
        fatal(run(effect, backend, arguments=['tool=' + str(helper), 'run-process=tool=' + str(helper)]), 'CAPABILITY')
        fatal(run(effect, backend, arguments=['--read-file=tool=' + str(helper)]), 'CAPABILITY')
        fatal(run(effect, backend, arguments=['--run-process=other=' + str(helper)]), 'CAPABILITY')
        for flag in malformed:
            fatal(run(effect, backend, helper, [flag]), 'CAPABILITY')
        # Every earlier occurrence must validate, even if overwritten later.
        fatal(run(effect, backend, arguments=['--run-process=tool=relative', '--run-process=tool=' + str(helper)]), 'CAPABILITY')
        assert not marker.exists()
        exact(run(effect, backend, arguments=['--run-process=tool=/missing/not-opened', '--run-process=tool=' + str(helper)]))
        assert marker.read_text() == 'executed'
        marker.unlink()
        fatal(run(effect, backend, helper, ['--run-process=tool=/missing/last-wins']), 'PROCESS')
        assert not marker.exists()
    capture = raw_capture(root, 'exec-errors')
    no_execute = root / 'no-execute'
    no_execute.write_text('ordinary data')
    no_format = root / 'no-format'
    no_format.write_text('printf shell-fallback-is-forbidden')
    no_format.chmod(0o700)
    no_interpreter = root / 'no-interpreter'
    no_interpreter.write_text('#!/missing/interpreter\n')
    no_interpreter.chmod(0o700)
    for backend in ['VM', 'native']:
        for executable in [root / 'missing', no_execute, no_format, no_interpreter, root]:
            fatal(run(capture, backend, executable, invocation([])), 'PROCESS')
        exact(run(capture, backend, helper, invocation(['empty', '127'])), b'|127')
    # Captured child output does not leak into ordinary non-stdio execution;
    # traces include only the public resource name.
    silent = program(root, 'silent', [process_op(), op('OUTPUT_FIELD', key='data.status')])
    for backend in ['VM', 'native']:
        prefix = [AIL, 'ail-vm', silent[0], '--action', 'Run'] if backend == 'VM' else [silent[1]]
        result = invoke(prefix + ['--run-process=tool=' + str(helper)] + invocation(['echo']) + ['data.input=secret child input'])
        assert result.returncode == 0, result.stderr
        assert b'secret child input' not in result.stdout + result.stderr
        assert str(helper).encode() not in result.stdout + result.stderr
    default_output = program(root, 'default-output', [process_op()])
    for backend in ['VM', 'native']:
        prefix = [AIL, 'ail-vm', default_output[0], '--action', 'Run'] if backend == 'VM' else [default_output[1]]
        result = invoke(prefix + ['--run-process=tool=' + str(helper)] + invocation(['empty']) + ['data.input='])
        assert result.returncode == 0, result.stderr
        assert str(helper).encode() not in result.stdout + result.stderr, 'permission path leaked into state output'
    print('PASS: authority provenance, all malformed/duplicate grants, no shell fallback, exec failure versus 127 and capture privacy', flush=True)


def recovery(root, helper):
    old = [seed('data.output', 'old output'), seed('data.errors', 'old errors'), seed('data.status', '27'),
           op('COPY_FIELD', key='data.oldout', source='data.output', text='old alias'),
           op('COPY_FIELD', key='data.olderr', source='data.errors', text='old alias')]
    for name, argv, timeout, code in [
        ('nul', ['empty', '\0'], '2000', 'PROCESS'),
        ('timeout', ['sleep'], '40', 'TIMEOUT'),
        ('badstdout', ['bad', '1'], '2000', 'TEXT'),
        ('badstderr', ['bad', '2', str(root / 'child-effect')], '2000', 'TEXT')]:
        instructions = initial(argv, 'input snapshot', timeout) + old
        instructions += [op('TRY_BEGIN', errors='any', key='data.failure'), process_op(),
                         op('RECOVER'), op('TRY_END')]
        instructions += report() + [write('data.separator'), write('data.failure'), write('data.separator'),
                                   write('data.oldout'), write('data.separator'), write('data.olderr')]
        checked_program = program(root, 'atomic-' + name, instructions)
        result = expected('old output', 'old errors', 27) + f'|AIL-RUNTIME-{code}|old output|old errors'.encode()
        for backend in ['VM', 'native']:
            exact(run(checked_program, backend, helper), result)
    assert (root / 'child-effect').read_text() == 'child effect'
    # Snapshots survive two aliased destinations and later result mutation.
    alias = program(root, 'aliases', initial(['echo', '7'], 'input\0λ', '3000') +
        [op('COPY_FIELD', key='data.alias', source='data.input', text='saved input'),
         process_op(stdout='data.input', status='data.timeout'), write('data.input'),
         write('data.alias'), op('JSON_FORMAT', key='data.n', source='data.timeout', type='Int'), write('data.n')])
    for backend in ['VM', 'native']:
        exact(run(alias, backend, helper), 'input\0λinput\0λ7'.encode())
    owned_alias = program(root, 'owned-output-alias', initial(['echo'], 'first\0λ') +
        [process_op(), op('COPY_FIELD', key='data.alias', source='data.output', text='saved capture'),
         seed('data.input', 'second🙂'), process_op(), write('data.alias'), write('data.output')])
    for backend in ['VM', 'native']:
        exact(run(owned_alias, backend, helper), 'first\0λsecond🙂'.encode())
    # The handler itself invokes a fixture that observes the old descendant.
    # A generic harness checking only after AIL exits would miss late cleanup.
    marker = root / 'handler-descendant'
    cleanup = program(root, 'cleanup-before-handler', initial(['descendant', '5000', str(marker)], timeout='80') +
        [op('TRY_BEGIN', errors='TIMEOUT', key='data.failure'), process_op(), op('RECOVER'),
         seed('data.arguments', canonical(['checkdead', str(marker)])), seed('data.timeout', '2000'),
         process_op(), write('data.output'), op('TRY_END')])
    for backend in ['VM', 'native']:
        exact(run(cleanup, backend, helper), b'dead')
    cleanup_success = program(root, 'cleanup-before-success', initial(['closed-descendant', str(marker)]) +
        [process_op(), seed('data.arguments', canonical(['checkdead', str(marker)])), process_op(), write('data.output')])
    for backend in ['VM', 'native']:
        exact(run(cleanup_success, backend, helper), b'dead')
    # Arguments/input are borrowed; a process invocation charges exactly its two
    # outputs. At a 64 MiB boundary even empty outputs each consume one byte.
    fill = [seed('data.chunk', 'x' * LIMIT), seed('data.empty', '')]
    fill += [op('TEXT_JOIN', key='data.scratch', left='data.chunk', right='data.empty')] * 63
    fill += [seed('data.tail', 'x' * (LIMIT - 2)), op('TEXT_JOIN', key='data.scratch', left='data.tail', right='data.empty')]
    success = program(root, 'budget-exact', initial(['empty']) + fill + [process_op(), seed('data.done', 'exact'), write('data.done')])
    excess = program(root, 'budget-extra', initial(['empty']) + fill + [process_op(), process_op()])
    # Failed validation of the second capture must consume zero of the final two
    # arena bytes, and the following successful empty call must still fit.
    failed_first = program(root, 'budget-failure-no-charge', initial(['bad', '2']) + fill +
        [op('TRY_BEGIN', errors='TEXT', key='data.failure'), process_op(), op('RECOVER'),
         seed('data.arguments', '["empty"]'), process_op(), op('TRY_END'), seed('data.done', 'no charge'), write('data.done')])
    for backend in ['VM', 'native']:
        exact(run(success, backend, helper), b'exact')
        fatal(run(excess, backend, helper), 'TEXT-LIMIT')
        exact(run(failed_first, backend, helper), b'no charge')
    list_fill = [seed('data.list', '[]'), seed('data.item', 'x' * (LIMIT - 4))]
    list_fill += [op('LIST_APPEND', key='data.scratchlist', source='data.list', type='Text', value='data.item')] * 64
    borrowed_arguments = program(root, 'full-list-budget', initial(['empty']) + list_fill +
        [process_op(), seed('data.done', 'borrowed'), write('data.done')])
    for backend in ['VM', 'native']:
        exact(run(borrowed_arguments, backend, helper), b'borrowed')
    # Calls through another action share authority and arena. This also exercises
    # repeated real success/launch/timeout/Text failures under bounded resources.
    child = {'action': 'Child', 'instructions': [process_op(), op('RETURN_SUCCESS')]}
    repeated = initial(['empty'])
    repeated += [op('CALL_ACTION', target='Child')] * 180
    for argv, timeout, code in [(['empty', '\0'], '2000', 'PROCESS'), (['sleep'], '2', 'TIMEOUT'), (['bad', '2'], '2000', 'TEXT')]:
        repeated += [seed('data.arguments', canonical(argv)), seed('data.timeout', timeout)]
        repeated += [op('TRY_BEGIN', errors=code, key='data.failure'), op('CALL_ACTION', target='Child'), op('RECOVER'), op('TRY_END')] * 30
    repeated += [seed('data.arguments', '["parent-children"]'), seed('data.timeout', '2000'),
                 process_op(), write('data.output'), seed('data.done', 'reused'), write('data.done')]
    repeated_program = program(root, 'repeated', repeated, [child])
    def constrained():
        resource.setrlimit(resource.RLIMIT_NOFILE, (32, 32))
        resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024, 256 * 1024 * 1024))
    for backend in ['VM', 'native']:
        exact(run(repeated_program, backend, helper, prepare=constrained, timeout=20), b'1reused')
    print('PASS: atomic recovery/aliases, child effects, cleanup before handlers, exact shared arena accounting and 271 repeated constrained calls with no unreaped direct children', flush=True)


def filter_expected(rows):
    errors = {'AIL-RUNTIME-DIVZERO': 0, 'AIL-RUNTIME-JSON': 0, 'AIL-RUNTIME-JSON-PATH': 0, 'AIL-RUNTIME-OVERFLOW': 0}
    output = ''
    count = accepted = 0
    for row in rows:
        if not row.strip():
            continue
        count += 1
        code = None
        try:
            parsed = json.loads(row)
        except (ValueError, TypeError):
            code = 'AIL-RUNTIME-JSON'
        if code is None:
            if not isinstance(parsed, dict) or 'value' not in parsed or 'divisor' not in parsed:
                code = 'AIL-RUNTIME-JSON-PATH'
            elif type(parsed['value']) is not int or type(parsed['divisor']) is not int or not all(-(1 << 63) <= parsed[k] < (1 << 63) for k in ['value', 'divisor']):
                code = 'AIL-RUNTIME-JSON'
            elif parsed['divisor'] == 0:
                code = 'AIL-RUNTIME-DIVZERO'
            elif parsed['value'] == -(1 << 63) and parsed['divisor'] == -1:
                code = 'AIL-RUNTIME-OVERFLOW'
        if code:
            errors[code] += 1
        else:
            accepted += 1
            left, right = parsed['value'], parsed['divisor']
            quotient = abs(left) // abs(right) * (-1 if (left < 0) != (right < 0) else 1)
            output += canonical({'value': quotient}) + '\n'
    summary = {'rows': count, 'accepted': accepted, 'rejected': count - accepted, 'errors': errors}
    return output, canonical(summary) + '\n'


def coordinator(root, rng, helper):
    worker = root / 'ail worker.elf'
    checked([AIL, 'ail-compile', REPO / 'examples/json-lines-filter', '--action', 'Filter', '--target', 'linux-x86_64-elf', '--out', worker])
    artifacts = routes(root, REPO / 'examples/tool-coordinator', 'coordinator', 'Coordinate')
    tasks, results = [], []
    for index in range(24):
        rows = [canonical({'value': rng.randint(-100000, 100000), 'divisor': rng.choice([-19, -1, 1, 3, 17])}) for _ in range(rng.randrange(1, 8))]
        rows += ['{', '{}', '{"value":4,"divisor":0}', '{"value":-9223372036854775808,"divisor":-1}', ' ', '\t']
        rng.shuffle(rows)
        task = {'arguments': ['--stdio'], 'input': '\n'.join(rows), 'timeout': 3000}
        out, err = filter_expected(rows)
        result = {'status': 0, 'stdout': out, 'stderr': err, 'error': ''}
        if index % 7 == 1:
            task['timeout'] = 0
            result = {'status': None, 'stdout': '', 'stderr': '', 'error': 'AIL-RUNTIME-PROCESS'}
        elif index % 7 == 2:
            task['arguments'].append('\0')
            result = {'status': None, 'stdout': '', 'stderr': '', 'error': 'AIL-RUNTIME-PROCESS'}
        tasks.append(task)
        results.append(result)
    payload = ('\n'.join(map(canonical, tasks)) + '\n').encode()
    wanted = ('\n'.join(map(canonical, results)) + '\n').encode()
    for artifacts_pair in artifacts:
        for backend in ['VM', 'native']:
            exact(run(artifacts_pair, backend, arguments=['--run-process=worker=' + str(worker)], input=payload, action='Coordinate', timeout=25), wanted)
    controlled = [
        ({'arguments': ['empty', '127'], 'input': '', 'timeout': 2000}, {'status': 127, 'stdout': '', 'stderr': '', 'error': ''}),
        ({'arguments': ['echo', '1'], 'input': 'λ\0\n', 'timeout': 2000}, {'status': 1, 'stdout': 'λ\0\n', 'stderr': 'err\0λ\n', 'error': ''}),
        ({'arguments': ['sleep'], 'input': '', 'timeout': 40}, {'status': None, 'stdout': '', 'stderr': '', 'error': 'AIL-RUNTIME-TIMEOUT'}),
        ({'arguments': ['bad', '2'], 'input': '', 'timeout': 2000}, {'status': None, 'stdout': '', 'stderr': '', 'error': 'AIL-RUNTIME-TEXT'}),
        ({'arguments': ['empty', '255'], 'input': '', 'timeout': 2000}, {'status': 255, 'stdout': '', 'stderr': '', 'error': ''})]
    payload = ('\n'.join(canonical(task) for task, _ in controlled) + '\n').encode()
    wanted = ('\n'.join(canonical(result) for _, result in controlled) + '\n').encode()
    for artifacts_pair in artifacts:
        for backend in ['VM', 'native']:
            exact(run(artifacts_pair, backend, arguments=['--run-process=worker=' + str(helper)], input=payload, action='Coordinate'), wanted)
    print('PASS: 24 independent AIL-to-AIL tasks plus 5 controlled nonzero/timeout/Text tasks across 12 coordinator routes', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--section', choices=['all', 'transport', 'limits', 'lifecycle', 'authority', 'recovery', 'coordinator'], default='all')
    args = parser.parse_args()
    libc = ctypes.CDLL(None, use_errno=True)
    assert libc.prctl(36, 1, 0, 0, 0) == 0, 'PR_SET_CHILD_SUBREAPER failed'
    try:
        with tempfile.TemporaryDirectory(prefix='ail-process-oracle-') as directory:
            root = Path(directory)
            helper = fixture(root)
            check_harness(root, helper)
            rng = random.Random(SEED)
            if args.section in ['all', 'transport']:
                transport(root, helper, rng)
            if args.section in ['all', 'limits']:
                limits(root, helper)
            if args.section in ['all', 'lifecycle']:
                lifecycle(root, helper)
            if args.section in ['all', 'authority']:
                authority(root, helper)
            if args.section in ['all', 'recovery']:
                recovery(root, helper)
            if args.section in ['all', 'coordinator']:
                coordinator(root, rng, helper)
    finally:
        reap_adopted(False)
    print(f"PASS: process oracle seed={SEED:#x}; {COUNTS['cases']} independent assertions, {COUNTS['executions']} bounded commands", flush=True)


if __name__ == '__main__':
    main()
