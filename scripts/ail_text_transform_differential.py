#!/usr/bin/env python3
"""Independent, seeded Python oracle for AIL text transformations and TSV reports.

Run cargo build first. Expectations use Python's explicit-separator split, join,
replace and strip with the contract's fixed 29-character whitespace set. No AIL
implementation, benchmark answer, third-party package or platform Unicode table
is used to calculate expected values. subprocess.run drains both pipes; every
child has a timeout and failures have bounded diagnostic output.
"""
import json
from pathlib import Path
import random
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576
SEED = 0x7E875F04
WHITESPACE = "\u0009\u000a\u000b\u000c\u000d\u001c\u001d\u001e\u001f\u0020\u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000"
assert len(WHITESPACE) == 29
ALPHABET = ["", "a", "b", "é", "🙂", "\U0010ffff", "e\u0301", "\x00", "\x01", "\x7f", "\x85", "\u200b", "\ufeff", '"', "\\", "[", "]", ",", "=", "~", "/", "data.raw", "local.separator", "and"] + list(WHITESPACE)


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
    source.write_text(json.dumps({"kind": "AIL-Bytecode", "package": "text-transform-oracle", "version": "1", "profile": "Application", "capability_grants": [], "target_support": {}, "external_bindings": [], "failures": [], "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}] + list(routines)}))
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


def transform(opcode, key="data.result", **overrides):
    operands = {"key": key, "source": "input.items" if opcode == "TEXT_JOIN_LIST" else "input.source"}
    if opcode in ["TEXT_SPLIT", "TEXT_JOIN_LIST"]:
        operands["separator"] = "input.separator"
    if opcode == "TEXT_REPLACE":
        operands.update(needle="input.needle", replacement="input.replacement")
    operands.update(overrides)
    return op(opcode, **operands)


def batch(root, rng):
    steps = [op("LIST_GET", key=f"input.{name}", source="input.values", index=f"i:{index}", type="Text") for index, name in enumerate(["source", "separator", "needle", "replacement", "items"])]
    for name, opcode, result_type in [("split", "TEXT_SPLIT", "List<Text>"), ("join", "TEXT_JOIN_LIST", "Text"), ("replace", "TEXT_REPLACE", "Text"), ("trim", "TEXT_TRIM", "Text")]:
        steps += [transform(opcode), op("JSON_FORMAT", key=f"out.{name}", source="data.result", type=result_type), output(f"out.{name}")]
    aliases = [("split_source", "TEXT_SPLIT", "source"), ("split_separator", "TEXT_SPLIT", "separator"), ("join_source", "TEXT_JOIN_LIST", "source"), ("join_separator", "TEXT_JOIN_LIST", "separator"), ("replace_source", "TEXT_REPLACE", "source"), ("replace_needle", "TEXT_REPLACE", "needle"), ("replace_replacement", "TEXT_REPLACE", "replacement"), ("trim_source", "TEXT_TRIM", "source")]
    for name, opcode, operand in aliases:
        field = "input.items" if opcode == "TEXT_JOIN_LIST" and operand == "source" else f"input.{operand}"
        steps += [copy("data.alias", field), copy("data.snapshot", "data.alias"), transform(opcode, key="data.alias", **{operand: "data.alias"}), op("JSON_FORMAT", key=f"out.{name}", source="data.alias", type="List<Text>" if opcode == "TEXT_SPLIT" else "Text"), output(f"out.{name}"), op("JSON_FORMAT", key=f"out.{name}_snapshot", source="data.snapshot", type="Text"), output(f"out.{name}_snapshot")]
    program = compile_program(root, "batch", steps)
    fixed = [("", ":", "", "", []), ("aaaaa", "aa", "aa", "aaa", ["", "a", ""]), ("é🙂e\u0301", "🙂", "", "|", ["é", "\x00", "🙂"]), (WHITESPACE + "\x00" + WHITESPACE + "\u200b" + WHITESPACE, "\x00", "", "🙂", ["\x7f\x85", "\n\t"]), ('[value="and"],type=Text', "and", "value", "data.raw", [])]
    datasets = fixed + [("".join(rng.choices(ALPHABET, k=rng.randrange(35))), rng.choice(ALPHABET[1:]) or ":", "".join(rng.choices(ALPHABET, k=rng.randrange(4))), "".join(rng.choices(ALPHABET, k=rng.randrange(5))), ["".join(rng.choices(ALPHABET, k=rng.randrange(9))) for _ in range(rng.randrange(8))]) for _ in range(995)]
    for index, (source, separator, needle, replacement, items) in enumerate(datasets):
        # Alternate escaped surrogate pairs and UTF-8; nested raw list is borrowed.
        raw_items = json.dumps(items, ensure_ascii=index % 2 == 0, separators=(",", ":"))
        values = dict(source=source, separator=separator, needle=needle, replacement=replacement, items=raw_items)
        expected = dict(split=canonical(source.split(separator)), join=quoted(separator.join(items)), replace=quoted(source.replace(needle, replacement)), trim=quoted(source.strip(WHITESPACE)))
        for name, opcode, operand in aliases:
            expected[name] = expected[{"TEXT_SPLIT":"split", "TEXT_JOIN_LIST":"join", "TEXT_REPLACE":"replace", "TEXT_TRIM":"trim"}[opcode]]
            expected[name + "_snapshot"] = quoted(values["items" if opcode == "TEXT_JOIN_LIST" and operand == "source" else operand])
        wanted = ''.join(f"out.{key}={value}\n" for key, value in sorted(expected.items())).encode()
        arguments = ["input.values=" + json.dumps(list(values.values()), ensure_ascii=index % 2 == 0)]
        for backend in ["VM", "native"]:
            succeeded(run(program, backend, arguments), backend, wanted)
    print(f"PASS: {len(datasets)} seeded datasets × four transformations + eight aliases with immutable snapshots, Unicode/NUL, strict list escapes and all 29 whitespace scalars", flush=True)


def invalid_and_priority(root):
    programs = {opcode: compile_program(root, opcode.lower(), [transform(opcode)]) for opcode in ["TEXT_SPLIT", "TEXT_JOIN_LIST", "TEXT_REPLACE", "TEXT_TRIM"]}
    cases = [("TEXT_SPLIT", {}, "TEXT"), ("TEXT_SPLIT", {"source":"", "separator":""}, "TEXT"), ("TEXT_JOIN_LIST", {}, "LIST"), ("TEXT_JOIN_LIST", {"items":'[]'}, "TEXT"), ("TEXT_REPLACE", {"source":"", "needle":"absent"}, "TEXT"), ("TEXT_REPLACE", {"source":"", "replacement":"x"}, "TEXT"), ("TEXT_TRIM", {}, "TEXT")]
    bad_lists = ['["a",false]', '["a",]', '["a"] trailing', '["a",["nested"]]', '["\\ud800"]', '["\\udfff"]', '["\\u0000",1]', '["a" "b"]', '["a"\u00a0]', '["a"\x00]', 'PRIVATE_INVALID_LIST']
    cases += [("TEXT_JOIN_LIST", {"items":raw}, "LIST") for raw in bad_lists]
    for opcode, values, category in cases:
        for backend in ["VM", "native"]:
            arguments = [f"input.{key}={value}" for key, value in values.items()]
            if any("\x00" in value for value in values.values()):
                program = compile_program(root, "nul_list", [seed(f"input.{key}", value) for key, value in values.items()] + [transform(opcode)])
            else:
                program = programs[opcode]
            result = run(program, backend, arguments if program == programs[opcode] else [])
            failed(result, backend, category)
            assert b"PRIVATE_INVALID_LIST" not in result.stdout + result.stderr
    for raw in [b"\xff", b"\xc0\xaf", b"\xed\xa0\x80", b"\xf4\x90\x80\x80"]:
        for opcode, bad_field, category in [("TEXT_SPLIT", "source", "TEXT"), ("TEXT_SPLIT", "separator", "TEXT"), ("TEXT_JOIN_LIST", "separator", "TEXT"), ("TEXT_REPLACE", "source", "TEXT"), ("TEXT_REPLACE", "needle", "TEXT"), ("TEXT_REPLACE", "replacement", "TEXT"), ("TEXT_TRIM", "source", "TEXT")]:
            values = dict(source=b"", separator=b":", needle=b"absent", replacement=b"", items=b"[]")
            values[bad_field] = raw
            failed(invoke([str(programs[opcode][1]).encode()] + [b"input." + key.encode() + b"=" + value for key, value in values.items()]), "native", category)
        failed(invoke([str(programs["TEXT_JOIN_LIST"][1]).encode(), b'input.items=["'+raw+b'"]', b"input.separator="+raw]), "native", "LIST")
    print(f"PASS: {len(cases)} malformed/missing/empty-separator and error-priority cases plus 32 native invalid UTF-8 operand probes", flush=True)


def limits_and_matchers(root):
    cases = [
        ("split_exact", "TEXT_SPLIT", dict(source="x"*(LIMIT-4), separator=":"), 1, None),
        ("split_over", "TEXT_SPLIT", dict(source="x"*(LIMIT-3), separator=":"), None, "LIST-LIMIT"),
        ("split_count", "TEXT_SPLIT", dict(source=","*65535, separator=","), 65536, None),
        ("split_count_over", "TEXT_SPLIT", dict(source=","*65536, separator=","), None, "LIST-LIMIT"),
        ("split_expansion", "TEXT_SPLIT", dict(source="\x7f"*180000, separator=":"), None, "LIST-LIMIT"),
        ("split_source_over", "TEXT_SPLIT", dict(source=" "*(LIMIT+1), separator=""), None, "TEXT-LIMIT"),
        ("split_separator_over", "TEXT_SPLIT", dict(source="", separator="x"*(LIMIT+1)), None, "TEXT-LIMIT"),
        ("join_borrowed", "TEXT_JOIN_LIST", dict(items='["'+'\x7f'*180000+'"]', separator=""), 180000, None),
        ("join_exact", "TEXT_JOIN_LIST", dict(items='["",""]', separator="x"*LIMIT), LIMIT, None),
        ("join_over", "TEXT_JOIN_LIST", dict(items='["","",""]', separator="x"*LIMIT), None, "TEXT-LIMIT"),
        ("join_raw_exact", "TEXT_JOIN_LIST", dict(items='["'+'x'*(LIMIT-4)+'"]', separator=""), LIMIT-4, None),
        ("join_raw_over", "TEXT_JOIN_LIST", dict(items='["'+'x'*(LIMIT-3)+'"]'), None, "LIST-LIMIT"),
        ("join_count", "TEXT_JOIN_LIST", dict(items='['+','.join(['""']*65536)+']', separator=""), 0, None),
        ("join_count_over", "TEXT_JOIN_LIST", dict(items='['+','.join(['""']*65537)+']'), None, "LIST-LIMIT"),
        ("join_separator_over", "TEXT_JOIN_LIST", dict(items='[]', separator="x"*(LIMIT+1)), None, "TEXT-LIMIT"),
        ("replace_exact", "TEXT_REPLACE", dict(source="a", needle="a", replacement="x"*LIMIT), LIMIT, None),
        ("replace_over", "TEXT_REPLACE", dict(source="aa", needle="a", replacement="x"*LIMIT), None, "TEXT-LIMIT"),
        ("replace_source_over", "TEXT_REPLACE", dict(source="x"*(LIMIT+1)), None, "TEXT-LIMIT"),
        ("replace_needle_over", "TEXT_REPLACE", dict(source="", needle="x"*(LIMIT+1)), None, "TEXT-LIMIT"),
        ("replace_replacement_over", "TEXT_REPLACE", dict(source="", needle="absent", replacement="x"*(LIMIT+1)), None, "TEXT-LIMIT"),
        ("replace_empty_needle", "TEXT_REPLACE", dict(source="🙂"*(LIMIT//4), needle="", replacement="x"), None, "TEXT-LIMIT"),
        ("trim_exact", "TEXT_TRIM", dict(source="x"*LIMIT), LIMIT, None),
        ("trim_over", "TEXT_TRIM", dict(source=" "*(LIMIT+1)), None, "TEXT-LIMIT"),
    ]
    # Repetitive prefixes force quadratic naive searching to exceed the child timeout.
    for suffix in ["b", "c"]:
        haystack = "a"*(LIMIT-1)+"b"
        needle = "a"*65535+suffix
        cases += [(f"matcher_split_{suffix}", "TEXT_SPLIT", dict(source=haystack,separator=needle), len(haystack.split(needle)) if suffix=="b" else None, None if suffix=="b" else "LIST-LIMIT"), (f"matcher_replace_{suffix}", "TEXT_REPLACE", dict(source=haystack,needle=needle,replacement="x"), len(haystack.replace(needle,"x")), None)]
    for name, opcode, values, expected, category in cases:
        steps = [seed(f"input.{key}", value) for key, value in values.items()] + [transform(opcode)]
        steps += [op("LIST_LENGTH", source="data.result", key="out.value", type="Text") if opcode == "TEXT_SPLIT" else op("TEXT_LENGTH", source="data.result", key="out.value"), output("out.value")]
        program = compile_program(root, name, steps)
        for backend in ["VM", "native"]:
            result = run(program, backend)
            if category:
                failed(result, backend, category)
            else:
                succeeded(result, backend, f"out.value={expected}\n".encode())
    print(f"PASS: {len(cases)} exact/raw/canonical/item/Text limits, borrowed canonical-overflow joins and adversarial 1 MiB repetitive-needle searches", flush=True)


def prefill(arena, reserved):
    steps = [seed("seed.empty", ""), seed("seed.large", "x"*LIMIT if arena == "text" else '"'+'x'*(LIMIT-2)+'"'), seed("seed.partial", "x"*(LIMIT-reserved) if arena == "text" else '"'+'x'*(LIMIT-reserved-2)+'"')]
    make = lambda field: op("TEXT_JOIN", left=field, right="seed.empty", key="data.fill") if arena == "text" else op("JSON_PARSE", source=field, type="Json", key="data.fill")
    steps += [make("seed.large") for _ in range(63)] + [make("seed.partial")]
    return steps


def arenas_and_returns(root):
    for opcode, arena, values, size in [
        ("TEXT_SPLIT", "list", dict(source="", separator=":"), 4),
        ("TEXT_JOIN_LIST", "text", dict(items="[]", separator=""), 1),
        ("TEXT_REPLACE", "text", dict(source="", needle="absent", replacement=""), 1),
        ("TEXT_TRIM", "text", dict(source=""), 1),
    ]:
        type_name = "List<Text>" if opcode == "TEXT_SPLIT" else "Text"
        local_values = {key: f"local.{key}" for key in values}
        parameters = ';'.join(f"{key}:{'List<Text>' if key == 'items' else 'Text'}" for key in sorted(values))
        arguments = ';'.join(f"{key}=f:input.{key}" for key in sorted(values))
        overrides = {("source" if key == "items" else key): value for key,value in local_values.items()}
        routine = {"action":"transform", "instructions":[op("VALUE_FUNCTION", parameters=parameters, locals=f"result:{type_name}", result=type_name), transform(opcode,key="local.result",**overrides),op("RETURN_VALUE",type=type_name,expression="f:local.result")]}
        base = prefill(arena, size*2) + [seed(f"input.{key}", value) for key,value in values.items()] + [op("CALL_VALUE", target="transform", arguments=arguments, key="data.result")]
        for extra in [False, True]:
            steps = base + ([transform(opcode)] if extra else []) + [op("LIST_LENGTH", type="Text", key="out.value",source="data.result") if opcode == "TEXT_SPLIT" else op("TEXT_LENGTH", key="out.value",source="data.result"), output("out.value")]
            program = compile_program(root, f"arena_{opcode}_{extra}", steps, [routine])
            for backend in ["VM","native"]:
                result = run(program,backend)
                if extra:
                    failed(result,backend,"LIST-LIMIT" if arena=="list" else "TEXT-LIMIT")
                else:
                    succeeded(result,backend,b"out.value=1\n" if opcode=="TEXT_SPLIT" else b"out.value=0\n")
    # Opposite arenas must remain free; scratch and decoded parts consume neither.
    for arena, opcode, values, expected in [("text","TEXT_SPLIT",dict(source="\x00é",separator=":"),'["\\u0000é"]'),("list","TEXT_JOIN_LIST",dict(items='["é","🙂"]',separator=""),"é🙂")]:
        steps = prefill(arena,0) + [seed(f"input.{key}",value) for key,value in values.items()] + [transform(opcode,key="out.value"),output("out.value")]
        program = compile_program(root,f"opposite_{arena}",steps)
        for backend in ["VM","native"]:
            succeeded(run(program,backend),backend,f"out.value={expected}\n".encode())
    # A parameter is a borrowed validated list; only the joined Text return is copied.
    routine={"action":"borrow", "instructions":[op("VALUE_FUNCTION",parameters="items:List<Text>;separator:Text",locals="result:Text",result="Text"),op("TEXT_JOIN_LIST",key="local.result",source="local.items",separator="local.separator"),op("RETURN_VALUE",type="Text",expression="f:local.result")]}
    program=compile_program(root,"borrowed_parameter",[seed("input.items",'["'+'\x7f'*180000+'"]'),seed("input.separator",""),op("CALL_VALUE",target="borrow",arguments="items=f:input.items;separator=f:input.separator",key="data.result"),op("TEXT_LENGTH",key="out.value",source="data.result"),output("out.value")],[routine])
    for backend in ["VM","native"]:
        succeeded(run(program,backend),backend,b"out.value=180000\n")
    print("PASS: four exact shared arena boundaries with one operation + one function-return copy, zero/unchanged outputs, both opposite-arena checks and borrowed expanding List<Text> parameter",flush=True)


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


def public_source(root,rng):
    package=root/"source";package.mkdir()
    (package/"ail-package.md").write_text("# AIL Package\nname: text-transform-oracle\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, traces\nconformance: first-slice\n")
    datasets=[("[value=Int],name=local.raw\x00🙂",",","",'" \\ '), (WHITESPACE+"\u200b"+WHITESPACE,"\x85","\u200b","a")]+[("".join(rng.choices(ALPHABET,k=rng.randrange(12))),rng.choice(ALPHABET[1:]),"".join(rng.choices(ALPHABET,k=rng.randrange(3))),"".join(rng.choices(ALPHABET,k=rng.randrange(4)))) for _ in range(58)]
    declarations="The application Transforms manages records.\nA Data has:\n- raw: Text\n- separator: Text\n- needle: Text\n- replacement: Text\n- parts: List<Text>\n- result: Text\nAn Output has:\n"+''.join(f"- v{n:03}: Text\n" for n in range(len(datasets)*4))
    lines=["Action: Run.","When Run happens:"];expected={}
    for index,(raw,separator,needle,replacement) in enumerate(datasets):
        for field,value in [("raw",raw),("separator",separator),("needle",needle),("replacement",replacement)]:
            lines.append(f"- the system assigns {json.dumps(value,ensure_ascii=index%2==0)} to data.{field}")
        parts=raw.split(separator)
        for offset,(statement,value,type_name,field) in enumerate([(f"splits text data.raw by data.separator into data.parts",parts,"List<Text>","data.parts"),(f"joins list data.parts with data.separator into data.result",separator.join(parts),"Text","data.result"),(f"replaces text data.needle in data.raw with data.replacement into data.result",raw.replace(needle,replacement),"Text","data.result"),(f"trims text data.raw into data.result",raw.strip(WHITESPACE),"Text","data.result")]):
            key=f"output.v{index*4+offset:03}"
            lines += [f"- the system {statement}",f"- the system formats {field} as JSON text into {key}",f"- the system returns {key}"]
            expected[key]=canonical(value)
    lines.append("- the system records a trace event named Done")
    (package/"spec.ail-spec.md").write_text(declarations+'\n'.join(lines)+'\n')
    wanted=''.join(f"{key}={value}\n" for key,value in sorted(expected.items())).encode()
    for program in routes(root,package,"public"):
        for backend in ["VM","native"]:
            succeeded(run(program,backend),backend,wanted)
    print("PASS: 240 independent source transformations through source/saved Core/rendered source/direct STE × VM/native, including opaque attributes, NUL and surrogate pairs",flush=True)


def tabular_reports(root,rng):
    programs=routes(root,REPO/"examples/tabular-report","tabular","Summarize")
    input_path,report_path=root/"input file=tsv",root/"report file=json"
    for case in range(54):
        rows=[]
        for _ in range(rng.randrange(25)):
            kind=rng.randrange(7)
            name=''.join(rng.choices(["é","🙂","\x00","and","=","\x7f","e\u0301","\u200b","\ufeff"],k=rng.randrange(1,6)))
            if kind==0: row=[name,"1"]
            elif kind==1: row=[name,"", "10"]
            elif kind==2: row=["", "2", "10"]
            elif kind==3: row=["","",""]
            else: row=[name,str(rng.randrange(-2,10)),str(rng.randrange(-2,300))]
            rows.append('\t'.join(rng.choice([""," ","\u0085","\u2003","\u001c"])+value+rng.choice([""," ","\u3000"]) for value in row))
        separator='\r\n' if case%2 else '\n'
        raw=separator.join(["item\tunits\tunit_cents"]+rows)+separator
        if case==0: raw="item\tunits\tunit_cents\r\n Apple \t2\t125\r\n\u2003Unicode λ\u2003\t3\t200\r\nzero units\t0\t350\r\nbad\tcolumns\r\n\r\n"
        names=[];rejected=0;units=0;total=0
        for line in raw.replace('\r\n','\n').split('\n')[1:]:
            if not line.strip(WHITESPACE): continue
            columns=[part.strip(WHITESPACE) for part in line.split('\t')]
            if len(columns)!=3 or any(not value for value in columns): rejected+=1;continue
            name,raw_units,raw_cents=columns
            count,cents=int(raw_units),int(raw_cents)
            if count<=0 or cents<0: rejected+=1;continue
            names.append(name);units+=count;total+=count*cents
        expected=canonical(dict(names=names,summary=' | '.join(names),accepted=len(names),rejected=rejected,units=units,total_cents=total)).encode()
        input_path.write_bytes(raw.encode())
        for artifact,binary in programs:
            for backend in ["VM","native"]:
                prefix=[AIL,"ail-vm",artifact,"--action","Summarize"] if backend=="VM" else [binary]
                result=invoke(prefix+[f"--read-file=input={input_path}",f"--write-file=report={report_path}"])
                assert result.returncode==0,(case,backend,result.stdout[:1000],result.stderr[-2000:])
                assert report_path.read_bytes()==expected,(case,backend,report_path.read_bytes()[:1500],expected[:1500])
    for token in ["01","+2","1.0","1e2","null","false","oops"]:
        input_path.write_text("item\tunits\tunit_cents\nfirst\t2\t10\nlast\t"+token+"\t3\n")
        for artifact,binary in programs:
            for backend in ["VM","native"]:
                report_path.write_bytes(b"previous report")
                prefix=[AIL,"ail-vm",artifact,"--action","Summarize"] if backend=="VM" else [binary]
                result=invoke(prefix+[f"--read-file=input={input_path}",f"--write-file=report={report_path}"])
                failed(result,backend,"JSON")
                assert report_path.read_bytes()==b"previous report"
    print("PASS: 54 independently calculated Unicode/NUL TSV reports on 12 public routes, plus seven malformed-number datasets preserving the previous report",flush=True)


def main():
    rng=random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix="ail-text-transform-oracle-") as directory:
        root=Path(directory)
        batch(root,rng)
        invalid_and_priority(root)
        limits_and_matchers(root)
        arenas_and_returns(root)
        public_source(root,rng)
        tabular_reports(root,rng)
    print(f"PASS: independent text transformation oracle, seed={SEED}",flush=True)


if __name__ == "__main__":
    main()
