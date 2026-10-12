#!/usr/bin/env python3
"""Independent typed-literal oracle. Run cargo build first.

Checks VM/native values, immutable copies, malformed inputs and exact limits.
Leaves the frozen numeric capability ladder unchanged.
"""
import json, pathlib, random, subprocess, tempfile
REPO=pathlib.Path(__file__).resolve().parent.parent
AIL=str(REPO/'target/debug/ail')
def invoke(args):return subprocess.run(args,cwd=REPO,capture_output=True,timeout=30)
def op(code,**operands):return {'opcode':code,'operands':operands}
def program(steps):return dict(kind='AIL-Bytecode',package='literal-review',version='1',profile='Application',capability_grants=[],target_support={},external_bindings=[],failures=[],actions=[dict(action='Run',instructions=steps)])
def quoted(text):
    escapes={'"':r'\"','\\':r'\\','\n':r'\n','\r':r'\r','\t':r'\t'}
    return '"'+''.join(escapes[c] if c in escapes else ('\\u%04x'%ord(c)) if ord(c)<32 or 127<=ord(c)<=159 else c for c in text)+'"'
def canonical(value):
    if type(value)==bool:return str(value).lower()
    if type(value)==int:return str(value)
    if type(value)==str:return quoted(value)
    return '['+','.join(canonical(v) for v in value)+']'
def transport(value):return value if type(value)==str else canonical(value)
def write_program(root,name,steps):
    source=root/(name+'.json');source.write_text(json.dumps(program(steps)));return source

def compile_program(source):
    binary=source.with_suffix('.elf')
    result=invoke([AIL,'ail-compile',str(source),'--action','Run','--target','linux-x86_64-elf','--out',str(binary)])
    assert result.returncode==0,(result.returncode,result.stdout,result.stderr)
    return binary

def check_program(root,name,steps,expected):
    source=write_program(root,name,steps+[op('OUTPUT_FIELD',key=k) for k in sorted(expected)]+[op('RETURN_SUCCESS')])
    vm=invoke([AIL,'ail-vm',str(source),'--action','Run'])
    native=invoke([str(compile_program(source))])
    wanted=''.join(k+'='+transport(v)+'\n' for k,v in sorted(expected.items())).encode()
    assert vm.returncode==0 and vm.stdout.startswith(b'ail-vm succeeded\n'+wanted),(name,'VM',vm.returncode,vm.stdout,vm.stderr,wanted)
    assert native.returncode==0 and native.stdout==wanted,(name,'native',native.returncode,native.stdout,native.stderr,wanted)

