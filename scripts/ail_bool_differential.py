#!/usr/bin/env python3
"""Independent Boolean predicate oracle across public AIL pipelines.

Uses Python values, ordering and lazy logical evaluation; never imports AIL
parsers or evaluators. Build target/debug/ail first. Fixed seed, bounded child
processes, temporary artifacts, and capped diagnostics make failures repeatable.
"""
import json
import math
import operator
from pathlib import Path
import random
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576
SEED = 0xB001C0DE


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=30)


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions, routines=()):
    source = root / f"{name}.json"
    source.write_text(json.dumps({
        "kind": "AIL-Bytecode", "package": "boolean-oracle", "version": "1",
        "profile": "Application", "capability_grants": [], "target_support": {},
        "external_bindings": [], "failures": [],
        "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}] + list(routines),
    }))
    binary = root / f"{name}.elf"
    result = invoke([AIL, "ail-compile", source, "--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
    assert result.returncode == 0, (name, result.stdout[:1000], result.stderr[-2000:])
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


def failed(result, backend, category="JSON"):
    marker = f"AIL-RUNTIME-{category}:".encode()
    assert result.returncode != 0, (backend, result.stdout[:1000], result.stderr[-1000:])
    assert marker in result.stdout + result.stderr, (backend, category, result.stdout[:1000], result.stderr[-2000:])
    if backend == "native":
        assert not result.stdout, result.stdout[:1000]


COMPARISONS = {"==": operator.eq, "!=": operator.ne, "<": operator.lt, "<=": operator.le, ">": operator.gt, ">=": operator.ge}
ALPHABET = ["a", "é", "🙂", "\U0010ffff", "e\u0301", "\x00", "\x01", "\n", "\r", "\t", "\x7f", "\x85", '"', "\\", "=", "[value=1],type=Text", "/", "~", "and or not (integer data.secret)"]


def boolean(value):
    return "true" if value else "false"


def quoted(value):
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + "".join(escapes[c] if c in escapes else f"\\u{ord(c):04x}" if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value) + '"'


def canonical(value):
    if value is None:
        return "null"
    if type(value) is bool:
        return boolean(value)
    if type(value) is int:
        return str(value)
    if type(value) is str:
        return quoted(value)
    if type(value) is list:
        return "[" + ",".join(map(canonical, value)) + "]"
    return "{" + ",".join(quoted(key) + ":" + canonical(child) for key, child in value.items()) + "}"


def checked(arguments):
    result = invoke(arguments)
    assert result.returncode == 0, (list(map(str,arguments)), result.stdout[:1000], result.stderr[-2000:])
    return result.stdout


def field(name, kind):
    return ("field", kind, name)


def literal(value, kind):
    return ("literal", kind, value)


def evaluate(node, values):
    tag = node[0]
    if tag == "literal":
        return node[2]
    if tag == "field":
        return values[node[2]]
    if tag == "not":
        return not evaluate(node[1], values)
    if tag == "and":
        return evaluate(node[1], values) and evaluate(node[2], values)
    if tag == "or":
        return evaluate(node[1], values) or evaluate(node[2], values)
    if tag == "compare":
        return COMPARISONS[node[2]](evaluate(node[3], values), evaluate(node[4], values))
    if tag == "add":
        return evaluate(node[1], values) + evaluate(node[2], values)
    raise AssertionError(node)


def postfix(node):
    tag = node[0]
    if tag == "literal":
        kind, value = node[1:]
        return {"Bool": lambda: "b:" + boolean(value), "Int": lambda: "i:" + str(value), "Float": lambda: "d:" + repr(value), "Text": lambda: "s:" + json.dumps(value, ensure_ascii=False)}[kind]()
    if tag == "field":
        return {"Bool": "bf:", "Int": "if:", "Float": "ff:", "Text": "tf:"}[node[1]] + node[2]
    if tag == "not":
        return postfix(node[1]) + " not"
    if tag in ["and", "or"]:
        return postfix(node[1]) + " " + postfix(node[2]) + " " + tag
    if tag == "add":
        return postfix(node[1]) + " " + postfix(node[2]) + " i+"
    if tag == "compare":
        return postfix(node[3]) + " " + postfix(node[4]) + " " + {"Bool": "bcmp:", "Int": "icmp:", "Float": "fcmp:", "Text": "tcmp:"}[node[1]] + node[2]
    raise AssertionError(node)


def source_expression(node):
    tag = node[0]
    if tag == "literal":
        return boolean(node[2]) if node[1] == "Bool" else json.dumps(node[2], ensure_ascii=False) if node[1] == "Text" else repr(node[2])
    if tag == "field":
        return node[2]
    if tag == "not":
        return "not (" + source_expression(node[1]) + ")"
    if tag in ["and", "or"]:
        return "(" + source_expression(node[1]) + ") " + tag + " (" + source_expression(node[2]) + ")"
    if tag == "add":
        return "(" + source_expression(node[1]) + " + " + source_expression(node[2]) + ")"
    if tag == "compare":
        left, right = source_expression(node[3]), source_expression(node[4])
        if node[1] == "Bool":
            left, right = "("+left+")", "("+right+")"
        return {"Bool": "boolean", "Int": "integer", "Float": "floating", "Text": "text"}[node[1]] + " " + left + " " + node[2] + " " + right
    raise AssertionError(node)


def random_predicate(rng, depth=0, fields=True):
    kind = rng.randrange(7 if depth < 3 else 4)
    if kind == 0:
        return field(rng.choice(["input.ba", "input.bb"]), "Bool") if fields and rng.randrange(2) else literal(rng.choice([True, False]), "Bool")
    if kind == 1:
        left = ("add", field("input.small", "Int"), literal(rng.randrange(-10,10), "Int")) if fields else literal(rng.randrange(-100,100), "Int")
        return ("compare", "Int", rng.choice(list(COMPARISONS)), left, literal(rng.randrange(-100,100), "Int"))
    if kind == 2:
        left = field(rng.choice(["input.fa", "input.fb"]), "Float") if fields else literal(rng.choice([-0., 0., math.nan, math.inf, -math.inf, .5]), "Float")
        return ("compare", "Float", rng.choice(list(COMPARISONS)), left, literal(rng.choice([-0., 0., math.nan, math.inf, -math.inf, .5]), "Float"))
    if kind == 3:
        left = field(rng.choice(["input.ta", "input.tb"]), "Text") if fields else literal("".join(rng.choices(ALPHABET,k=rng.randrange(5))), "Text")
        return ("compare", "Text", rng.choice(list(COMPARISONS)), left, literal("".join(rng.choices(ALPHABET,k=rng.randrange(5))), "Text"))
    if kind == 4:
        return ("not", random_predicate(rng, depth+1, fields))
    if kind == 5:
        return (rng.choice(["and", "or"]), random_predicate(rng, depth+1, fields), random_predicate(rng, depth+1, fields))
    return ("compare", "Bool", rng.choice(list(COMPARISONS)), random_predicate(rng, depth+1, fields), random_predicate(rng, depth+1, fields))


def batched_predicates(root, rng):
    trees = [("compare", kind, comparison, field(f"input.{stem}a", kind), field(f"input.{stem}b", kind)) for kind, stem in [("Int", "i"), ("Float", "f"), ("Text", "t"), ("Bool", "b")] for comparison in COMPARISONS]
    trees += [random_predicate(rng) for _ in range(60)]
    steps = [op("LIST_GET", type="Text", source="input.texts", index="i:0", key="input.ta"), op("LIST_GET", type="Text", source="input.texts", index="i:1", key="input.tb")]
    for n, tree in enumerate(trees):
        steps += [op("EVAL_BOOL", expression=postfix(tree), key=f"out.v{n:03}"), op("OUTPUT_FIELD", key=f"out.v{n:03}")]
    program = compile_program(root, "batch", steps)
    floats = [math.nan, math.inf, -math.inf, -0., 0., 5e-324, -5e-324, .5, -.5, 1.7976931348623157e308]
    for case in range(350):
        values = {"input.ia": rng.choice([-2**63, 2**63-1, rng.randrange(-10**12,10**12)]), "input.ib": rng.choice([-2**63, 2**63-1, rng.randrange(-10**12,10**12)]), "input.fa": rng.choice(floats), "input.fb": rng.choice(floats), "input.ba": bool(rng.randrange(2)), "input.bb": bool(rng.randrange(2)), "input.small": rng.randrange(-100,100), "input.ta": "".join(rng.choices(ALPHABET,k=rng.randrange(8))), "input.tb": "".join(rng.choices(ALPHABET,k=rng.randrange(8)))}
        arguments = [f"{key}={boolean(value) if type(value) is bool else repr(value)}" for key, value in values.items() if key not in ["input.ta", "input.tb"]]
        arguments.append("input.texts=" + json.dumps([values["input.ta"],values["input.tb"]]))
        expected = "".join(f"out.v{n:03}={boolean(evaluate(tree, values))}\n" for n, tree in enumerate(trees)).encode()
        for backend in ["VM", "native"]:
            succeeded(run(program, backend, arguments), backend, expected)
    print(f"PASS: 350 seeded datasets × {len(trees)} independently evaluated typed/lazy predicates, including all six scalar comparisons, Unicode/NUL, NaN and signed zero", flush=True)


def lazy_errors(root):
    cases = [("b:false bf:input.bad and", [], False), ("b:true bf:input.bad or", ["input.bad=PRIVATE_INVALID"], True), ("b:true tf:input.bad s:\"x\" tcmp:== or", [], True), ("b:false if:input.bad i:0 i/ i:1 icmp:== and", [], False), ("b:true ff:input.bad d:0 f/ d:0 fcmp:== or", [], True), ("bf:input.bad", [], "BOOL"), ("bf:input.bad", ["input.bad=True"], "BOOL"), ("i:1 i:0 i/ if:input.bad icmp:==", [], "DIVZERO"), ("if:input.bad i:1 i:0 i/ icmp:==", [], "INT"), ("d:1 d:0 f/ ff:input.bad fcmp:==", [], "DIVZERO"), ("ff:input.bad d:1 d:0 f/ fcmp:==", [], "FLOAT"), ("bf:input.bad tf:input.text s:\"\" tcmp:== bcmp:==", [], "BOOL")]
    for n, (expression, arguments, expected) in enumerate(cases):
        program = compile_program(root, f"lazy{n}", [op("EVAL_BOOL", expression=expression, key="out.value"), op("OUTPUT_FIELD", key="out.value")])
        for backend in ["VM", "native"]:
            result = run(program, backend, arguments)
            if type(expected) is bool:
                succeeded(result, backend, f"out.value={boolean(expected)}\n".encode())
            else:
                failed(result, backend, expected)
                assert b"PRIVATE_INVALID" not in result.stdout + result.stderr
    for skip in [True, False]:
        expression = 'tf:input.bad s:"x" tcmp:=='
        if skip:
            expression = "b:true " + expression + " or"
        program = compile_program(root, f"utf8{skip}", [op("EVAL_BOOL", expression=expression, key="out.value"), op("OUTPUT_FIELD", key="out.value")])
        for raw in [b"\xff",b"\xc0\xaf",b"\xed\xa0\x80",b"\xf4\x90\x80\x80"]:
            result = invoke([str(program[1]).encode(), b"input.bad="+raw])
            if skip:
                succeeded(result, "native", b"out.value=true\n")
            else:
                failed(result, "native", "TEXT")
    for length, skip in [(LIMIT,False),(LIMIT+1,False),(LIMIT+1,True)]:
        expression = 'tf:input.large s:"" tcmp:>'
        if skip:
            expression = "b:true " + expression + " or"
        program = compile_program(root, f"textlimit{length}_{skip}", [op("SET_FIELD", key="input.large", value="x"*length, text="seed"), op("EVAL_BOOL", expression=expression, key="out.value"), op("OUTPUT_FIELD", key="out.value")])
        for backend in ["VM", "native"]:
            result = run(program, backend)
            if length > LIMIT and not skip:
                failed(result, backend, "TEXT-LIMIT")
            else:
                succeeded(result, backend, b"out.value=true\n")
    print("PASS: lazy missing/invalid fields, left-to-right error families, eight reached/skipped invalid UTF-8 probes and Text exact/overflow bounds", flush=True)


def no_arena_charge(root):
    steps = [op("SET_FIELD", key="input.text", value="x"*LIMIT, text="seed"), op("SET_FIELD", key="input.empty", value="", text="seed"), op("SET_FIELD", key="input.json", value='"'+'x'*(LIMIT-2)+'"', text="seed")]
    for _ in range(64):
        steps += [op("TEXT_JOIN", key="data.text", left="input.text", right="input.empty"), op("JSON_PARSE", key="data.json", source="input.json", type="Json")]
    steps += [op("EVAL_BOOL", expression='tf:input.text s:"" tcmp:> b:true and', key="out.value") for _ in range(128)]
    steps += [op("CALL_VALUE", target="predicate", arguments="value=f:out.value", key="out.value") for _ in range(128)]
    steps += [op("OUTPUT_FIELD", key="out.value")]
    routine = {"action":"predicate", "instructions":[op("VALUE_FUNCTION", parameters="value:Bool", locals="result:Bool", result="Bool"), op("EVAL_BOOL", key="local.result", expression="bf:local.value b:false or"), op("RETURN_VALUE", type="Bool", expression="f:local.result")]}
    program = compile_program(root, "arenas", steps, [routine])
    for backend in ["VM","native"]:
        succeeded(run(program, backend), backend, b"out.value=true\n")
    print("PASS: 128 predicates plus 128 Bool function returns after both 64 MiB arenas are exactly exhausted", flush=True)


def public_routes(root, rng):
    trees = [random_predicate(rng, fields=False) for _ in range(90)]
    # Force hostile quoted Core syntax and grouped Boolean comparisons in every run.
    trees += [("compare", "Text", "==", literal(text,"Text"),literal(text,"Text")) for text in ["[value=1],type=Text", "and or not integer floating () local.secret\x00é🙂", '"quoted" \\ escaped', "line\nnext\r\t\x85"]]
    package = root / "source"
    package.mkdir()
    metadata = "# AIL Package\nname: bool-oracle\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, traces\nconformance: first-slice\n"
    (package / "ail-package.md").write_text(metadata)
    spec = "The application BoolOracle manages values.\nAn Output has:\n"+"\n".join(f"- v{n:03}: Bool" for n in range(len(trees)))+"\nAction: Run.\nWhen Run happens:\n"+"\n".join(f"- the system computes boolean output.v{n:03} as {source_expression(tree)}\n- the system returns output.v{n:03}" for n,tree in enumerate(trees))+"\n- the system records a trace event named Done\n"
    (package / "spec.ail-spec.md").write_text(spec)
    expected = "".join(f"output.v{n:03}={boolean(evaluate(tree,{}))}\n" for n,tree in enumerate(trees)).encode()
    core = root / "saved.core"
    core.write_bytes(checked([AIL,"ail-core",package]))
    rendered = root / "rendered"
    rendered.mkdir()
    (rendered / "ail-package.md").write_text(metadata)
    (rendered / "spec.ail-spec.md").write_bytes(checked([AIL,"ail-spec","--core-file",core]))
    for name,arguments in [("source",[package]),("core",["--core-file",core]),("rendered",[rendered])]:
        bytecode = root / f"route_{name}.json"
        bytecode.write_bytes(checked([AIL,"ail-lower"]+arguments))
        ste = root / f"route_{name}.ail-ste"
        ste.write_bytes(checked([AIL,"ail-ste"]+arguments))
        for label,artifact,compile_arguments in [("source",bytecode,arguments),("ste",ste,[ste])]:
            binary = root / f"route_{name}_{label}.elf"
            checked([AIL,"ail-compile"]+compile_arguments+["--action","Run","--target","linux-x86_64-elf","--out",binary])
            succeeded(run((artifact,binary),"VM"),"VM",expected)
            succeeded(run((artifact,binary),"native"),"native",expected)
    print(f"PASS: {len(trees)} independent source predicates across source/saved Core/rendered source/STE × VM/native, including opaque quote/NUL/Core syntax", flush=True)


def file_filters(root, rng):
    package = REPO / "examples/boolean-filter"
    core = root / "filter.core"
    core.write_bytes(checked([AIL,"ail-core",package]))
    rendered = root / "filter-rendered"
    rendered.mkdir()
    (rendered / "ail-package.md").write_bytes((package / "ail-package.md").read_bytes())
    (rendered / "spec.ail-spec.md").write_bytes(checked([AIL,"ail-spec","--core-file",core]))
    routes = []
    for name,arguments in [("source",[package]),("core",["--core-file",core]),("rendered",[rendered])]:
        bytecode = root / f"filter_{name}.json"
        bytecode.write_bytes(checked([AIL,"ail-lower"]+arguments))
        ste = root / f"filter_{name}.ail-ste"
        ste.write_bytes(checked([AIL,"ail-ste"]+arguments))
        for label,artifact,compile_arguments in [("source",bytecode,arguments),("ste",ste,[ste])]:
            binary = root / f"filter_{name}_{label}.elf"
            checked([AIL,"ail-compile"]+compile_arguments+["--action","Filter","--target","linux-x86_64-elf","--out",binary])
            routes.append((artifact,binary))
    input_path, report_path = root/"input file=json", root/"report file=json"
    for n in range(48):
        records = [{"name":"".join(rng.choices(ALPHABET,k=rng.randrange(8))),"enabled":bool(rng.randrange(2)),"kind":rng.choice(["task","issue","other","TASK","task\x00","and or not"]),"count":rng.randrange(-3,6),"score":rng.choice([-.0,.7499999999999999,.75,.7500000000000001,1.,rng.random()])} for _ in range(rng.randrange(18))]
        selected = [row["name"] for row in records if row["enabled"] and row["kind"] in ["task","issue"] and row["count"]>0 and row["score"]>=.75]
        expected = canonical({"names":selected,"accepted":len(selected),"rejected":len(records)-len(selected)}).encode()
        input_path.write_text(json.dumps(records,ensure_ascii=n%2==0))
        for artifact,binary in routes:
            for backend in ["VM","native"]:
                prefix = [AIL,"ail-vm",artifact,"--action","Filter"] if backend=="VM" else [binary]
                result = invoke(prefix+[f"--read-file=input={input_path}",f"--write-file=report={report_path}"])
                assert result.returncode==0,(n,backend,result.stdout[:1000],result.stderr[-2000:])
                assert report_path.read_bytes()==expected,(n,backend,report_path.read_bytes()[:1500],expected[:1500])
    print("PASS: 48 independent JSON record filtering datasets through source/Core/rendered source/STE × VM/native, reusable Bool function, Unicode/NUL names and exact reports",flush=True)


def main():
    rng = random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix="ail-bool-oracle-") as directory:
        root = Path(directory)
        batched_predicates(root,rng)
        lazy_errors(root)
        no_arena_charge(root)
        public_routes(root,rng)
        file_filters(root,rng)
    print(f"PASS: independent Boolean predicate oracle, seed={SEED}",flush=True)


if __name__ == "__main__":
    main()
