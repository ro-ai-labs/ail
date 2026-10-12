#!/usr/bin/env python3
"""Independent bounded standard-stream oracle. Run cargo build first.

Every expected byte is derived independently; subprocess pipes are fed/drained
concurrently, hard timeouts bound execution, and temporary artifacts are removed.
"""
import json
import fcntl
import os
from pathlib import Path
import random
import resource
import subprocess
import tempfile
import threading
import time

REPO=Path(__file__).resolve().parent.parent
AIL=REPO/'target/debug/ail'
LIMIT=1_048_576
SEED=0x57D10

def quoted(value):
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + "".join(escapes[c] if c in escapes else f"\\u{ord(c):04x}" if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value) + '"'


def canonical(value):
    if isinstance(value, str):
        return quoted(value)
    if isinstance(value, list):
        return '[' + ','.join(canonical(item) for item in value) + ']'
    if isinstance(value, dict):
        return '{' + ','.join(quoted(key) + ':' + canonical(item) for key, item in value.items()) + '}'
    return str(value)


def invoke(arguments, input=b""):
    return subprocess.run(arguments, input=input, cwd=REPO, capture_output=True, timeout=30)


def checked(arguments):
    result = invoke(arguments)
    assert result.returncode == 0, (arguments[:4], result.stdout[:1000], result.stderr[-2000:])
    return result.stdout


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions, routines=()):
    source = root / f"{name}.json"
    source.write_text(json.dumps({"kind": "AIL-Bytecode", "package": "stdio-oracle", "version": "1", "profile": "Application", "capability_grants": [], "target_support": {}, "external_bindings": [], "failures": [], "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}] + list(routines)}))
    binary = root / f"{name}.elf"
    checked([AIL, "ail-compile", source, "--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
    return source, binary


def run(program, backend, input=b"", arguments=()):
    artifact,binary=program
    prefix=[AIL,'ail-vm',artifact,'--action','Run'] if backend=='VM' else [binary]
    return invoke(prefix+['--stdio']+list(arguments),input)


def exact(result, stdout=b"", stderr=b""):
    assert result.returncode==0,(result.returncode,result.stdout[:1000],result.stderr[-2000:])
    assert result.stdout==stdout,(result.stdout[:1000],stdout[:1000])
    assert result.stderr==stderr,(result.stderr[:1000],stderr[:1000])


def fatal(result, category, prefix=b""):
    assert result.returncode>0,(result.returncode,result.stdout[:1000],result.stderr[-2000:])
    assert result.stdout==prefix,(result.stdout[:1000],prefix[:1000])
    assert ('AIL-RUNTIME-'+category+':').encode() in result.stderr,result.stderr[-2000:]


def seed(key, value):
    return op("SET_FIELD", key=key, value=value, text="oracle fixture")


def copy(key, source):
    return op("COPY_FIELD", key=key, source=source, text="immutable copy")


def output(key):
    return op("OUTPUT_FIELD", key=key)


def routes(root, package, stem, action="Run"):
    core = root / f"{stem}.core"
    core.write_bytes(checked([AIL,"ail-core",package]))
    rendered=root/f"{stem}-rendered";rendered.mkdir()
    (rendered/"ail-package.md").write_bytes((package/"ail-package.md").read_bytes())
    (rendered/"spec.ail-spec.md").write_bytes(checked([AIL,"ail-spec","--core-file",core]))
    result=[]
    for name,arguments in [("source",[package]),("core",["--core-file",core]),("rendered",[rendered])]:
        bytecode=root/f"{stem}_{name}.json";bytecode.write_bytes(checked([AIL,"ail-lower"]+arguments))
        ste=root/f"{stem}_{name}.ail-ste";ste.write_bytes(checked([AIL,"ail-ste"]+arguments))
        for label,artifact,compile_arguments in [("source",bytecode,arguments),("ste",ste,[ste])]:
            binary=root/f"{stem}_{name}_{label}.elf"
            checked([AIL,"ail-compile"]+compile_arguments+["--action",action,"--target","linux-x86_64-elf","--out",binary])
            result.append((artifact,binary))
    return result


def package(root,name,text):
    path=root/name;path.mkdir()
    (path/'ail-package.md').write_text('# AIL Package\nname: '+name+'\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, functions, traces\nconformance: first-slice\n')
    (path/'spec.ail-spec.md').write_text(text)
    return path


def read(key='data.raw'):return op('STDIN_READ_TEXT',key=key)
def write(source='data.raw'):return op('STDOUT_WRITE_TEXT',source=source)
def warning(source='data.raw'):return op('STDERR_WRITE_TEXT',source=source)


ECHO='''The application Echo manages streams.
A Data has:
- raw: Text
- returned: Text
Function: identity.
The function needs:
- value: Text
The function produces:
- result: Text
The function executes:
- the function returns local.value
Action: Run.
When Run happens:
- the system reads text from standard input into data.raw
- the system calls identity with value = data.raw into data.returned
- the system writes text data.returned to standard output
- the system writes text data.raw to standard error
- the system returns data.raw
- the system records a trace event named HiddenInStdio
'''


def transport(root,rng):
    artifacts=routes(root,package(root,'echo',ECHO),'echo')
    alphabet=['a','Z',' ','\0','\n','\r','\t','λ','🙂','漢','\u0085','\x7f','[',']','"','\\','=',':','try:','--stdio']
    cases=['','no newline','all\0bytes\r\nλ🙂','[value=Text],kind=anything']
    cases += [''.join(rng.choice(alphabet) for _ in range(rng.randrange(150))) for _ in range(96)]
    for value in cases:
        data=value.encode()
        for program in artifacts:
            for backend in ['VM','native']:exact(run(program,backend,data),data,data)
    print('PASS: 100 independent Unicode/NUL/keyword byte transports across 12 source/Core/STE VM/native routes, with function return and quiet wrappers',flush=True)
    return artifacts


def primitive_errors(root):
    echo=compile_program(root,'raw_echo',[read(),write()])
    for backend in ['VM','native']:
        exact(run(echo,backend,b'x'*LIMIT),b'x'*LIMIT)
        for bad in [b'\x80',b'\xc0\x80',b'\xed\xa0\x80',b'\xf4\x90\x80\x80',b'\xf0\x9f\x99',b'x'*(LIMIT-1)+b'\xff']:
            fatal(run(echo,backend,bad),'TEXT')
        for bad in [b'x'*(LIMIT+1),b'\xff'+b'x'*LIMIT]:fatal(run(echo,backend,bad),'TEXT-LIMIT')
        denied_prefix=[AIL,'ail-vm',echo[0],'--action','Run'] if backend=='VM' else [echo[1]]
        fatal(invoke(denied_prefix+['stdio=true'],b'unconsumed'),'CAPABILITY')
        for flag in ['--stdio=true','--stdioX','--stdio=','--stdio=false']:
            fatal(run(echo,backend,b'unconsumed',[flag]),'CAPABILITY')
    recovered=compile_program(root,'recovered',[seed('data.raw','prior'),seed('data.prefix','first:'),write('data.prefix'),op('TRY_BEGIN',errors='TEXT',key='data.error'),read(),op('RECOVER'),write('data.error'),op('TRY_END'),write(),read('data.next'),write('data.next')])
    for backend in ['VM','native']:exact(run(recovered,backend,b'\xff'),b'first:AIL-RUNTIME-TEXTprior')
    missing=compile_program(root,'missing_write',[op('TRY_BEGIN',errors='TEXT',key='data.error'),write('missing.value'),op('RECOVER'),write('data.error'),op('TRY_END')])
    for backend in ['VM','native']:exact(run(missing,backend),b'AIL-RUNTIME-TEXT')
    # Rust String transports cannot hold invalid UTF-8; direct ELF argv can.
    # A malformed write operand must be validated before emitting any prefix.
    invalid_write=compile_program(root,'invalid_native_write',[op('TRY_BEGIN',errors='TEXT',key='data.error'),write(),op('RECOVER'),warning('data.error'),op('TRY_END')])
    for invalid in [b'\x80',b'\xc0\x80',b'\xed\xa0\x80',b'\xf0\x9f\x99']:
        exact(run(invalid_write,'native',arguments=[b'data.raw='+invalid]),b'',b'AIL-RUNTIME-TEXT')
    oversized_write=compile_program(root,'oversized_write',[seed('data.raw','x'*(LIMIT+1)),write()])
    for backend in ['VM','native']:fatal(run(oversized_write,backend),'TEXT-LIMIT')
    prefix=compile_program(root,'fatal_prefix',[seed('data.prefix','kept prefix\0'),write('data.prefix'),read()])
    for backend in ['VM','native']:fatal(run(prefix,backend,b'x'*(LIMIT+1)),'TEXT-LIMIT',b'kept prefix\0')
    print('PASS: exact 1 MiB input, malformed UTF-8/read-write limits, authority/spoof/flag rejection, catchable Text recovery and retained prior output',flush=True)
    return echo


def pipe_harness(root,echo):
    large='λ'.encode()*262144;input='🙂'.encode()*196608
    program=compile_program(root,'duplex',[seed('data.prefix',large.decode()),write('data.prefix'),warning('data.prefix'),read(),write()])
    for backend in ['VM','native']:exact(run(program,backend,input),large+input,large)
    # Keep the producer open after the first excess byte: limit detection must
    # not wait for EOF. A feeder avoids blocking on the pipe's finite capacity.
    for backend in ['VM','native']:
        argv=[AIL,'ail-vm',echo[0],'--action','Run','--stdio'] if backend=='VM' else [echo[1],'--stdio']
        child=subprocess.Popen(argv,cwd=REPO,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        def feed():
            try:child.stdin.write(b'x'*(LIMIT+1));child.stdin.flush()
            except BrokenPipeError:pass
        feeder=threading.Thread(target=feed);feeder.start()
        try:child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill();child.wait();raise AssertionError('oversized input waited for producer EOF')
        finally:
            feeder.join(timeout=2)
            try:child.stdin.close()
            except BrokenPipeError:pass
        result=subprocess.CompletedProcess(argv,child.returncode,child.stdout.read(),child.stderr.read());fatal(result,'TEXT-LIMIT')
    # A scalar's UTF-8 bytes can cross arbitrary read boundaries.
    argv=[echo[1],'--stdio'];child=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    payload='x🙂λ\0終'.encode()
    def split_feed():
        try:
            for byte in payload:child.stdin.write(bytes([byte]));child.stdin.flush();time.sleep(.002)
            child.stdin.close()
        except BrokenPipeError:pass
    feeder=threading.Thread(target=split_feed);feeder.start()
    try:child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        child.kill();child.wait();raise
    finally:feeder.join(timeout=2)
    exact(subprocess.CompletedProcess(argv,child.returncode,child.stdout.read(),child.stderr.read()),payload)
    # Same sink verifies cross-stream order without assuming scheduling order
    # for distinct stdout and stderr pipes.
    ordered=compile_program(root,'merged',[seed('data.a','stdout:'),seed('data.b','stderr:'),write('data.a'),warning('data.b'),write('data.a')])
    for backend in ['VM','native']:
        argv=[AIL,'ail-vm',ordered[0],'--action','Run','--stdio'] if backend=='VM' else [ordered[1],'--stdio']
        result=subprocess.run(argv,cwd=REPO,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=10)
        assert result.returncode==0 and result.stdout==b'stdout:stderr:stdout:',(backend,result)
    print('PASS: bidirectional pipes larger than capacity, immediate limit+1 termination with open producer, split Unicode and exact merged-stream ordering',flush=True)


def shared_input_cursor(root,echo):
    path=root/'shared inherited cursor.bin'
    skipped=b'caller-prefix|';payload='remaining λ\0🙂'.encode()
    for backend in ['VM','native']:
        argv=[AIL,'ail-vm',echo[0],'--action','Run','--stdio'] if backend=='VM' else [echo[1],'--stdio']
        path.write_bytes(skipped+payload)
        with path.open('rb',buffering=0) as incoming:
            incoming.seek(len(skipped))
            result=subprocess.run(argv,stdin=incoming,capture_output=True,cwd=REPO,timeout=10)
            exact(result,payload)
            assert os.lseek(incoming.fileno(),0,os.SEEK_CUR)==len(skipped)+len(payload)
        path.write_bytes(skipped+b'x'*(LIMIT+16384))
        with path.open('rb',buffering=0) as incoming:
            incoming.seek(len(skipped))
            result=subprocess.run(argv,stdin=incoming,capture_output=True,cwd=REPO,timeout=10)
            fatal(result,'TEXT-LIMIT')
            consumed=os.lseek(incoming.fileno(),0,os.SEEK_CUR)-len(skipped)
            assert consumed==LIMIT+1,('input prefetched past limit sentinel',backend,consumed,LIMIT+1)
    print('PASS: inherited regular-file cursor starts at caller offset and oversize reads consume exactly limit+1 without hidden prefetch',flush=True)


def descriptor_failures(root,echo):
    writer=compile_program(root,'stdout_writer',[seed('data.raw','payload'),write()])
    error_writer=compile_program(root,'stderr_writer',[seed('data.raw','payload'),warning()])
    for program,fd in [(echo,0),(writer,1),(error_writer,2)]:
        for backend in ['VM','native']:
            argv=[AIL,'ail-vm',program[0],'--action','Run','--stdio'] if backend=='VM' else [program[1],'--stdio']
            def close():os.close(fd)
            result=subprocess.run(argv,input=b'',capture_output=True,cwd=REPO,timeout=10,preexec_fn=close)
            # Rust startup replaces closed inherited 0/1/2 with /dev/null before
            # main. The standalone ELF receives the actual closed descriptor.
            # A legitimate /dev/null stream must remain valid, so these startup
            # cases intentionally have distinct documented backend expectations.
            if backend=='VM':exact(result)
            else:
                assert result.returncode>0,(fd,backend,result)
                if fd!=2:fatal(result,'IO')
    for program,fd in [(writer,1),(error_writer,2)]:
        for backend in ['VM','native']:
            read_fd,write_fd=os.pipe();os.close(read_fd)
            argv=[AIL,'ail-vm',program[0],'--action','Run','--stdio'] if backend=='VM' else [program[1],'--stdio']
            kwargs={'stdout':write_fd,'stderr':subprocess.PIPE} if fd==1 else {'stdout':subprocess.PIPE,'stderr':write_fd}
            child=subprocess.Popen(argv,cwd=REPO,stdin=subprocess.DEVNULL,**kwargs);os.close(write_fd)
            try:out,err=child.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill();child.communicate();raise
            assert child.returncode>0,(fd,backend,child.returncode)
            if fd==1:assert b'AIL-RUNTIME-IO:' in err,(backend,err)
            else:
                assert out==b'',(backend,out)
                assert child.returncode!=101,('stderr diagnostic path panicked',backend)
    for backend in ['VM','native']:
        read_fd,write_fd=os.pipe()
        fcntl.fcntl(read_fd,fcntl.F_SETFL,fcntl.fcntl(read_fd,fcntl.F_GETFL)|os.O_NONBLOCK)
        argv=[AIL,'ail-vm',echo[0],'--action','Run','--stdio'] if backend=='VM' else [echo[1],'--stdio']
        result=subprocess.run(argv,stdin=read_fd,capture_output=True,cwd=REPO,timeout=10)
        assert fcntl.fcntl(read_fd,fcntl.F_GETFL)&os.O_NONBLOCK
        os.close(read_fd);os.close(write_fd);fatal(result,'IO')
        read_fd,write_fd=os.pipe();capacity=fcntl.fcntl(write_fd,fcntl.F_GETPIPE_SZ)
        fcntl.fcntl(write_fd,fcntl.F_SETFL,fcntl.fcntl(write_fd,fcntl.F_GETFL)|os.O_NONBLOCK)
        payload=b'p'*(capacity+1)
        program=compile_program(root,'partial_pipe_'+backend,[seed('data.raw',payload.decode()),write()])
        argv=[AIL,'ail-vm',program[0],'--action','Run','--stdio'] if backend=='VM' else [program[1],'--stdio']
        child=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=write_fd,stderr=subprocess.PIPE,cwd=REPO)
        try:_,error=child.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill();child.wait();raise
        assert fcntl.fcntl(write_fd,fcntl.F_GETFL)&os.O_NONBLOCK
        os.close(write_fd);prefix=os.read(read_fd,len(payload));os.close(read_fd)
        assert child.returncode>0 and b'AIL-RUNTIME-IO:' in error,(backend,child.returncode,error)
        assert 0<len(prefix)<len(payload) and prefix==payload[:len(prefix)],(backend,len(prefix),len(payload))
    print('PASS: closed-descriptor startup distinction, broken pipes without SIGPIPE/panics, inherited nonblocking IO failures and retained partial-write prefixes',flush=True)


def budgets_and_cleanup(root):
    initial=[seed('data.large','x'*LIMIT),seed('data.empty','')]+[op('TEXT_JOIN',key='data.fill',left='data.large',right='data.empty') for _ in range(63)]
    program=compile_program(root,'arena_exact',initial+[read(),write(),read('data.empty')])
    for backend in ['VM','native']:fatal(run(program,backend,b'x'*LIMIT),'TEXT-LIMIT',b'x'*LIMIT)
    # Empty EOF results charge one byte each, while each temporary read mapping
    # is freed. The inherited descriptor must remain open across all calls.
    cleanup=compile_program(root,'eof_cleanup',[seed('data.count','0'),op('LABEL',name='again'),read(),op('ADD_INT_FIELD',key='data.count',delta='1',text='iteration'),op('BRANCH_INT',left='f:data.count',comparison='<',right='i:2000',label='again'),seed('data.result','done'),write('data.result')])
    def limits():resource.setrlimit(resource.RLIMIT_AS,(160*1024*1024,160*1024*1024));resource.setrlimit(resource.RLIMIT_NOFILE,(24,24))
    result=subprocess.run([cleanup[1],'--stdio'],input=b'',capture_output=True,cwd=REPO,timeout=30,preexec_fn=limits);exact(result,b'done')
    print('PASS: exact shared 64 MiB read budget and empty-read charge; 2000 EOF reads retain fd0 and reclaim native scratch under 160 MiB/24-descriptor limits',flush=True)


class Number(str):pass
class Pairs(list):pass
WHITESPACE="\u0009\u000a\u000b\u000c\u000d\u001c\u001d\u001e\u001f\u0020\u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000"


def rejected_constant(value):raise ValueError(value)


def row_result(raw):
    try:
        value=json.loads(raw,parse_int=Number,parse_float=Number,parse_constant=rejected_constant,object_pairs_hook=Pairs)
        def validate(item):
            if type(item) is str and any(0xd800<=ord(c)<=0xdfff for c in item):raise ValueError('surrogate')
            if isinstance(item,Pairs):
                for key,child in item:validate(key);validate(child)
            elif isinstance(item,list):
                for child in item:validate(child)
        validate(value)
    except (ValueError,RecursionError):return None,'JSON'
    if not isinstance(value,Pairs):return None,'JSON-PATH'
    numbers=[]
    for key in ['value','divisor']:
        selected=[item for name,item in value if name==key]
        if len(selected)!=1:return None,'JSON-PATH'
        token=selected[0]
        if not isinstance(token,Number) or '.' in token or 'e' in token.lower():return None,'JSON'
        number=int(token)
        if not -(2**63)<=number<2**63:return None,'JSON'
        numbers.append(number)
    numerator,denominator=numbers
    if denominator==0:return None,'DIVZERO'
    if numerator==-(2**63) and denominator==-1:return None,'OVERFLOW'
    quotient=abs(numerator)//abs(denominator)
    return (-quotient if (numerator<0)!=(denominator<0) else quotient),None


def expected_report(raw):
    values=[];rows=0;errors={'AIL-RUNTIME-'+kind:0 for kind in ['DIVZERO','JSON','JSON-PATH','OVERFLOW']}
    for line in raw.split('\n'):
        line=line.strip(WHITESPACE)
        if not line:continue
        rows+=1;value,error=row_result(line)
        if error:errors['AIL-RUNTIME-'+error]+=1
        else:values.append(value)
    stdout=''.join(canonical({'value':value})+'\n' for value in values).encode()
    stderr=(canonical(dict(rows=rows,accepted=len(values),rejected=rows-len(values),errors=errors))+'\n').encode()
    return stdout,stderr,len(values)


def filter_reports(root,rng):
    artifacts=routes(root,REPO/'examples/json-lines-filter','filter','Filter')
    fixed=['{"value":8,"divisor":4}','{"value":-7,"divisor":2}','{"value":1,"divisor":0}','bad','{"value":1}','{"value":"1","divisor":2}','{"value":-9223372036854775808,"divisor":-1}','{"value":1,"value":2,"divisor":1}','{"value":1,"\\u0076alue":1,"divisor":1}','{"value":1.0,"divisor":1}','{"value":9223372036854775808,"divisor":1}','{"value":1,"divisor":true}','{"value":1,"divisor":1,"unused":"\\ud800"}','{"value":1,"divisor":1,"unused":1e9999999999999}','[]','null','false',' \t\u0085\u3000 ']
    last=None
    for case in range(60):
        rows=fixed.copy() if case==0 else []
        for _ in range(0 if case==1 else rng.randrange(35)):
            rows.append(rng.choice(fixed) if rng.randrange(2) else json.dumps(dict(value=rng.choice([-2**63,2**63-1,rng.randrange(-10**12,10**12)]),divisor=rng.choice([-1,0,1,rng.randrange(-100,101)]))))
        raw=('\r\n' if case%2 else '\n').join(rng.choice(['',' ','\u0085'])+row+rng.choice(['',' ','\u3000']) for row in rows)+'\n'
        expected=expected_report(raw)
        if case==0:last=(raw,*expected)
        for artifact,binary in artifacts:
            for backend in ['VM','native']:
                argv=[AIL,'ail-vm',artifact,'--action','Filter','--stdio'] if backend=='VM' else [binary,'--stdio']
                exact(invoke(argv,raw.encode()),expected[0],expected[1])
    for artifact,binary in artifacts:
        for backend in ['VM','native']:
            argv=[AIL,'ail-vm',artifact,'--action','Filter','--stdio'] if backend=='VM' else [binary,'--stdio']
            fatal(invoke(argv,b'\xff'),'TEXT');fatal(invoke(argv,b'x'*(LIMIT+1)),'TEXT-LIMIT')
    print('PASS: 60 independently calculated JSON-lines filters across 12 routes, exact stdout records/stderr summaries and fatal input validation before output',flush=True)
    return artifacts,last


COUNTER='''The application Counter manages lines.
A Data has:
- raw: Text
- separator: Text
- lines: List<Text>
- line: Text
- length: Int
- index: Int
- count: Int
- accepted: Int
- result: Text
Action: Run.
When Run happens:
- the system reads text from standard input into data.raw
- the system assigns "\\n" to data.separator
- the system splits text data.raw by data.separator into data.lines
- the system counts items in list data.lines into data.count
- the system assigns 0 to data.index
- the system assigns 0 to data.accepted
- while data.index < data.count:
- the system gets item data.index from list data.lines into data.line
- the system counts characters in data.line into data.length
- if data.length > 0:
- the system increments data.accepted by 1
- end if
- the system increments data.index by 1
- end while
- the system formats data.accepted as JSON text into data.result
- the system writes text data.result to standard output
- the system records a trace event named Counted
'''


def actual_pipeline(root,filters,example):
    counter=routes(root,package(root,'counter',COUNTER),'counter')[0]
    raw,_,summary,count=example
    for first in ['VM','native']:
        for second in ['VM','native']:
            argv1=[AIL,'ail-vm',filters[0][0],'--action','Filter','--stdio'] if first=='VM' else [filters[0][1],'--stdio']
            argv2=[AIL,'ail-vm',counter[0],'--action','Run','--stdio'] if second=='VM' else [counter[1],'--stdio']
            p1=subprocess.Popen(argv1,cwd=REPO,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            p2=subprocess.Popen(argv2,cwd=REPO,stdin=p1.stdout,stdout=subprocess.PIPE,stderr=subprocess.PIPE);p1.stdout.close()
            error=[]
            def drain():error.append(p1.stderr.read())
            def feed():
                try:p1.stdin.write(raw.encode());p1.stdin.close()
                except BrokenPipeError:pass
            drainer=threading.Thread(target=drain);feeder=threading.Thread(target=feed);drainer.start();feeder.start()
            try:out,err=p2.communicate(timeout=30);p1.wait(timeout=10)
            except subprocess.TimeoutExpired:
                p1.kill();p2.kill();p1.wait();p2.wait();raise
            finally:drainer.join(timeout=2);feeder.join(timeout=2)
            assert p1.returncode==p2.returncode==0,(first,second,p1.returncode,p2.returncode,error,err)
            assert out==str(count).encode() and err==b'' and error==[summary],(out,err,error,count)
    print('PASS: four actual two-process AIL filter → AIL count pipelines across VM/native combinations',flush=True)


def main():
    rng=random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix='ail-stdio-oracle-') as directory:
        root=Path(directory)
        transport(root,rng)
        echo=primitive_errors(root)
        pipe_harness(root,echo)
        shared_input_cursor(root,echo)
        descriptor_failures(root,echo)
        budgets_and_cleanup(root)
        filters,example=filter_reports(root,rng)
        actual_pipeline(root,filters,example)
    print(f'PASS: independent standard-stream oracle, seed={SEED}',flush=True)


if __name__=='__main__':main()
