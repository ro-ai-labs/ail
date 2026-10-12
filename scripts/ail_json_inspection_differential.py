#!/usr/bin/env python3
"""Independent JSON kind/key inspection oracle for AIL VM and native ELF.

Run cargo build first. Uses Python JSON's number-token and object-pair hooks,
then separately implements AIL canonical strings and typed pointer selection.
No production parser, emitter, third-party dependency or benchmark is used.
"""
import json
from pathlib import Path
import random
import re
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576


class Number(str):
    """Exact spelling of a JSON number, never converted to float."""


class Pairs(list):
    """An object preserving member order and repeated keys."""


class OracleError(ValueError):
    pass


def quoted(value):
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + "".join(escapes[c] if c in escapes else f"\\u{ord(c):04x}" if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value) + '"'


def canonical(value):
    if isinstance(value, Number):
        return str(value)
    if value is None:
        return "null"
    if type(value) is bool:
        return "true" if value else "false"
    if type(value) is str:
        return quoted(value)
    if isinstance(value, Pairs):
        return "{" + ",".join(quoted(key) + ":" + canonical(item) for key, item in value) + "}"
    return "[" + ",".join(canonical(item) for item in value) + "]"


def reject_constant(value):
    raise ValueError(value)


def checked_string(value):
    if any(0xD800 <= ord(c) <= 0xDFFF for c in value):
        raise OracleError("JSON")


