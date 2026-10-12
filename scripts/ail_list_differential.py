#!/usr/bin/env python3
"""Compare typed lists against Python with a fixed seed. Run cargo build first.

Covers VM/native values, canonical JSON, strict input grammar and index errors.
Does not alter the frozen numeric capability ladder.
"""
import json, pathlib, random, subprocess, tempfile
REPO = pathlib.Path(__file__).resolve().parent.parent
AIL = str(REPO / 'target/debug/ail')
def invoke(args, **kw):
    return subprocess.run(args, cwd=REPO, capture_output=True, timeout=20, **kw)
def op(code, **args): return {'opcode': code, 'operands': args}
def quoted(text):
    escapes = {'"':r'\"', '\\':r'\\', '\n':r'\n', '\r':r'\r', '\t':r'\t'}
    return '"' + ''.join(escapes[c] if c in escapes else ('\\u%04x' % ord(c)) if ord(c)<32 or 127<=ord(c)<=159 else c for c in text) + '"'
def canonical(items):
    return '[' + ','.join(str(item) if type(item) is int else quoted(item) for item in items) + ']'
def program(steps):
    return dict(kind='AIL-Bytecode', package='list-review', version='1', profile='Application', capability_grants=[], target_support={}, external_bindings=[], failures=[], actions=[dict(action='Run',instructions=steps)])
def compile_steps(root, name, steps):
    source=root/(name+'.json'); source.write_text(json.dumps(program(steps)))
    binary=root/(name+'.elf')
    compiled=invoke([AIL,'ail-compile',str(source),'--action','Run','--target','linux-x86_64-elf','--out',str(binary)])
    assert compiled.returncode==0,(name,compiled.stdout,compiled.stderr)
    return source,binary
with tempfile.TemporaryDirectory(prefix='ail-list-independent-review-') as scratch:
    root=pathlib.Path(scratch)
    random.seed(4194)
    count=0
    chars=['a','b','é','🙂','\U0010ffff','\u0301','\x00','\x01','\b','\f','\t','\n','\r','\x7f','\x80','\x85','\x9f','\xa0','"','\\','/','=','[',']',',']
    scalar=lambda: ''.join(random.choices(chars,k=random.randrange(12)))
    for typ in ['Int','Text']:
        steps=[]
        if typ=='Text':
            steps.append(op('LIST_GET',type='Text',source='input.replacements',index='i:0',key='input.replacement'))
        steps += [op('COPY_FIELD',source='input.list',key='out.original',text='copy'),op('LIST_LENGTH',type=typ,source='input.list',key='out.length'),op('LIST_GET',type=typ,source='input.list',index='f:input.index',key='out.item'),op('LIST_SET',type=typ,source='input.list',index='f:input.index',value='f:input.value' if typ=='Int' else 'input.replacement',key='out.updated'),op('LIST_APPEND',type=typ,source='out.updated',value='f:input.value' if typ=='Int' else 'input.replacement',key='out.updated'),op('LIST_GET',type=typ,source='out.updated',index='i:-1',key='out.last'),op('LIST_EMPTY',type=typ,key='out.empty')]
        steps += [op('OUTPUT_FIELD',key='out.'+key) for key in ['empty','item','last','length','original','updated']]+[op('RETURN_SUCCESS')]
        source,binary=compile_steps(root,'pipeline_'+typ,steps)
        for n in range(300):
            items=[random.choice([-2**63,2**63-1,0,random.randrange(-1000000,1000000)]) if typ=='Int' else scalar() for _ in range(random.randrange(1,15))]
            value=random.choice([-2**63,2**63-1,0,random.randrange(-1000000,1000000)]) if typ=='Int' else scalar()
            index=random.randrange(-len(items),len(items))
            raw=json.dumps(items,ensure_ascii=n%2==0,separators=(',',':') if n%3 else (', ', ':'))
            if n%4==0: raw=' \r\n'+raw+'\t '
            args=['input.list='+raw,'input.index='+str(index),'out.empty=old-invalid-value']
            args += ['input.value='+str(value)] if typ=='Int' else ['input.replacements='+json.dumps([value],ensure_ascii=n%2==1)]
            updated=items.copy();updated[index]=value;updated.append(value)
            expected=f'out.empty=[]\nout.item={items[index]}\nout.last={value}\nout.length={len(items)}\nout.original={raw}\nout.updated={canonical(updated)}\n'.encode()
            vm=invoke([AIL,'ail-vm',str(source),'--action','Run']+args)
            native=invoke([str(binary)]+args)
            assert vm.returncode==0 and vm.stdout.startswith(b'ail-vm succeeded\n'+expected),(typ,n,'VM',vm.returncode,vm.stdout,expected,vm.stderr)
            assert native.returncode==0 and native.stdout==expected,(typ,n,'native',native.returncode,native.stdout,expected,native.stderr)
            count+=1
    print(f'PASS: {count} dynamic VM/native/Python list pipelines covering Int/Text get/set/append/length/empty, aliases, escaped NUL, surrogate pairs, and canonical JSON')
    malformed=['','null','{}','[[1]]','[true]','[false]','[null]','[,]','[1,]','[1] trailing','[+1]','[01]','[-01]','[1.]','[1.0]','[1e0]','[NaN]','[Infinity]','[9223372036854775808]','[-9223372036854775809]','[1, "wrong"]','[\v1]','[\u00851]']
    malformed_text=['[1]','["ok",1]','["\\ud800"]','["\\udc00"]','["\\ud800\\u0000"]','["\\u+041"]','["\\u004"]','["\\x00"]','["raw\nline"]','["unterminated]','[["x"]]','["x",]','["x"] junk','\u0085["x"]']
    invalids=0
    for typ,bad_inputs in [('Int',malformed),('Text',malformed_text)]:
        source,binary=compile_steps(root,'get_'+typ,[op('LIST_GET',type=typ,source='input.list',index='f:input.index',key='out.item'),op('OUTPUT_FIELD',key='out.item'),op('RETURN_SUCCESS')])
        for raw in bad_inputs:
            args=['input.list='+raw,'input.index=0']
            for label,result in [('VM',invoke([AIL,'ail-vm',str(source),'--action','Run']+args)),('native',invoke([str(binary)]+args))]:
                assert result.returncode!=0 and b'AIL-RUNTIME-LIST:' in result.stderr,(typ,raw,label,result.returncode,result.stdout,result.stderr)
                if label=='native':assert result.stdout==b''
            invalids+=1
        for raw,index in [('[]','0'),('[1,2]' if typ=='Int' else '["a","b"]','-3'),('[1]' if typ=='Int' else '["a"]','-9223372036854775808')]:
            args=['input.list='+raw,'input.index='+index]
            for result in [invoke([AIL,'ail-vm',str(source),'--action','Run']+args),invoke([str(binary)]+args)]:
                assert result.returncode!=0 and b'AIL-RUNTIME-INDEX:' in result.stderr,(typ,raw,index,result.stdout,result.stderr)
    print(f'PASS: {invalids} malformed/mixed/nested list inputs rejected by both backends, plus six index-error cases')