with tempfile.TemporaryDirectory(prefix='ail-literal-independent-review-') as scratch:
    root=pathlib.Path(scratch);random.seed(48279)
    chars=['a',' ','é','🙂','\U0010ffff','\u0301','\x00','\x01','\b','\f','\t','\n','\r','\x7f','\x80','\x85','\x9f','\xa0','"','\\','/','=','[',']',',',' to ','the system','data.value']
    scalar=lambda: ''.join(random.choices(chars,k=random.randrange(15)))
    integer=lambda:random.choice([-2**63,2**63-1,0,random.randrange(-1000000,1000000)])
    count=0
    for batch in range(10):
        steps=[];expected={}
        for idx in range(50):
            typ=['Int','Bool','Text','List<Int>','List<Text>'][idx%5]
            value=integer() if typ=='Int' else random.choice([True,False]) if typ=='Bool' else scalar() if typ=='Text' else [integer() if typ=='List<Int>' else scalar() for _ in range(random.randrange(8))]
            replacement=1 if typ=='Int' else False if typ=='Bool' else 'changed' if typ=='Text' else []
            literal=json.dumps(value,ensure_ascii=(idx+batch)%2==0,separators=(',',':') if idx%3 else (', ',':'))
            if idx%4==0:literal=' \r\n'+literal+'\t '
            if typ=='Int' and value==0 and idx%2==0:literal=' -0 '
            key=f'data.value{idx}';saved=f'out.saved{idx}';current=f'out.current{idx}'
            steps.extend([op('ASSIGN_LITERAL',key=key,type=typ,value=literal),op('COPY_FIELD',source=key,key=saved,text='copy constant'),op('ASSIGN_LITERAL',key=key,type=typ,value=json.dumps(replacement)),op('COPY_FIELD',source=key,key=current,text='copy replacement')])
            expected[saved]=value;expected[current]=replacement
            if typ=='Int':
                derived=f'out.derived{idx}';steps.append(op('EVAL_INT',key=derived,expression='f:'+saved+' i:1 /'));expected[derived]=value
            elif typ=='Text':
                derived=f'out.derived{idx}';steps.append(op('TEXT_LENGTH',key=derived,source=saved));expected[derived]=len(value)
            elif typ.startswith('List'):
                derived=f'out.derived{idx}';steps.append(op('LIST_LENGTH',key=derived,source=saved,type='Int' if typ=='List<Int>' else 'Text'));expected[derived]=len(value)
            count+=1
        check_program(root,'values_'+str(batch),steps,expected)
    print(f'PASS: {count} VM/native/Python typed literal values, immutable copies, reassignment, and scalar/list/text composition')
    malformed={
      'Int':['','null','true','"1"','[]','{}','+1','01','-01','1.0','1e0','NaN','Infinity','9223372036854775808','-9223372036854775809','\x851','1\x85','1 2','--1','-'],
      'Bool':['','0','1','"true"','True','FALSE','null','[]','{}','true false','\vtrue'],
      'Text':['','null','[]','{}','true','1','"unterminated','"\\ud800"','"\\udc00"','"\\ud800\\u0000"','"\\u+041"','"\\u004"','"\\x00"','"raw\nline"','"raw\x00nul"','"ok" trailing','\x85"ok"','"ok"\x85'],
      'List<Int>':['','null','{}','[[1]]','[true]','[null]','[,]','[1,]','[1] trailing','[+1]','[01]','[-01]','[1.]','[1.0]','[1e0]','[NaN]','[Infinity]','[9223372036854775808]','[-9223372036854775809]','[1,"wrong"]','[\v1]','[\x851]'],
      'List<Text>':['[1]','["ok",1]','["\\ud800"]','["\\udc00"]','["\\ud800\\u0000"]','["\\u+041"]','["\\u004"]','["\\x00"]','["raw\nline"]','["unterminated]','[["x"]]','["x",]','["x"] junk','\x85["x"]']}
    invalids=0
    for typ,values in malformed.items():
        for literal in values:
            source=write_program(root,'invalid',[op('ASSIGN_LITERAL',key='out.value',type=typ,value=literal),op('OUTPUT_FIELD',key='out.value'),op('RETURN_SUCCESS')])
            for cmd in [[AIL,'ail-vm',str(source),'--action','Run'],[AIL,'ail-compile',str(source),'--action','Run','--target','linux-x86_64-elf','--out',str(root/'invalid.elf')]]:
                result=invoke(cmd)
                assert result.returncode!=0 and b'AIL-LITERAL-001:' in result.stdout+result.stderr,(typ,literal,result.returncode,result.stdout,result.stderr)
            invalids+=1
    print(f'PASS: {invalids} malformed/mistyped literal cases rejected by VM and native compilation')

