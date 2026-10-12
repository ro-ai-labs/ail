#!/usr/bin/env python3
"""Independent typed-list transformation oracle; Python standard library only.

Run cargo build first. Stable sorting is calculated in Python with explicit
NaN-last partitioning in BOTH directions. Float results are checked by binary64
bits, including signed zero; Text expected encoding is independent. Every child
has a timeout and drained/capped diagnostics. Frozen benchmarks are untouched.
"""
import json
import math
from pathlib import Path
import random
import re
import struct
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576
SEED = 0x11575A07
ALPHABET = ["", "a", "aa", "é", "🙂", "\U0010ffff", "e\u0301", "\x00", "\x01", "\x7f", "\x85", "\n", "\t", "\u200b", "\ufeff", '"', "\\", "[", "]", ",", "=", "data.values", "local.source", "ascending"]

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
    source.write_text(json.dumps({"kind": "AIL-Bytecode", "package": "list-transform-oracle", "version": "1", "profile": "Application", "capability_grants": [], "target_support": {}, "external_bindings": [], "failures": [], "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}] + list(routines)}))
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



def bits(value):
    return 0x7ff8000000000000 if math.isnan(value) else struct.unpack('>Q',struct.pack('>d',value))[0]


def ordered(values, reverse=False):
    if any(type(value) is float for value in values):
        return sorted([value for value in values if not math.isnan(value)],reverse=reverse)+[value for value in values if math.isnan(value)]
    return sorted(values,reverse=reverse)


def floats(raw):
    text=raw.strip(' \t\r\n')
    assert text.startswith('[') and text.endswith(']'),raw[:100]
    return [] if not text[1:-1].strip() else [float(token) for token in text[1:-1].split(',')]


def output_fields(result,backend):
    assert result.returncode==0,(backend,result.stdout[:1000],result.stderr[-2000:])
    return dict(line.split('=',1) for line in result.stdout.decode().splitlines() if line.startswith('out.'))


def match_list(raw,expected,element,label):
    if element=='Float':
        actual=floats(raw)
        assert [bits(value) for value in actual]==[bits(value) for value in expected],(label,raw[:300],expected[:20])
        for token in raw[1:-1].split(',') if actual else []:
            if math.isnan(float(token)): assert token=='nan',(label,token)
            elif math.isinf(float(token)): assert token in ['inf','-inf'],(label,token)
            else: assert re.fullmatch(r'-?[0-9](?:\.[0-9]+)?e-?[0-9]+',token),(label,token)
    else:
        assert raw==canonical(expected),(label,raw[:1000],canonical(expected)[:1000])


def transform(opcode,element,key='data.result',**overrides):
    values=dict(key=key,source='input.left',type=element)
    if opcode=='LIST_CONCAT': values['other']='input.right'
    if opcode=='LIST_SLICE': values.update(start='f:input.start',end='f:input.end')
    if opcode=='LIST_SORT': values['order']='ascending'
    values.update(overrides)
    return op(opcode,**values)


def emit_list(values,element,ascii_only=False):
    if element=='Float': return '['+','.join(repr(value) for value in values)+']'
    return json.dumps(values,ensure_ascii=ascii_only)


def random_list(rng,element):
    if element=='Int': return [rng.choice([-2**63,2**63-1,rng.randrange(-100000,100000),0]) for _ in range(rng.randrange(35))]
    if element=='Text': return [''.join(rng.choices(ALPHABET,k=rng.randrange(7))) for _ in range(rng.randrange(35))]
    special=[math.nan,math.inf,-math.inf,-0.,0.,-1.,1.,5e-324,-5e-324,1.7976931348623157e308]
    return [rng.choice(special) if rng.randrange(2) else struct.unpack('>d',struct.pack('>Q',rng.getrandbits(64)))[0] for _ in range(rng.randrange(35))]


def batch(root,rng):
    for element in ['Int','Float','Text']:
        steps=[]
        for name,opcode,extra in [('concat','LIST_CONCAT',{}),('slice','LIST_SLICE',{}),('reverse','LIST_REVERSE',{}),('ascending','LIST_SORT',{}),('descending','LIST_SORT',{'order':'descending'})]:
            steps += [transform(opcode,element,key=f'out.{name}',**extra),output(f'out.{name}')]
            steps += [copy('data.alias','input.left'),copy('data.saved','data.alias'),transform(opcode,element,key='data.alias',source='data.alias',**extra),copy(f'out.alias_{name}','data.alias'),output(f'out.alias_{name}')]
        steps += [copy('data.alias','input.right'),transform('LIST_CONCAT',element,key='data.alias',other='data.alias'),copy('out.alias_other','data.alias'),output('out.alias_other'),transform('LIST_REVERSE',element,key='data.reverse',source='data.saved'),transform('LIST_REVERSE',element,key='out.saved',source='data.reverse'),output('out.saved')]
        program=compile_program(root,f'batch_{element}',steps)
        fixed=[([],[],0,0)]
        if element=='Float': fixed += [([math.nan,-0.,0.,-math.inf,2.,math.nan,-0.,math.inf,0.,-2.],[0.,-0.],-2**63,2**63-1)]
        if element=='Text': fixed += [(['é','e\u0301','\x00','a','aa','a','🙂'],['\x85'],0,99)]
        cases=fixed+[(random_list(rng,element),random_list(rng,element),rng.choice([-2**63,2**63-1,rng.randrange(-50,51)]),rng.choice([-2**63,2**63-1,rng.randrange(-50,51)])) for _ in range(500-len(fixed))]
        for case,(left,right,start,end) in enumerate(cases):
            expected=dict(concat=left+right,slice=left[start:end],reverse=left[::-1],ascending=ordered(left),descending=ordered(left,True),saved=left,alias_other=left+right)
            for name in ['concat','slice','reverse','ascending','descending']: expected['alias_'+name]=expected[name]
            arguments=[f'input.left={emit_list(left,element,case%2==0)}',f'input.right={emit_list(right,element,case%2!=0)}',f'input.start={start}',f'input.end={end}']
            parity=[]
            for backend in ['VM','native']:
                fields=output_fields(run(program,backend,arguments),backend)
                assert fields.keys()=={f'out.{name}' for name in expected},fields.keys()
                for name,value in expected.items():match_list(fields['out.'+name],value,element,(case,backend,element,name))
                parity.append(fields)
            assert parity[0]==parity[1],(element,case)
    print('PASS: 1,500 seeded Int/Float/Text datasets × both sorts, concat/slice/reverse and six aliases/snapshots; NaNs last, signed zeros stable, Unicode/NUL and i64 extreme bounds',flush=True)


def validation(root):
    cases=[]
    for element,valid,invalid in [('Int','[1,2]','[1,false]'),('Float','[nan,-0]','[nan,NaN]'),('Text','["ok"]','["ok",false]')]:
        cases += [(element,'LIST_SLICE',{'left':invalid},dict(start='i:999',end='i:1 i:0 /'),'LIST'),(element,'LIST_SLICE',{'left':valid},dict(start='f:missing.start',end='i:1 i:0 /'),'INT'),(element,'LIST_SLICE',{'left':valid},dict(start='i:1 i:0 /',end='f:missing.end'),'DIVZERO'),(element,'LIST_SLICE',{'left':'[]'},dict(start='i:999',end='i:1 i:0 /'),'DIVZERO'),(element,'LIST_SLICE',{'left':'[]'},dict(start='i:0',end='i:9223372036854775807 i:1 +'),'OVERFLOW'),(element,'LIST_CONCAT',{'left':valid,'right':invalid},{},'LIST'),(element,'LIST_CONCAT',{'left':valid},{},'LIST'),(element,'LIST_REVERSE',{'left':invalid},{},'LIST'),(element,'LIST_SORT',{'left':invalid},{},'LIST')]
    cases += [('Text','LIST_REVERSE',{'left':'["\\q"]'},{},'LIST'),('Text','LIST_SORT',{'left':'["\\ud800"]'},{},'LIST')]
    for index,(element,opcode,values,extra,category) in enumerate(cases):
        program=compile_program(root,f'validation{index}',[seed(f'input.{key}',value) for key,value in values.items()]+[transform(opcode,element,**extra)])
        for backend in ['VM','native']:
            failed(run(program,backend),backend,category)
    for opcode in ['LIST_CONCAT','LIST_SLICE','LIST_REVERSE','LIST_SORT']:
        program=compile_program(root,f'utf8_{opcode}',[transform(opcode,'Text',start='i:0',end='i:0') if opcode=='LIST_SLICE' else transform(opcode,'Text')])
        for invalid in [b'\xff',b'\xc0\xaf',b'\xed\xa0\x80',b'\xf4\x90\x80\x80']:
            failed(invoke([str(program[1]).encode(),b'input.left=["'+invalid+b'"]',b'input.right=[]']), 'native','LIST')
            if opcode=='LIST_CONCAT':failed(invoke([str(program[1]).encode(),b'input.left=[]',b'input.right=["'+invalid+b'"]']), 'native','LIST')
    print(f'PASS: {len(cases)} full-source/second-input/ordered-bound error cases plus 20 malformed UTF-8 probes',flush=True)


def limits(root):
    cases=[
        ('Text','LIST_REVERSE',{'left':'["'+'x'*(LIMIT-4)+'"]'},{},1,None),
        ('Text','LIST_REVERSE',{'left':'["'+'x'*(LIMIT-3)+'"]'},{},None,'LIST-LIMIT'),
        ('Int','LIST_REVERSE',{'left':'['+','.join(['0']*65536)+']'},{},65536,None),
        ('Int','LIST_REVERSE',{'left':'['+','.join(['0']*65537)+']'},{},None,'LIST-LIMIT'),
        ('Int','LIST_CONCAT',{'left':'['+','.join(['0']*65536)+']','right':'[1]'},{},None,'LIST-LIMIT'),
        ('Int','LIST_CONCAT',{'left':'['+','.join(['0']*65536)+']','right':'[false]'},{},None,'LIST'),
        ('Text','LIST_CONCAT',{'left':'["'+'\x7f'*180000+'"]','right':'[false]'},{},None,'LIST'),
    ]
    for element,raw in [('Text','["'+'\x7f'*180000+'","ok"]'),('Float','['+','.join(['1234567890123']*65536)+']')]:
        cases += [(element,'LIST_SLICE',{'left':raw},dict(start='i:-1',end='i:9223372036854775807'),1,None),(element,'LIST_SLICE',{'left':raw},dict(start='i:99',end='i:-9223372036854775808'),0,None)]
        for opcode in ['LIST_REVERSE','LIST_SORT','LIST_CONCAT']:
            cases.append((element,opcode,{'left':raw,'right':'[]'},{},None,'LIST-LIMIT'))
    # Complete validation precedes an empty slice, including raw count bounds.
    cases += [('Int','LIST_SLICE',{'left':'['+','.join(['0']*65537)+']'},dict(start='i:5',end='i:0'),None,'LIST-LIMIT'),('Text','LIST_SLICE',{'left':'["'+'x'*(LIMIT-3)+'"]'},dict(start='i:0',end='i:0'),None,'LIST-LIMIT')]
    for index,(element,opcode,values,extra,count,category) in enumerate(cases):
        program=compile_program(root,f'limits{index}',[seed('input.'+key,value) for key,value in values.items()]+[transform(opcode,element,**extra),op('LIST_LENGTH',source='data.result',key='out.count',type=element),output('out.count')])
        for backend in ['VM','native']:
            result=run(program,backend)
            if category:failed(result,backend,category)
            else:assert output_fields(result,backend)=={'out.count':str(count)}
    print(f'PASS: {len(cases)} exact byte/item/result boundaries, canonical expansion shrunk by slices and second-input priority',flush=True)


def arena_fill(reserved,text=False):
    large='x'*LIMIT if text else '"'+'x'*(LIMIT-2)+'"'
    partial='x'*(LIMIT-reserved) if text else '"'+'x'*(LIMIT-reserved-2)+'"'
    produce=lambda field:op('TEXT_JOIN',key='data.fill',left=field,right='seed.empty') if text else op('JSON_PARSE',key='data.fill',source=field,type='Json')
    return [seed('seed.large',large),seed('seed.partial',partial),seed('seed.empty','')]+[produce('seed.large') for _ in range(63)]+[produce('seed.partial')]


def arenas(root):
    for element in ['Int','Float','Text']:
        for opcode in ['LIST_CONCAT','LIST_SLICE','LIST_REVERSE','LIST_SORT']:
            extra={'source':'local.values'}
            if opcode=='LIST_CONCAT':extra['other']='local.values'
            if opcode=='LIST_SLICE':extra.update(start='i:0',end='i:0')
            routine={'action':'empty','instructions':[op('VALUE_FUNCTION',parameters=f'values:List<{element}>',locals=f'result:List<{element}>',result=f'List<{element}>'),transform(opcode,element,key='local.result',**extra),op('RETURN_VALUE',type=f'List<{element}>',expression='f:local.result')]}
            # Empty operation result [] costs 2, and returned list costs another 2.
            base=arena_fill(4)+[seed('input.left','[]'),op('CALL_VALUE',key='data.result',target='empty',arguments='values=f:input.left')]
            for extra_result in [False,True]:
                steps=base+([op('LIST_REVERSE',key='data.result',source='data.result',type=element)] if extra_result else [])+[copy('out.value','data.result'),output('out.value')]
                program=compile_program(root,f'arena_{element}_{opcode}_{extra_result}',steps,[routine])
                for backend in ['VM','native']:
                    result=run(program,backend)
                    if extra_result:failed(result,backend,'LIST-LIMIT')
                    else:succeeded(result,backend,b'out.value=[]\n')
    steps=arena_fill(0,text=True)+[seed('input.left','["é","\\u0000","🙂"]'),seed('input.right','[]')]
    for opcode in ['LIST_SORT','LIST_REVERSE','LIST_SLICE','LIST_CONCAT']:
        steps += [transform(opcode,'Text',key='input.left',**({'start':'i:-99','end':'i:99'} if opcode=='LIST_SLICE' else {}))]
    steps += [copy('out.value','input.left'),output('out.value')]
    program=compile_program(root,'no_text_charge',steps)
    for backend in ['VM','native']:succeeded(run(program,backend),backend,('out.value='+canonical(['🙂','é','\x00'])+'\n').encode())
    print('PASS: exact 64 MiB List/Json arena boundaries for all 12 operation/type combinations including one return copy; all operations after Text arena exhaustion',flush=True)


def stress(root,rng):
    cases=[]
    base=list(range(65536));shuffled=base.copy();rng.shuffle(shuffled)
    for values in [base,base[::-1],shuffled,[n%17 for n in base]]:cases.append(('Int',values))
    # 65,536 total items remain within 1MiB raw+canonical; common prefix costs are intentional.
    texts=[f'prefix{n%1000:03}' for n in range(65536)];rng.shuffle(texts)
    cases.append(('Text',texts))
    float_values=[-0. if n%5==0 else 0. if n%5==1 else math.nan if n%5==2 else float(n%100) for n in base]
    rng.shuffle(float_values);cases.append(('Float',float_values))
    for case,(element,values) in enumerate(cases):
        for direction in ['ascending','descending']:
            expected=ordered(values,direction=='descending')
            program=compile_program(root,f'stress{case}_{direction}',[seed('input.left',emit_list(values,element)),transform('LIST_SORT',element,key='out.values',order=direction),output('out.values')])
            for backend in ['VM','native']:
                fields=output_fields(run(program,backend),backend);match_list(fields['out.values'],expected,element,(case,backend,direction))
    print('PASS: twelve 65,536-item stable sorts (ordered, reversed, shuffled, duplicate, common-prefix Text and NaN/signed-zero Float) under bounded child timeouts',flush=True)


def public_source(root,rng):
    package=root/'source';package.mkdir()
    (package/'ail-package.md').write_text('# AIL Package\nname: list-transform-oracle\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, traces\nconformance: first-slice\n')
    cases=[]
    for element in ['Int','Float','Text']:
        for _ in range(12):
            left,right=random_list(rng,element),random_list(rng,element)
            if element=='Float':left=[value for value in left if math.isfinite(value)];right=[value for value in right if math.isfinite(value)]
            cases.append((element,left,right,rng.randrange(-15,15),rng.randrange(-15,15)))
    fields=[];body=[];expected={}
    for index,(element,left,right,start,end) in enumerate(cases):
        prefix=f'v{index:03}'
        fields += [f'- {prefix}Left: List<{element}>',f'- {prefix}Right: List<{element}>']
        body += [f'- the system assigns {json.dumps(left,ensure_ascii=index%2==0)} to data.{prefix}Left',f'- the system assigns {json.dumps(right,ensure_ascii=index%2!=0)} to data.{prefix}Right']
        operations=[('concat',f'concatenates list data.{prefix}Left and list data.{prefix}Right',left+right),('slice',f'slices list data.{prefix}Left from {start} to {end}',left[start:end]),('reverse',f'reverses list data.{prefix}Left',left[::-1]),('ascending',f'sorts list data.{prefix}Left in ascending order',ordered(left)),('descending',f'sorts list data.{prefix}Left in descending order',ordered(left,True))]
        for name,statement,value in operations:
            key=f'{prefix}{name}';fields.append(f'- {key}: List<{element}>');body += [f'- the system {statement} into data.{key}',f'- the system returns data.{key}'];expected['data.'+key.lower()]=(element,value)
    spec='The application ListOracle manages lists.\nA Data has:\n'+'\n'.join(fields)+'\nAction: Run.\nWhen Run happens:\n'+'\n'.join(body)+'\n- the system records a trace event named Done\n'
    (package/'spec.ail-spec.md').write_text(spec)
    for artifact,binary in routes(root,package,'public'):
        for backend in ['VM','native']:
            result=run((artifact,binary),backend);assert result.returncode==0,(backend,result.stderr[-2000:])
            fields=dict(line.split('=',1) for line in result.stdout.decode().splitlines() if line.startswith('data.'))
            assert fields.keys()==expected.keys(),(fields.keys(),expected.keys())
            for key,(element,value) in expected.items():match_list(fields[key],value,element,(backend,key))
    print('PASS: 180 independent typed source operations through source/saved Core/rendered source/direct STE × VM/native, including opaque Unicode/NUL strings',flush=True)


def canonical_float(value):
    # Reframe Python's independent shortest-round-trip spelling as scientific.
    # Report values are finite dyadic fractions, so no decimal rounding occurs.
    if value == 0:
        return '-0e0' if math.copysign(1.0, value) < 0 else '0e0'
    token = repr(abs(value)).lower()
    mantissa, _, exponent = token.partition('e')
    before, _, after = mantissa.partition('.')
    digits = before + after
    leading = len(digits) - len(digits.lstrip('0'))
    power = int(exponent or '0') + len(before) - leading - 1
    digits = digits.lstrip('0').rstrip('0')
    coefficient = digits[0] + ('.' + digits[1:] if len(digits) > 1 else '')
    return ('-' if value < 0 else '') + coefficient + 'e' + str(power)


def statistics_reports(root,rng):
    programs=routes(root,REPO/'examples/list-statistics','statistics','Analyze')
    input_path,report_path=root/'input file=json',root/'report file=json'
    for case in range(55):
        first=[rng.randrange(-5000,5000)/8 for _ in range(rng.randrange(25))]
        second=[rng.randrange(-5000,5000)/8 for _ in range(rng.randrange(25))]
        limit=rng.choice([-2**63,2**63-1,rng.randrange(-5,40)])
        if case==0:first,second,limit=[],[],0
        if case==1:first,second,limit=[-0.,0.,-0.,0.],[],4
        merged=sorted(first+second)
        length=len(merged)
        median=None if length==0 else merged[length//2] if length%2 else (merged[length//2-1]+merged[length//2])/2
        highest=merged[max(length-max(limit,0),0):][::-1]
        expected=dict(median=median,count=length,sorted=merged,highest=highest)
        input_path.write_text(json.dumps(dict(first=first,second=second,limit=limit)))
        for artifact,binary in programs:
            for backend in ['VM','native']:
                prefix=[AIL,'ail-vm',artifact,'--action','Analyze'] if backend=='VM' else [binary]
                result=invoke(prefix+[f'--read-file=input={input_path}',f'--write-file=report={report_path}'])
                assert result.returncode==0,(case,backend,result.stdout[:1000],result.stderr[-2000:])
                report=report_path.read_bytes();actual=json.loads(report)
                assert list(actual)==list(expected),(case,actual)
                assert actual['count']==length
                if median is None: assert actual['median'] is None
                else:assert bits(actual['median'])==bits(median),(case,backend,actual,expected)
                for name in ['sorted','highest']:assert [bits(float(value)) for value in actual[name]]==[bits(value) for value in expected[name]],(case,backend,name,actual,expected)
                # Enforce canonical JSON framing/keys; numeric spelling is independently bit-checked above.
                encoded_median='null' if median is None else canonical_float(median)
                encoded_sorted='['+','.join(canonical_float(value) for value in merged)+']'
                encoded_highest='['+','.join(canonical_float(value) for value in highest)+']'
                expected_bytes=('{"median":'+encoded_median+',"count":'+str(length)+',"sorted":'+encoded_sorted+',"highest":'+encoded_highest+'}').encode()
                assert report==expected_bytes,(case,backend,report[:1500],expected_bytes[:1500])
    bad=['{"first":[1e9999],"second":[],"limit":1}','{"first":[1.7976931348623157e308],"second":[1.7976931348623157e308],"limit":1}','{"first":[1,true],"second":[],"limit":1}','{"first":[],"second":[],"limit":1.0}']
    for raw in bad:
        input_path.write_text(raw)
        for artifact,binary in programs:
            for backend in ['VM','native']:
                report_path.write_bytes(b'previous report')
                prefix=[AIL,'ail-vm',artifact,'--action','Analyze'] if backend=='VM' else [binary]
                result=invoke(prefix+[f'--read-file=input={input_path}',f'--write-file=report={report_path}'])
                assert result.returncode!=0,(raw,backend,result.stdout[:1000])
                assert report_path.read_bytes()==b'previous report'
    print('PASS: 55 independently computed real JSON statistics reports on 12 routes, stable zero-sign median/top windows and four invalid/overflow datasets preserving prior files',flush=True)


def main():
    rng=random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix='ail-list-transform-oracle-') as directory:
        root=Path(directory)
        batch(root,rng)
        validation(root)
        limits(root)
        arenas(root)
        stress(root,rng)
        public_source(root,rng)
        statistics_reports(root,rng)
    print(f'PASS: independent typed list transformation oracle, seed={SEED}',flush=True)


if __name__=='__main__':main()