def expected_list(raw,typ):
    try:
        value=json.loads(raw,parse_constant=lambda x: (_ for _ in ()).throw(ValueError(x)))
        if type(value)!=list:return None
        if typ=='Int' and not all(type(v)==int and -(1<<63)<=v<(1<<63) for v in value):return None
        if typ=='Text' and not all(type(v)==str and not any(0xd800<=ord(c)<=0xdfff for c in v) for v in value):return None
        return str(len(value))
    except (ValueError,UnicodeError):return None
with tempfile.TemporaryDirectory(prefix='ail-list-grammar-review-') as scratch:
    root=pathlib.Path(scratch)
    random.seed(21730)
    count=0
    for typ in ['Int','Text']:
        source,binary=compile_steps(root,'length_'+typ,[op('LIST_LENGTH',type=typ,source='input.list',key='out.length'),op('OUTPUT_FIELD',key='out.length'),op('RETURN_SUCCESS')])
        examples=['[]','[0]','[-0]','[1,-2,9223372036854775807,-9223372036854775808]',' \t[ 1, 2 ]\r\n'] if typ=='Int' else ['[]','[""]','["é","🙂","\\u0000"]','["\\ud800\\udc00","\\udbff\\udfff"]','["\\b\\f\\n\\r\\t\\/\\\\\\\"","\x7f\x85"]']
        tokens=list('[]{}"\\,:+-0129aenulltruef ') + ['\t','\r','\n','\v','\x7f','\x85','é','🙂']
        for i in range(800):
            raw=random.choice(examples)
            for _ in range(random.randrange(1,4)):
                pos=random.randrange(len(raw)+1)
                if random.randrange(3)==0:raw=raw[:pos]+raw[min(len(raw),pos+1):]
                else:raw=raw[:pos]+random.choice(tokens)+raw[pos:]
            expected=expected_list(raw,typ)
            results=[('VM',invoke([AIL,'ail-vm',str(source),'--action','Run','input.list='+raw])),('native',invoke([str(binary),'input.list='+raw]))]
            for backend,result in results:
                if expected is None:
                    assert result.returncode!=0 and b'AIL-RUNTIME-LIST:' in result.stderr,(typ,raw,backend,expected,result.returncode,result.stdout,result.stderr)
                else:
                    assert result.returncode==0 and (b'out.length='+expected.encode()+b'\n') in result.stdout,(typ,raw,backend,expected,result.returncode,result.stdout,result.stderr)
                if backend=='native' and expected is None:assert result.stdout==b''
            count+=1
        if typ=='Text':
            for value in [b'\x80',b'\xc0\xaf',b'\xc2',b'\xe0\x80\xaf',b'\xed\xa0\x80',b'\xf0\x80\x80\xaf',b'\xf4\x90\x80\x80',b'\xf5\x80\x80\x80',b'\xff']:
                result=invoke([str(binary).encode(),b'input.list=["'+value+b'"]'])
                assert result.returncode==1 and result.stdout==b'' and b'AIL-RUNTIME-LIST:' in result.stderr,(value,result.returncode,result.stdout,result.stderr)
    print(f'PASS: {count} independent Python JSON grammar mutation cases agree with VM/native; 9 invalid UTF-8 native inputs fail atomically')