LIMIT=1_048_576
with tempfile.TemporaryDirectory(prefix='ail-literal-boundary-review-') as scratch:
    root=pathlib.Path(scratch)
    cases=[
      ('text_ascii_exact','Text','"'+'x'*LIMIT+'"',True,LIMIT),
      ('text_ascii_over','Text','"'+'x'*(LIMIT+1)+'"',False,None),
      ('text_utf8_exact','Text','"'+'🙂'*(LIMIT//4)+'"',True,LIMIT//4),
      ('text_utf8_over','Text','"'+'🙂'*(LIMIT//4)+'x"',False,None),
      ('text_raw_exact','Text','"'+'\\u0000'*LIMIT+'"',True,LIMIT),
      ('text_raw_over','Text',' "'+'\\u0000'*LIMIT+'"',False,None),
      ('list_bytes_exact','List<Text>','["'+'x'*(LIMIT-4)+'"]',True,1),
      ('list_bytes_over','List<Text>','["'+'x'*(LIMIT-3)+'"]',False,None),
      ('list_raw_controls_expand','List<Text>','["'+'\x7f'*200000+'"]',False,None),
      ('list_controls_exact','List<Text>','["'+'\x7f'*174762+'"]',True,1),
      ('list_controls_over','List<Text>','["'+'\x7f'*174763+'"]',False,None),
      ('list_count_exact','List<Int>','['+','.join(['0']*65536)+']',True,65536),
      ('list_count_over','List<Int>','['+','.join(['0']*65537)+']',False,None),
    ]
    for name,typ,literal,valid,count in cases:
        steps=[op('ASSIGN_LITERAL',key='data.value',type=typ,value=literal)]
        if typ=='Text':steps.append(op('TEXT_LENGTH',source='data.value',key='out.count'))
        else:steps.append(op('LIST_LENGTH',source='data.value',type=typ[5:-1],key='out.count'))
        if valid:check_program(root,name,steps,{'out.count':count})
        else:
            source=write_program(root,name,steps+[op('RETURN_SUCCESS')])
            for cmd in [[AIL,'ail-vm',str(source),'--action','Run'],[AIL,'ail-compile',str(source),'--action','Run','--target','linux-x86_64-elf','--out',str(root/(name+'.elf'))]]:
                result=invoke(cmd)
                assert result.returncode!=0 and b'AIL-LITERAL-001:' in result.stdout+result.stderr,(name,result.returncode,result.stdout,result.stderr)
        print('PASS:',name,flush=True)
    print(f'PASS: {len(cases)} exact/overflow constant byte, decoded UTF-8, canonical JSON, and item-count boundaries')

with tempfile.TemporaryDirectory(prefix='ail-literal-source-review-') as scratch:
    root=pathlib.Path(scratch);package=root/'source';package.mkdir()
    meta=(REPO/'examples/typed-constants/ail-package.md').read_text();(package/'ail-package.md').write_text(meta)
    random.seed(43214)
    chars=['x',' to ',' into ','the system assigns ','data.secret','# Heading','\u2028','\u2029','\x00','\n','\r','\t','[literal=\"a\",value_type=Int]','🙂','é','"','\\','\x7f','\x85','.']
    values=[];fields=[];statements=[];expected={}
    for idx in range(50):
        typ=['Int','Bool','Text','List<Int>','List<Text>'][idx%5]
        integer=lambda:random.choice([-2**63,2**63-1,0,random.randrange(-99999,99999)])
        text=lambda:''.join(random.choices(chars,k=random.randrange(1,15)))
        value=integer() if typ=='Int' else random.choice([True,False]) if typ=='Bool' else text() if typ=='Text' else [integer() if typ=='List<Int>' else text() for _ in range(random.randrange(5))]
        key=f'output.v{idx}';fields.append(f'- v{idx}: {typ}')
        statements.append(f'- the system assigns {json.dumps(value,ensure_ascii=idx%2==0)} to {key}')
        expected[key]=value
    spec='The application Literals manages values.\nAn Output has:\n'+'\n'.join(fields)+'\n\nAction: Run.\nWhen Run happens:\n'+'\n'.join(statements)+'\n'+'\n'.join('- the system returns '+key for key in expected)+'\n- the system records a trace event named Done\n'
    (package/'spec.ail-spec.md').write_text(spec)
    wanted=''.join(k+'='+transport(v)+'\n' for k,v in sorted(expected.items())).encode()
    core_result=invoke([AIL,'ail-core',str(package)])
    assert core_result.returncode==0,(core_result.stdout,core_result.stderr)
    core=root/'saved.core';core.write_bytes(core_result.stdout)
    rendered=invoke([AIL,'ail-spec','--core-file',str(core)])
    assert rendered.returncode==0,(rendered.stdout,rendered.stderr)
    rendered_package=root/'rendered';rendered_package.mkdir();(rendered_package/'ail-package.md').write_text(meta);(rendered_package/'spec.ail-spec.md').write_bytes(rendered.stdout)
    for label,sourceargs in [('source',[str(package)]),('savedCore',['--core-file',str(core)]),('renderedSource',[str(rendered_package)])]:
        lowered=invoke([AIL,'ail-lower']+sourceargs)
        assert lowered.returncode==0,(label,lowered.stdout,lowered.stderr)
        bc=root/(label+'.json');bc.write_bytes(lowered.stdout)
        vm=invoke([AIL,'ail-vm',str(bc),'--action','Run'])
        assert vm.returncode==0 and vm.stdout.startswith(b'ail-vm succeeded\n'+wanted),(label,vm.stdout,vm.stderr,wanted)
        binary=root/(label+'.elf');compiled=invoke([AIL,'ail-compile']+sourceargs+['--action','Run','--target','linux-x86_64-elf','--out',str(binary)])
        assert compiled.returncode==0,(label,compiled.stdout,compiled.stderr)
        native=invoke([str(binary)])
        assert native.returncode==0 and native.stdout==wanted,(label,native.stdout,native.stderr,wanted)
        ste=invoke([AIL,'ail-ste']+sourceargs)
        assert ste.returncode==0,(label,ste.stdout,ste.stderr)
        artifact=root/(label+'.ail-ste');artifact.write_bytes(ste.stdout)
        stevm=invoke([AIL,'ail-vm',str(artifact),'--action','Run'])
        assert stevm.returncode==0 and stevm.stdout.startswith(b'ail-vm succeeded\n'+wanted),(label,stevm.stdout,stevm.stderr,wanted)
    print('PASS: 50 independent typed source literals with opaque keywords/field names/Core syntax/NUL/Unicode survive source, saved Core, rendered source, STE, VM, and native ELF')