def decode(raw):
    if len(raw.encode()) > LIMIT:
        raise OracleError("JSON-LIMIT")
    try:
        value = json.loads(raw, parse_int=Number, parse_float=Number,
                           parse_constant=reject_constant, object_pairs_hook=Pairs)
    except (ValueError, RecursionError):
        raise OracleError("JSON") from None
    nodes = 0

    def validate(item, depth):
        nonlocal nodes
        nodes += 1
        if nodes > 65_536:
            raise OracleError("JSON-LIMIT")
        if type(item) is str:
            checked_string(item)
        elif isinstance(item, list):
            depth += 1
            if depth > 64:
                raise OracleError("JSON-LIMIT")
            for entry in item:
                if isinstance(item, Pairs):
                    key, child = entry
                    checked_string(key)
                else:
                    child = entry
                validate(child, depth)

    validate(value, 0)
    if len(canonical(value).encode()) > LIMIT:
        raise OracleError("JSON-LIMIT")
    return value


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=30)


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions, routines=()):
    source = root / f"{name}.json"
    source.write_text(json.dumps({
        "kind": "AIL-Bytecode", "package": "value-function-oracle", "version": "1",
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


def emit(value, ascii_only=False):
    """Alternate legal spelling independent of AIL canonical escaping."""
    if isinstance(value, Number):
        return str(value)
    if isinstance(value, Pairs):
        return "{ " + ", ".join(json.dumps(key, ensure_ascii=ascii_only) + " : " + emit(item, ascii_only) for key, item in value) + " }"
    if type(value) is list:
        return "[ " + " , ".join(emit(item, ascii_only) for item in value) + " ]"
    return json.dumps(value, ensure_ascii=ascii_only)


def routine(name, parameters, locals_, result, body):
    return {"action": name, "instructions": [op("VALUE_FUNCTION", parameters=parameters, locals=locals_, result=result)] + body}


def call(target, arguments, key):
    return op("CALL_VALUE", target=target, arguments=arguments, key=key)


def returns(type_name, expression):
    return op("RETURN_VALUE", type=type_name, expression=expression)


def seed(key, value):
    return op("SET_FIELD", key=key, value=value, text="function boundary fixture")


SEED = 0x1A5EEC7
ALPHABET = ["a", "", "é", "🙂", "\U0010ffff", "e\u0301", "\x00", "\x01", "\n", "\r", "\t", "\x7f", "\x85", '"', "\\", "=", "[", "]", ",", "/", "~", "local.document"]


def kind(value):
    if value is None:
        return "null"
    if type(value) is bool:
        return "boolean"
    if isinstance(value, Number):
        return "number"
    if type(value) is str:
        return "string"
    return "object" if isinstance(value, Pairs) else "array"


def random_value(rng, depth=0, unique=False):
    choice = rng.randrange(6 if depth < 4 else 4)
    if choice == 0:
        return None
    if choice == 1:
        return rng.choice([True, False])
    if choice == 2:
        return Number(rng.choice(["-0", "1e99999999999999999999", "-0.00E+09", "999999999999999999999999999999999999999", "-1.2345678901234567890123456789", str(rng.randrange(-10000,10000))]))
    if choice == 3:
        return "".join(rng.choices(ALPHABET, k=rng.randrange(8)))
    if choice == 4:
        return [random_value(rng, depth+1, unique) for _ in range(rng.randrange(5))]
    pairs = []
    for _ in range(rng.randrange(5)):
        key = "".join(rng.choices(ALPHABET, k=rng.randrange(5)))
        if unique and any(previous == key for previous, _ in pairs):
            continue
        pairs.append((key, random_value(rng, depth+1, unique)))
    return Pairs(pairs)


def inspections(root, rng):
    programs = {opcode: compile_program(root, opcode.lower(), [op(opcode, key="out.value", source="input.json"), op("OUTPUT_FIELD", key="out.value")]) for opcode in ["JSON_KIND", "JSON_KEYS"]}
    values = [None, False, Number("-1e" + "9" * 4096), "", [], Pairs(), Pairs([("a", None), ("a", False), ("\x00", []), ("é", True), ("e\u0301", Number("0"))])]
    values += [random_value(rng) for _ in range(420)]
    values += [Pairs(("".join(rng.choices(ALPHABET,k=rng.randrange(8))),random_value(rng)) for _ in range(rng.randrange(15))) for _ in range(180)]
    for n, value in enumerate(values):
        raw = emit(value, ascii_only=n % 2 == 0)
        expected_kind = f"out.value={kind(value)}\n".encode()
        for backend in ["VM", "native"]:
            succeeded(run(programs["JSON_KIND"], backend, [f"input.json={raw}"]), backend, expected_kind)
            keys = run(programs["JSON_KEYS"], backend, [f"input.json={raw}"])
            if isinstance(value, Pairs):
                expected = ("out.value=" + canonical([key for key, _ in value]) + "\n").encode()
                succeeded(keys, backend, expected)
            else:
                failed(keys, backend)
    alias = compile_program(root, "alias", [op("COPY_FIELD", key="data.saved", source="input.json", text="snapshot"), op("JSON_KEYS", key="input.json", source="input.json"), op("JSON_KIND", key="out.kind", source="input.json"), op("JSON_KEYS", key="out.keys", source="data.saved"), op("OUTPUT_FIELD", key="out.keys"), op("OUTPUT_FIELD", key="out.kind")])
    for n in range(60):
        value = Pairs(("".join(rng.choices(ALPHABET,k=rng.randrange(10))),random_value(rng)) for _ in range(rng.randrange(10)))
        expected = ("out.keys=" + canonical([key for key, _ in value]) + "\nout.kind=array\n").encode()
        for backend in ["VM", "native"]:
            succeeded(run(alias, backend, [f"input.json={emit(value, n%2==0)}"]), backend, expected)
    print(f"PASS: {len(values)} independent six-kind/ordered-key inspections plus 60 immutable source/destination alias cases", flush=True)


def invalid_and_limits(root):
    programs = {opcode: compile_program(root, f"bad_{opcode}", [op(opcode, key="out.value", source="input.json")]) for opcode in ["JSON_KIND", "JSON_KEYS"]}
    malformed = ["", "null trailing", "[1,]", "{\"a\":1,}", "{\"ok\":0,\"hidden\":INVALID_PRIVATE_VALUE}", '{"a":"\\ud800"}', '{"a":"\\udc00"}', '{"a":"raw\nline"}', '{"a":NaN}', '{"a":Infinity}', '{"a":+1}', '{"a":01}', '{"a":.1}', '{"a":1.}', '{"a":0x1}', '[1]{}', '\x85{}', '{"a":true false}']
    for raw in malformed:
        for opcode, program in programs.items():
            for backend in ["VM", "native"]:
                result = run(program, backend, [f"input.json={raw}"])
                failed(result, backend)
                assert b"INVALID_PRIVATE_VALUE" not in result.stdout + result.stderr
    cases = [
        ("raw_exact", "{}" + " " * (LIMIT-2), True),
        ("raw_over", "{}" + " " * (LIMIT-1), False),
        ("depth_exact", '['*64+'0'+']'*64, True),
        ("depth_over", '['*65+'0'+']'*65, False),
        ("members_exact", '{'+','.join(['"k":0']*65_535)+'}', True),
        ("members_over", '{'+','.join(['"k":0']*65_536)+'}', False),
        ("canonical_exact", '"'+'\x85'*174_762+'xx'+'"', True),
        ("canonical_over", '"'+'\x85'*174_763+'"', False),
        ("key_result_max", '{"'+'x'*(LIMIT-6)+'":0}', True),
    ]
    for name, raw, valid in cases:
        for opcode in programs:
            program = compile_program(root, f"bound_{name}_{opcode}", [seed("input.json", raw), op(opcode, key="out.value", source="input.json"), op("TEXT_LENGTH", key="out.length", source="out.value"), op("OUTPUT_FIELD", key="out.length")])
            for backend in ["VM", "native"]:
                result = run(program, backend)
                if not valid:
                    failed(result, backend, "JSON-LIMIT")
                elif opcode == "JSON_KEYS" and not raw.startswith('{'):
                    failed(result, backend)
                else:
                    value = decode(raw)
                    expected = kind(value) if opcode == "JSON_KIND" else canonical([key for key, _ in value])
                    succeeded(result, backend, f"out.length={len(expected)}\n".encode())
    print(f"PASS: {len(malformed)} malformed complete sources and nine exact/overflow raw, canonical, depth, node and maximum key-result boundaries", flush=True)


def budgets(root):
    for opcode, result_type, cost in [("JSON_KIND", "Text", 6), ("JSON_KEYS", "List<Text>", 2)]:
        for function in [False, True]:
            total_cost = cost * (2 if function else 1)
            for extra in [False, True]:
                steps = [seed("input.object", "{}")]
                if opcode == "JSON_KIND":
                    steps += [seed("input.big", "x"*LIMIT), seed("input.short", "x"*(LIMIT-total_cost)), seed("input.empty", "")]
                    steps += [op("TEXT_JOIN", key="data.buffer", left="input.big", right="input.empty") for _ in range(63)]
                    steps += [op("TEXT_JOIN", key="data.buffer", left="input.short", right="input.empty")]
                else:
                    steps += [seed("input.big", '"'+'x'*(LIMIT-2)+'"'), seed("input.short", '"'+'x'*(LIMIT-total_cost-2)+'"')]
                    steps += [op("JSON_PARSE", key="data.buffer", source="input.big", type="Json") for _ in range(63)]
                    steps += [op("JSON_PARSE", key="data.buffer", source="input.short", type="Json")]
                routines = [routine("inspect", "value:Json", f"result:{result_type}", result_type, [op(opcode, key="local.result", source="local.value"), returns(result_type, "f:local.result")])]
                steps += [call("inspect", "value=f:input.object", "out.value") if function else op(opcode, key="out.value", source="input.object")]
                if extra:
                    steps.append(op(opcode, key="out.other", source="input.object"))
                steps.append(op("OUTPUT_FIELD", key="out.value"))
                program = compile_program(root, f"budget_{opcode}_{function}_{extra}", steps, routines)
                for backend in ["VM", "native"]:
                    result = run(program, backend)
                    if extra:
                        failed(result, backend, "JSON-LIMIT")
                    else:
                        succeeded(result, backend, b"out.value=object\n" if opcode == "JSON_KIND" else b"out.value=[]\n")
    print("PASS: exact 64 MiB Text and shared List/Json arenas, inspection charged once plus exactly one function-return copy, next allocation rejected", flush=True)


def checked(arguments):
    result = invoke(arguments)
    assert result.returncode == 0, (list(map(str, arguments)), result.stdout[:1000], result.stderr[-2000:])
    return result.stdout


def leaves(value):
    if isinstance(value, Pairs):
        keys = [key for key, _ in value]
        if len(set(keys)) != len(keys):
            raise OracleError("JSON-PATH")
        return sum(leaves(child) for _, child in value)
    if type(value) is list:
        return sum(leaves(child) for child in value)
    return 1


def recursive_files(root, rng):
    # The checked-in AIL example is the system under test. Every expected tree,
    # kind, leaf count, pointer ambiguity and exact report is computed in Python.
    package = REPO / "examples/json-inspection"
    metadata = (package / "ail-package.md").read_text()
    core = root / "walker.core"
    core.write_bytes(checked([AIL, "ail-core", package]))
    rendered = root / "rendered"
    rendered.mkdir()
    (rendered / "ail-package.md").write_text(metadata)
    (rendered / "spec.ail-spec.md").write_bytes(checked([AIL, "ail-spec", "--core-file", core]))
    routes = []
    for name, arguments in [("source", [package]), ("core", ["--core-file", core]), ("rendered", [rendered])]:
        bytecode = root / f"walker_{name}.json"
        bytecode.write_bytes(checked([AIL, "ail-lower"] + arguments))
        ste = root / f"walker_{name}.ail-ste"
        ste.write_bytes(checked([AIL, "ail-ste"] + arguments))
        for kind_, artifact, compile_args in [("bytecode", bytecode, arguments), ("ste", ste, [ste])]:
            binary = root / f"walker_{name}_{kind_}.elf"
            checked([AIL, "ail-compile"] + compile_args + ["--action", "Inspect", "--target", "linux-x86_64-elf", "--out", binary])
            routes.append((artifact, binary))
    values = [None, True, Number("1e999999999999999999"), "\x00é🙂", [], Pairs(), Pairs([("", [None, False]), ("/~\x00é🙂", Pairs([("~1", Number("-0")), ("a/b", "x")]))]), Pairs([("duplicate", None), ("duplicate", True)]), Pairs([("nested", [Pairs([("same", 1), ("same", 2)])])])]
    values += [random_value(rng, unique=True) for _ in range(45)]
    input_path, report_path = root / "input file=json", root / "report file=json"
    for n, value in enumerate(values):
        raw = emit(value, ascii_only=n%2==0)
        input_path.write_text(raw)
        try:
            count = leaves(value)
            expected = canonical(Pairs([("kind", kind(value)), ("leaf_count", Number(str(count))), ("source", value)])).encode()
        except OracleError:
            expected = None
        for artifact, binary in routes:
            for backend in ["VM", "native"]:
                report_path.write_bytes(b"unchanged")
                grants = [f"--read-file=input={input_path}", f"--write-file=report={report_path}"]
                prefix = [AIL, "ail-vm", artifact, "--action", "Inspect"] if backend == "VM" else [binary]
                result = invoke(prefix + grants)
                if expected is None:
                    failed(result, backend, "JSON-PATH")
                    assert report_path.read_bytes() == b"unchanged"
                else:
                    assert result.returncode == 0, (n, backend, artifact, result.stdout[:1000], result.stderr[-2000:])
                    assert report_path.read_bytes() == expected, (n, backend, report_path.read_bytes()[:1500], expected[:1500])
    print(f"PASS: {len(values)} recursive unknown-key file trees across source/saved Core/rendered source/STE × VM/native, /~NUL/Unicode pointer escaping and duplicate-name ambiguity", flush=True)


def main():
    rng = random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix="ail-json-inspection-oracle-") as directory:
        root = Path(directory)
        inspections(root, rng)
        invalid_and_limits(root)
        budgets(root)
        recursive_files(root, rng)
    print(f"PASS: independent JSON inspection oracle, seed={SEED}", flush=True)


if __name__ == "__main__":
    main()
