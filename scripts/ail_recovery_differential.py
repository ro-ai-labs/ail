#!/usr/bin/env python3
"""Independent structured-recovery oracle for AIL VM and native executables.

Run cargo build first. Python models termination-style category handlers, state
commit and stable error codes independently. Native resource probes use bounded
address space/file descriptors. Children drain pipes and have hard timeouts.
"""
import json
from pathlib import Path
import random
import resource
import subprocess
import tempfile

REPO=Path(__file__).resolve().parent.parent
AIL=REPO/'target/debug/ail'
LIMIT=1_048_576
SEED=0xAEC0AE12
CATEGORIES=['INT','OVERFLOW','DIVZERO','FLOAT','BOOL','TEXT','LIST','INDEX','JSON','JSON-PATH']

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


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=30)


def checked(arguments):
    result = invoke(arguments)
    assert result.returncode == 0, (arguments[:4], result.stdout[:1000], result.stderr[-2000:])
    return result.stdout


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions, routines=()):
    source = root / f"{name}.json"
    source.write_text(json.dumps({"kind": "AIL-Bytecode", "package": "recovery-oracle", "version": "1", "profile": "Application", "capability_grants": [], "target_support": {}, "external_bindings": [], "failures": [], "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}] + list(routines)}))
    binary = root / f"{name}.elf"
    checked([AIL, "ail-compile", source, "--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
    return source, binary


def run(program, backend, arguments=()):
    source, binary = program
    prefix = [AIL, "ail-vm", source, "--action", "Run"] if backend == "VM" else [binary]
    return invoke(prefix + list(arguments))


def succeeded(result, backend, expected):
    assert result.returncode == 0, (backend, result.stdout[:1000], result.stderr[-2000:])
    if backend == "VM":
        assert result.stdout.startswith(b"ail-vm succeeded\n" + expected), (backend, result.stdout[:1000], expected[:1000])
    else:
        assert result.stdout == expected, (backend, result.stdout[:1000], expected[:1000])


def failed(result, backend, category):
    marker = f"AIL-RUNTIME-{category}:".encode()
    assert result.returncode != 0, (backend, result.stdout[:1000], result.stderr[-1000:])
    assert marker in result.stdout + result.stderr, (backend, category, result.stdout[:1000], result.stderr[-2000:])
    if backend == "native":
        assert not result.stdout, result.stdout[:1000]


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




def begin(errors='any',key='out.error'):
    return op('TRY_BEGIN',errors=errors,key=key)


def recover():return op('RECOVER')
def end():return op('TRY_END')
def code(category):return 'AIL-RUNTIME-'+category


def failure(category,key='data.atomic'):
    return {
        'INT':op('EVAL_INT',key=key,expression='f:missing.integer'),
        'OVERFLOW':op('EVAL_INT',key=key,expression='i:9223372036854775807 i:1 +'),
        'DIVZERO':op('EVAL_INT',key=key,expression='i:1 i:0 /'),
        'FLOAT':op('EVAL_FLOAT',key=key,expression='f:missing.floating'),
        'BOOL':op('EVAL_BOOL',key=key,expression='bf:missing.flag'),
        'TEXT':op('TEXT_TRIM',key=key,source='missing.text'),
        'LIST':op('LIST_LENGTH',key=key,source='input.bad',type='Int'),
        'INDEX':op('LIST_GET',key=key,source='input.empty',index='i:0',type='Int'),
        'JSON':op('JSON_PARSE',key=key,source='input.bad',type='Int'),
        'JSON-PATH':op('JSON_GET',key=key,source='input.document',path='input.path',type='Int'),
    }[category]


def fixtures():
    return [seed('input.bad','PRIVATE_INVALID_VALUE'),seed('input.empty','[]'),seed('input.document','{}'),seed('input.path','/missing'),seed('data.atomic','sentinel')]


class Caught(Exception):pass


def evaluate(nodes,state):
    for node in nodes:
        if node[0]=='add':state['out.count']+=node[1]
        elif node[0]=='fail':raise Caught(node[1])
        else:
            _,filters,key,body,handler=node
            try:evaluate(body,state)
            except Caught as error:
                category=str(error)
                if filters!='any' and category not in filters:raise
                state[key]=code(category)
                evaluate(handler,state)


def emit(nodes):
    steps=[]
    for node in nodes:
        if node[0]=='add':steps.append(op('ADD_INT_FIELD',key='out.count',delta=str(node[1]),text='oracle increment'))
        elif node[0]=='fail':steps.append(failure(node[1]))
        else:
            _,filters,key,body,handler=node
            steps += [begin('any' if filters=='any' else ';'.join(sorted(filters)),key)]+emit(body)+[recover()]+emit(handler)+[end()]
    return steps


def random_nodes(rng,keys,depth=0):
    nodes=[('add',rng.randrange(-5,10))]
    if depth<3 and rng.randrange(3):
        key=f'out.error{len(keys):03}';keys.append(key)
        filters='any' if rng.randrange(3)==0 else rng.sample(CATEGORIES,rng.randrange(1,5))
        nodes.append(('try',filters,key,random_nodes(rng,keys,depth+1),random_nodes(rng,keys,depth+1)))
    elif rng.randrange(4):nodes.append(('fail',rng.choice(CATEGORIES)))
    nodes.append(('add',rng.randrange(-5,10)))
    return nodes


def generated_handlers(root,rng):
    for batch in range(20):
        steps=fixtures();expected={}
        for index in range(15):
            keys=[]
            body=random_nodes(rng,keys)
            root_key=f'out.error{len(keys):03}';keys.append(root_key)
            tree=[('try','any',root_key,body,[('add',7)])]
            state={'out.count':0,**{key:'unchanged' for key in keys}}
            evaluate(tree,state)
            steps += [seed('out.count','0')]+[seed(key,'unchanged') for key in keys]+emit(tree)
            for key,value in sorted(state.items()):
                destination=f'out.case{index:02}_{key[4:]}'
                steps += [copy(destination,key),output(destination)];expected[destination]=str(value)
        steps += [copy('out.atomic','data.atomic'),output('out.atomic')];expected['out.atomic']='sentinel'
        program=compile_program(root,f'generated{batch}',steps)
        wanted=''.join(f'{key}={value}\n' for key,value in sorted(expected.items())).encode()
        for backend in ['VM','native']:succeeded(run(program,backend),backend,wanted)
    print('PASS: 300 independent nested handler trees, specific/nonmatching/any filters, handler failures, successful prior writes and atomic failed destinations',flush=True)


def category_matrix(root):
    for category in CATEGORIES:
        for filter_ in [category,'any',';'.join(sorted(CATEGORIES))]:
            program=compile_program(root,f'category_{category}_{len(filter_)}',fixtures()+[seed('out.value','41'),begin(filter_),failure(category,'out.value'),seed('out.value','wrong'),recover(),end(),output('out.error'),output('out.value')])
            for backend in ['VM','native']:
                result=run(program,backend);succeeded(result,backend,f'out.error={code(category)}\nout.value=41\n'.encode());assert (code(category)+':').encode() not in result.stderr
    program=compile_program(root,'normal', [seed('out.error','untouched'),begin(),seed('out.value','7'),recover(),seed('out.value','wrong'),end(),output('out.error'),output('out.value')])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.error=untouched\nout.value=7\n')
    # Use a computed Float's mutable native numeric buffer, then preserve both
    # the destination and a borrowed alias after the failing replacement.
    program=compile_program(root,'computed_float_atomic',[op('EVAL_FLOAT',key='out.value',expression='d:-12.25'),copy('out.alias','out.value'),begin('DIVZERO'),op('EVAL_FLOAT',key='out.value',expression='d:1 d:0 /'),recover(),end(),output('out.alias'),output('out.error'),output('out.value')])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.alias=-1.225e1\nout.error=AIL-RUNTIME-DIVZERO\nout.value=-1.225e1\n')
    print('PASS: all ten recoverable categories with specific/any/full masks, static handler codes, silent native catches, normal-handler skipping and computed Float alias atomicity',flush=True)


def function_frames(root):
    integer={'action':'integerFail','instructions':[op('INTEGER_FUNCTION',parameters='n'),op('RETURN_INT',expression='i:10 f:local.n /')]}
    recurse={'action':'recurse','instructions':[op('VALUE_FUNCTION',parameters='n:Int',locals='result:Int',result='Int'),op('BRANCH_INT',left='f:local.n',comparison='>',right='i:0',label='more'),op('CALL_INT',target='integerFail',arguments='n=i:0',key='local.result'),op('RETURN_VALUE',type='Int',expression='f:local.result'),op('LABEL',name='more'),op('CALL_VALUE',target='recurse',arguments='n=f:local.n i:1 -',key='local.result'),op('RETURN_VALUE',type='Int',expression='f:local.result')]}
    safe={'action':'safe','instructions':[op('VALUE_FUNCTION',parameters='n:Int',locals='error:Text;result:Int',result='Int'),begin('DIVZERO','local.error'),op('RETURN_VALUE',type='Int',expression='i:10 f:local.n /'),recover(),op('RETURN_VALUE',type='Int',expression='i:7'),end()]}
    steps=fixtures()+[seed('out.value','41'),seed('out.count','0')]
    for _ in range(160):steps += [begin('DIVZERO'),op('CALL_VALUE',target='recurse',arguments='n=i:20',key='out.value'),recover(),op('ADD_INT_FIELD',key='out.count',delta='1',text='caught'),end()]
    steps += [op('CALL_VALUE',target='safe',arguments='n=i:0',key='out.safe'),output('out.error'),output('out.value'),output('out.count'),output('out.safe')]
    program=compile_program(root,'frames',steps,[integer,recurse,safe])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.count=160\nout.error=AIL-RUNTIME-DIVZERO\nout.safe=7\nout.value=41\n')
    # Successful return removes its handler; later caller errors cannot jump into a dead frame.
    returning={'action':'returns','instructions':[op('VALUE_FUNCTION',parameters='',locals='error:Text',result='Int'),begin('INT','local.error'),op('RETURN_VALUE',type='Int',expression='i:3'),recover(),op('RETURN_VALUE',type='Int',expression='i:99'),end()]}
    program=compile_program(root,'stale_return',[op('CALL_VALUE',key='out.value',target='returns',arguments=''),begin('INT'),failure('INT'),recover(),end(),output('out.error'),output('out.value')],[returning])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.error=AIL-RUNTIME-INT\nout.value=3\n')
    for name,steps in [('returned',[op('CALL_VALUE',key='out.value',target='returns',arguments='')]),('normal',[begin('INT'),seed('out.value','3'),recover(),end()])]:
        program=compile_program(root,'stale_uncaught_'+name,steps+[failure('INT')],[returning] if name=='returned' else [])
        for backend in ['VM','native']:failed(run(program,backend),backend,'INT')
    # Legacy Bool argument classification remains JSON, before callee body/handler.
    unused={'action':'unused','instructions':[op('VALUE_FUNCTION',parameters='flag:Bool',locals='error:Text',result='Int'),begin('BOOL','local.error'),op('RETURN_VALUE',type='Int',expression='i:1'),recover(),op('RETURN_VALUE',type='Int',expression='i:2'),end()]}
    program=compile_program(root,'bool_argument',[seed('input.flag','invalid'),seed('out.value','41'),begin('JSON'),op('CALL_VALUE',target='unused',arguments='flag=f:input.flag',key='out.value'),recover(),end(),output('out.error'),output('out.value')],[unused])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.error=AIL-RUNTIME-JSON\nout.value=41\n')
    print('PASS: 160 repeated unwinds through 21 value frames plus an integer callee, return-expression recovery, successful-return scope disposal and Bool argument family compatibility',flush=True)


def action_and_jump_scopes(root):
    child={'action':'Child','instructions':[output('out.child'),seed('out.child','kept child write'),op('ADD_INT_FIELD',key='out.count',delta='1',text='completed iteration'),op('BRANCH_INT',left='f:out.count',comparison='==',right='i:2',label='fail'),op('JUMP',label='done'),op('LABEL',name='fail'),failure('INT'),op('LABEL',name='done'),op('RETURN_SUCCESS')]}
    steps=[seed('out.count','0'),output('out.count'),begin('INT'),op('REPEAT_ACTION',target='Child',count='3'),recover(),end(),output('out.error')]
    program=compile_program(root,'repeated_actions',steps,[child])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.count=2\nout.error=AIL-RUNTIME-INT\n')
    # Leaving an inner handler for its enclosing protected region must retain
    # that region's live handler even though the inner region has been removed.
    program=compile_program(root,'jump_from_inner_handler',fixtures()+[begin('INT'),begin('JSON'),failure('JSON'),recover(),op('JUMP',label='outer_body'),end(),op('LABEL',name='outer_body'),failure('INT'),recover(),end(),output('out.error')])
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.error=AIL-RUNTIME-INT\n')
    print('PASS: repeated-action errors retain completed iterations and restore caller output selection; jumps from inner handlers preserve the enclosing protected scope',flush=True)


def fatal_cases(root):
    cases=[('TEXT-LIMIT',[seed('input.raw','x'*(LIMIT+1))],op('TEXT_TRIM',key='out.value',source='input.raw')),('LIST-LIMIT',[seed('input.raw','['+','.join(['0']*65537)+']')],op('LIST_LENGTH',key='out.value',source='input.raw',type='Int')),('JSON-LIMIT',[seed('input.raw',' '*(LIMIT+1))],op('JSON_PARSE',key='out.value',source='input.raw',type='Json')),('FLOAT-LIMIT',[seed('input.raw','1'*(LIMIT+1))],op('EVAL_FLOAT',key='out.value',expression='f:input.raw')),('CAPABILITY',[],op('FILE_READ_TEXT',key='out.value',resource='missing'))]
    for category,setup,operation in cases:
        program=compile_program(root,'fatal_'+category,setup+[begin(),operation,recover(),seed('out.value','wrong'),end(),output('out.value')])
        for backend in ['VM','native']:failed(run(program,backend),backend,category)
    for behavior in ['return','depth','fuel']:
        routine={'action':'bad','instructions':[op('VALUE_FUNCTION',parameters='',locals='result:Text',result='Text')]}
        if behavior=='depth':routine['instructions'] += [op('CALL_VALUE',target='bad',arguments='',key='local.result'),op('RETURN_VALUE',type='Text',expression='f:local.result')]
        if behavior=='fuel':routine['instructions'] += [op('LABEL',name='again'),begin('DIVZERO','local.result'),op('EVAL_INT',key='local.result',expression='i:1 i:0 /'),recover(),end(),op('JUMP',label='again')]
        # The fuel loop is an action: local result Text cannot hold an Int computation.
        if behavior=='fuel':
            program=compile_program(root,'fatal_fuel',[op('LABEL',name='again'),begin('DIVZERO'),failure('DIVZERO'),recover(),end(),op('JUMP',label='again')]);expected='step limit'
        else:
            program=compile_program(root,'fatal_'+behavior,[begin(),op('CALL_VALUE',target='bad',arguments='',key='out.value'),recover(),end(),output('out.value')],[routine]);expected='AIL-RUNTIME-'+('RETURN' if behavior=='return' else 'DEPTH')
        for backend in ['VM','native']:
            result=run(program,backend);assert result.returncode!=0 and expected.encode() in result.stdout+result.stderr,(behavior,backend,result.stderr[-1000:])
    io=compile_program(root,'fatal_io',[begin(),op('FILE_READ_TEXT',key='out.value',resource='input'),recover(),end()])
    for backend in ['VM','native']:failed(run(io,backend,[f'--read-file=input={root/"absent"}']),backend,'IO')
    missing_output=compile_program(root,'fatal_output',[begin(),output('missing.output'),recover(),seed('missing.output','wrong'),end()])
    for backend in ['VM','native']:failed(run(missing_output,backend),backend,'OUTPUT')
    # The native arenas lazily reserve 64 MiB. A smaller address-space limit
    # forces real allocation failure inside a live handler without host OOM.
    for name,operation,category in [('text',op('TEXT_JOIN',key='out.value',left='input.raw',right='input.raw'),'TEXT-LIMIT'),('list',op('LIST_REVERSE',key='out.value',source='input.raw',type='Int'),'LIST-LIMIT')]:
        program=compile_program(root,'fatal_allocation_'+name,[seed('input.raw','[1]'),begin(),operation,recover(),seed('out.value','wrong'),end(),output('out.value')])
        failed(restricted_native(program,address_space=32*1024*1024),'native',category)
    print('PASS: fatal Text/List/JSON/Float limits, native allocation failure, capability/IO/output, missing return, depth exhaustion and caught-error loops still exhausting cumulative fuel',flush=True)


def arena_fill(text=False):
    value='x'*LIMIT if text else '"'+'x'*(LIMIT-2)+'"'
    return [seed('input.large',value),seed('input.empty','')]+[op('TEXT_JOIN',key='data.fill',left='input.large',right='input.empty') if text else op('JSON_PARSE',key='data.fill',source='input.large',type='Json') for _ in range(64)]


def allocations(root):
    # Handler codes are static; catching after both arenas are full remains legal.
    steps=arena_fill(True)+arena_fill(False)+[seed('out.value','41')]
    for _ in range(80):steps += [begin('INT'),failure('INT','out.value'),recover(),end()]
    steps += [output('out.error'),output('out.value')]
    program=compile_program(root,'handler_static_codes',steps)
    for backend in ['VM','native']:succeeded(run(program,backend),backend,b'out.error=AIL-RUNTIME-INT\nout.value=41\n')
    # Successful allocations before a throw are retained, not refunded.
    for text in [False,True]:
        full=arena_fill(text);prefix=full[:2];produce=full[2]
        steps=prefix
        for _ in range(64):steps += [begin('DIVZERO'),produce,failure('DIVZERO'),recover(),end()]
        steps += [begin(),produce,recover(),end()]
        program=compile_program(root,'no_refund_'+str(text),steps)
        for backend in ['VM','native']:failed(run(program,backend),backend,'TEXT-LIMIT' if text else 'JSON-LIMIT')
    print('PASS: static handler codes with both 64 MiB arenas full and successful Text/JSON allocations retained across recovery',flush=True)


def restricted_native(program,arguments=(),address_space=256*1024*1024,descriptors=32):
    def limits():
        resource.setrlimit(resource.RLIMIT_AS,(address_space,address_space))
        resource.setrlimit(resource.RLIMIT_NOFILE,(descriptors,descriptors))
    return subprocess.run([program[1]]+list(arguments),cwd=REPO,capture_output=True,timeout=30,preexec_fn=limits)


def scratch_cleanup(root):
    invalid_file=root/'invalid utf8.bin';invalid_file.write_bytes(b'x'*(LIMIT-1)+b'\xff')
    cases=[('json',seed('input.raw','['+'0,'*20000+'false,]'),op('JSON_PARSE',key='data.value',source='input.raw',type='Json'),'JSON'),('list',seed('input.raw','['+'"ok",'*20000+'false]'),op('LIST_REVERSE',key='data.value',source='input.raw',type='Text'),'LIST'),('json_path',seed('input.raw','{"key":['+'0,'*20000+'0]}'),op('JSON_GET',key='data.value',source='input.raw',path='input.path',type='Int'),'JSON-PATH'),('file',seed('input.raw','unused'),op('FILE_READ_TEXT',key='data.value',resource='input'),'TEXT')]
    for name,setup,operation,category in cases:
        steps=[setup,seed('input.path','/missing'),seed('out.count','0'),seed('data.value','prior'),op('LABEL',name='again'),begin(category),operation,recover(),op('ADD_INT_FIELD',key='out.count',delta='1',text='caught'),end(),op('BRANCH_INT',left='f:out.count',comparison='<',right='i:800',label='again'),output('out.count'),copy('out.value','data.value'),output('out.value')]
        program=compile_program(root,'cleanup_'+name,steps)
        args=[f'--read-file=input={invalid_file}'] if name=='file' else []
        succeeded(restricted_native(program,args),'native',b'out.count=800\nout.value=prior\n')
    print('PASS: 800 repeated malformed JSON/list/path/file errors each under native 256 MiB address-space and 32-descriptor limits; scratch/FD cleanup and destination atomicity',flush=True)


def public_source(root):
    package=root/'source';package.mkdir()
    (package/'ail-package.md').write_text('# AIL Package\nname: recovery-oracle\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, functions, traces\nconformance: first-slice\n')
    (package/'spec.ail-spec.md').write_text('''The application Recovery manages records.
A Data has:
- count: Int
- value: Int
- error: Text
An Output has:
- count: Int
- value: Int
Function: safe.
The function needs:
- n: Int
The function produces:
- result: Int
The function stores:
- error: Text
The function executes:
- try:
- the function returns 8 / local.n
- recover DIVZERO into local.error:
- the function returns 7
- end try
Action: Run.
When Run happens:
- the system assigns 0 to data.count
- the system assigns 0 to output.count
- while data.count < 8:
- the system increments data.count by 1
- try:
- if data.count == 2:
- continue
- end if
- if data.count == 7:
- break
- end if
- the system computes data.value as 8 / (data.count % 2)
- recover DIVZERO or INT into data.error:
- the system increments output.count by 1
- end try
- end while
- the system calls safe with n = 0 into output.value
- the system returns output.count
- the system returns output.value
- the system returns data.error
- the system records a trace event named Done
''')
    # Independently execute equivalent Python termination/recovery control.
    count=0
    for index in range(1,9):
        if index==2:continue
        if index==7:break
        try:8//(index%2)
        except ZeroDivisionError:count+=1
    expected=f'data.error=AIL-RUNTIME-DIVZERO\noutput.count={count}\noutput.value=7\n'.encode()
    for program in routes(root,package,'public'):
        for backend in ['VM','native']:succeeded(run(program,backend),backend,expected)
    print('PASS: source/saved Core/rendered source/direct STE × VM/native with source loops, continue/break, return recovery and canonical category sets',flush=True)



class Number(str):
    """Preserve lexical JSON numbers so integer extraction stays strict."""


class Pairs(list):
    """Object member order and duplicate names are part of path semantics."""


WHITESPACE="\u0009\u000a\u000b\u000c\u000d\u001c\u001d\u001e\u001f\u0020\u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000"


def reject_constant(token):raise ValueError(token)


def row_result(raw):
    try:
        value=json.loads(raw,parse_int=Number,parse_float=Number,parse_constant=reject_constant,object_pairs_hook=Pairs)
        def valid(item):
            if type(item) is str and any(0xd800<=ord(char)<=0xdfff for char in item):raise ValueError('surrogate')
            if isinstance(item,Pairs):
                for key,child in item:valid(key);valid(child)
            elif isinstance(item,list):
                for child in item:valid(child)
        valid(value)
    except (ValueError,RecursionError):return None,'JSON'
    if not isinstance(value,Pairs):return None,'JSON-PATH'
    selected=[]
    for key in ['value','divisor']:
        matches=[child for name,child in value if name==key]
        if len(matches)!=1:return None,'JSON-PATH'
        token=matches[0]
        if not isinstance(token,Number) or '.' in token or 'e' in token.lower():return None,'JSON'
        number=int(token)
        if not -(2**63)<=number<2**63:return None,'JSON'
        selected.append(number)
    numerator,denominator=selected
    if denominator==0:return None,'DIVZERO'
    if numerator==-(2**63) and denominator==-1:return None,'OVERFLOW'
    quotient=abs(numerator)//abs(denominator)
    if (numerator<0)!=(denominator<0):quotient=-quotient
    return quotient,None


def recovering_reports(root,rng):
    programs=routes(root,REPO/'examples/recovering-json-lines','json_lines','Analyze')
    input_path,report_path=root/'input file=jsonl',root/'report file=json'
    fixed=['{"value":8,"divisor":4}','{"value":-7,"divisor":2}','{"value":7,"divisor":-2}','{"value":1,"divisor":0}','bad','{"value":1}','{"value":"1","divisor":2}','{"value":-9223372036854775808,"divisor":-1}','{"value":1,"value":2,"divisor":1}','{"value":1,"divisor":2,"divisor":2}','{"value":1,"\\u0076alue":1,"divisor":1}','{"value":1.0,"divisor":1}','{"value":9223372036854775808,"divisor":1}','{"value":1,"divisor":true}','{"value":1,"divisor":1,"unused":"\\ud800"}','{"value":1,"divisor":1,"unused":1e9999999999999}','[]','null','false','{"value":1,"divisor":1,"x":0,"x":1}',' \t\u0085\u3000 ']
    for case in range(60):
        rows=fixed.copy() if case==0 else []
        for _ in range(rng.randrange(35)):
            if rng.randrange(2):rows.append(rng.choice(fixed))
            else:rows.append(json.dumps(dict(value=rng.choice([-2**63,2**63-1,rng.randrange(-10**12,10**12)]),divisor=rng.choice([-1,0,1,rng.randrange(-1000,1001)]))))
        raw=('\r\n' if case%2 else '\n').join(rng.choice(['',' ','\u0085','\u2003'])+row+rng.choice(['',' ','\u3000']) for row in rows)+'\n'
        accepted=[];errors={code(category):0 for category in ['DIVZERO','JSON','JSON-PATH','OVERFLOW']};count=0
        for line in raw.split('\n'):
            line=line.strip(WHITESPACE)
            if not line:continue
            count+=1;value,error=row_result(line)
            if error:errors[code(error)]+=1
            else:accepted.append(value)
        expected=canonical(dict(rows=count,accepted=len(accepted),rejected=count-len(accepted),values=sorted(accepted),errors=errors)).encode()
        input_path.write_bytes(raw.encode())
        for artifact,binary in programs:
            for backend in ['VM','native']:
                prefix=[AIL,'ail-vm',artifact,'--action','Analyze'] if backend=='VM' else [binary]
                result=invoke(prefix+[f'--read-file=input={input_path}',f'--write-file=report={report_path}'])
                assert result.returncode==0,(case,backend,result.stdout[:1000],result.stderr[-2000:])
                assert report_path.read_bytes()==expected,(case,backend,report_path.read_bytes()[:2000],expected[:2000])
    input_path.write_bytes(b'x'*(LIMIT+1))
    for artifact,binary in programs:
        for backend in ['VM','native']:
            report_path.write_bytes(b'previous report')
            prefix=[AIL,'ail-vm',artifact,'--action','Analyze'] if backend=='VM' else [binary]
            result=invoke(prefix+[f'--read-file=input={input_path}',f'--write-file=report={report_path}'])
            failed(result,backend,'TEXT-LIMIT');assert report_path.read_bytes()==b'previous report'
    print('PASS: 60 independent JSON-lines datasets across 12 routes, strict i64/toward-zero arithmetic, duplicate/missing paths, all four row error counts and fatal file limits preserving prior reports',flush=True)

def main():
    rng=random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix='ail-recovery-oracle-') as directory:
        root=Path(directory)
        generated_handlers(root,rng)
        category_matrix(root)
        function_frames(root)
        action_and_jump_scopes(root)
        fatal_cases(root)
        allocations(root)
        scratch_cleanup(root)
        public_source(root)
        recovering_reports(root,rng)
    print(f'PASS: independent recovery oracle, seed={SEED}',flush=True)


if __name__=='__main__':main()
