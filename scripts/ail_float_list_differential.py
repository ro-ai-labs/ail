#!/usr/bin/env python3
"""Independent List<Float> public-pipeline oracle (Python standard library only).

Run cargo build first. Check binary64 result bits, not Python/Rust decimal
spelling. Fixed seeds reproduce failures; temporary artifacts are removed.
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
SEED = 0x1157F10A


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=30)


def checked(arguments):
    result = invoke(arguments)
    assert result.returncode == 0, (list(map(str, arguments)), result.stdout[:1000], result.stderr[-2000:])
    return result.stdout


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions, routines=()):
    source = root / f"{name}.json"
    source.write_text(json.dumps({
        "kind": "AIL-Bytecode", "package": "float-list-oracle", "version": "1",
        "profile": "Application", "capability_grants": [], "target_support": {},
        "external_bindings": [], "failures": [],
        "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}] + list(routines),
    }))
    binary = root / f"{name}.elf"
    checked([AIL, "ail-compile", source, "--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
    return source, binary


def run(program, backend, arguments=()):
    artifact, binary = program
    prefix = [AIL, "ail-vm", artifact, "--action", "Run"] if backend == "VM" else [binary]
    return invoke(prefix + list(arguments))


def outputs(result, backend):
    assert result.returncode == 0, (backend, result.stdout[:1000], result.stderr[-2000:])
    fields = {}
    for line in result.stdout.decode().splitlines():
        if line.startswith("out."):
            key, value = line.split("=", 1)
            fields[key] = value
    return fields


def failed(result, backend, category):
    assert result.returncode != 0, (backend, result.stdout[:1000], result.stderr[-1000:])
    assert f"AIL-RUNTIME-{category}:".encode() in result.stdout + result.stderr, (backend, category, result.stdout[:1000], result.stderr[-2000:])
    if backend == "native":
        assert not result.stdout, result.stdout[:1000]


def bits(value):
    return 0x7FF8000000000000 if math.isnan(value) else struct.unpack(">Q", struct.pack(">d", value))[0]


def matching(actual, expected, label):
    assert bits(float(actual)) == bits(expected), (label, actual, expected, hex(bits(float(actual))), hex(bits(expected)))
    if math.isnan(expected):
        assert actual == "nan", (label, actual)
    elif math.isinf(expected):
        assert actual == ("-inf" if expected < 0 else "inf"), (label, actual)
    else:
        assert "e" in actual and "E" not in actual, (label, actual)


TOKEN = re.compile(r"[+-]?(?:(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|inf|nan)", re.ASCII)
WS = " \t\r\n"


def runtime_list(raw):
    if len(raw.encode()) > LIMIT:
        raise ValueError("LIST-LIMIT")
    text = raw.strip(WS)
    if not text.startswith("[") or not text.endswith("]"):
        raise ValueError("LIST")
    text = text[1:-1]
    if not text.strip(WS):
        return []
    tokens = [token.strip(WS) for token in text.split(",")]
    if any(not TOKEN.fullmatch(token) for token in tokens):
        raise ValueError("LIST")
    if len(tokens) > 65_536:
        raise ValueError("LIST-LIMIT")
    return [float(token) for token in tokens]


def matching_list(actual, expected, label):
    parsed = runtime_list(actual)
    assert [bits(value) for value in parsed] == [bits(value) for value in expected], (label, actual[:200], expected[:20])
    assert not any(c in actual for c in WS), (label, actual[:200])
    if actual != "[]":
        for token, value in zip(actual[1:-1].split(","), expected):
            matching(token, value, label)


def float_token(rng):
    if rng.randrange(3) == 0:
        return rng.choice(["-0", "+0", "0", "-nan", "+nan", "nan", "+inf", "-inf", "inf", ".5", "-1.", "+001", "5e-324", "-1e-9999", "1e9999", "1.7976931348623157e308"])
    value = struct.unpack(">d", struct.pack(">Q", rng.getrandbits(64)))[0]
    return repr(value)


def operations(root, rng):
    steps = [op("COPY_FIELD", source="input.values", key="data.saved", text="snapshot"), op("LIST_LENGTH", type="Float", source="input.values", key="out.count"), op("LIST_GET", type="Float", source="input.values", index="f:input.index", key="out.item"), op("LIST_SET", type="Float", source="input.values", index="f:input.index", value="f:input.value d:2 / neg", key="input.values"), op("LIST_APPEND", type="Float", source="input.values", value="f:input.value d:2 / neg", key="out.values"), op("LIST_GET", type="Float", source="data.saved", index="f:input.index", key="out.saved"), op("LIST_EMPTY", type="Float", key="out.empty")]
    steps += [op("OUTPUT_FIELD", key=f"out.{key}") for key in ["count", "item", "saved", "values", "empty"]]
    program = compile_program(root, "operations", steps)
    for n in range(650):
        tokens = [float_token(rng) for _ in range(rng.randrange(1, 18))]
        raw = rng.choice(["", " \r\n"]) + "[" + rng.choice([",", " ,\t", ",\n"]).join(tokens) + "]" + rng.choice(["", "\t "])
        values = [float(value) for value in tokens]
        replacement = float_token(rng)
        computed = -(float(replacement) / 2)
        index = rng.randrange(-len(values), len(values))
        expected = values.copy()
        expected[index] = computed
        expected.append(computed)
        parity = []
        for backend in ["VM", "native"]:
            fields = outputs(run(program, backend, [f"input.values={raw}", f"input.index={index}", f"input.value={replacement}"]), backend)
            assert fields["out.count"] == str(len(values)) and fields["out.empty"] == "[]"
            matching(fields["out.item"], values[index], (n, backend, "get"))
            matching(fields["out.saved"], values[index], (n, backend, "immutable copy"))
            matching_list(fields["out.values"], expected, (n, backend, "set/append"))
            parity.append(fields)
        assert parity[0] == parity[1], (n, parity)
    print("PASS: 650 seeded all-operation pipelines with Float expressions, negative indices, signed zero/specials and immutable aliases", flush=True)


def grammar(root, rng):
    program = compile_program(root, "grammar", [op("LIST_LENGTH", type="Float", source="input.values", key="out.count"), op("OUTPUT_FIELD", key="out.count")])
    invalid = ["", "null", "{}", "[true]", "[false]", "[null]", '["1"]', "[[1]]", "[NaN]", "[Infinity]", "[1_0]", "[0x1p0]", "[1,]", "[,]", "[1,,2]", "[1]extra", "[1 e1]", "[+ inf]", "[1e+]", "[١]", "[\v1]", "\x85[1]", "[1\x00]"]
    samples = invalid[:-1] + ["[]", " \n[+001, .5, -0, 1., nan, -inf]\t", "[1e99999, -1e-99999]"]
    cases = invalid[:-1].copy()
    alphabet = list("[]+-.,0129einfNaNtrue ") + ["\t", "\n", "\v", "\x85", "é"]
    for _ in range(650):
        raw = rng.choice(samples)
        for _ in range(rng.randrange(1, 4)):
            pos = rng.randrange(len(raw)+1)
            raw = raw[:pos] + (rng.choice(alphabet) if rng.randrange(3) else "") + raw[min(pos+1, len(raw)):]
        cases.append(raw)
    for n, raw in enumerate(cases):
        try:
            count = len(runtime_list(raw))
        except ValueError as error:
            expected_error = str(error)
        else:
            expected_error = None
        for backend in ["VM", "native"]:
            result = run(program, backend, [f"input.values={raw}"])
            if expected_error:
                failed(result, backend, expected_error)
            else:
                assert outputs(result, backend)["out.count"] == str(count), (n, backend, raw)
    nul = compile_program(root, "nul", [op("SET_FIELD", key="input.values", value="[1\x00]", text="seed"), op("LIST_LENGTH", type="Float", source="input.values", key="out.count")])
    for backend in ["VM", "native"]:
        failed(run(nul, backend), backend, "LIST")
    set_ = compile_program(root, "priority", [op("LIST_SET", type="Float", source="input.values", index="f:input.index", value="f:input.value d:0 /", key="out.values")])
    for raw, index, value, error in [("[broken]", "bad", "bad", "LIST"), ("[1]", "bad", "bad", "INT"), ("[]", "0", "bad", "INDEX"), ("[1]", "-2", "bad", "INDEX"), ("[1]", "0", "bad", "FLOAT"), ("[1]", "0", "1", "DIVZERO")]:
        for backend in ["VM", "native"]:
            failed(run(set_, backend, [f"input.values={raw}", f"input.index={index}", f"input.value={value}"]), backend, error)
    print(f"PASS: {len(cases)} independent grammar mutations, embedded NUL rejection and source/index/value error priority", flush=True)


def identity():
    return {"action": "identity", "instructions": [op("VALUE_FUNCTION", parameters="values:List<Float>", locals="", result="List<Float>"), op("RETURN_VALUE", type="List<Float>", expression="f:local.values")]}


def boundaries(root):
    for name, raw, expected_error in [
        ("raw_exact", "[0]" + " " * (LIMIT-3), None),
        ("raw_over", "[0]" + " " * (LIMIT-2), "LIST-LIMIT"),
        ("items_exact", "[" + ",".join(["0"] * 65_536) + "]", None),
        ("items_over", "[" + ",".join(["0"] * 65_537) + "]", "LIST-LIMIT"),
    ]:
        program = compile_program(root, name, [op("SET_FIELD", key="input.values", value=raw, text="seed"), op("LIST_LENGTH", type="Float", source="input.values", key="out.count"), op("OUTPUT_FIELD", key="out.count")])
        for backend in ["VM", "native"]:
            result = run(program, backend)
            if expected_error:
                failed(result, backend, expected_error)
            else:
                assert outputs(result, backend)["out.count"] == str(65_536 if name == "items_exact" else 1)
    exact_items = ["1.2345678901e10"] * 65_535
    exact_items[0:2] = ["1.2345678901234567e200", "1.2345678901234567e-200"]
    exact = "[" + ",".join(exact_items) + "]"
    assert len(exact) == LIMIT
    # Parameter validation, scalar get/length, and caller destination writes
    # must not charge; each returned list copy charges its final bytes once.
    for extra, category in [(None, None), ("return", "LIST-LIMIT"), ("json", "JSON-LIMIT")]:
        steps = [op("SET_FIELD", key="input.values", value=exact, text="seed")]
        steps += [op("CALL_VALUE", target="identity", arguments="values=f:input.values", key="out.values") for _ in range(64)]
        steps += [op("LIST_LENGTH", type="Float", source="out.values", key="out.count"), op("LIST_GET", type="Float", source="out.values", index="i:-1", key="out.item")]
        if extra == "return":
            steps.append(op("CALL_VALUE", target="identity", arguments="values=f:input.values", key="out.values"))
        if extra == "json":
            steps += [op("SET_FIELD", key="input.raw", value="[]", text="seed"), op("JSON_PARSE", type="Json", source="input.raw", key="out.document")]
        steps += [op("OUTPUT_FIELD", key="out.count"), op("OUTPUT_FIELD", key="out.item")]
        program = compile_program(root, f"budget_{extra}", steps, [identity()])
        for backend in ["VM", "native"]:
            result = run(program, backend)
            if category:
                failed(result, backend, category)
            else:
                fields = outputs(result, backend)
                assert fields["out.count"] == "65535"
                matching(fields["out.item"], float(exact_items[-1]), backend)
    # Strict flat conversion counts items rather than recursive JSON nodes.
    raw = "[" + ",".join(["-0"] * 65_536) + "]"
    for kind in ["parse", "literal"]:
        operation = op("JSON_PARSE", source="input.raw", key="out.values", type="List<Float>") if kind == "parse" else op("ASSIGN_LITERAL", key="out.values", type="List<Float>", value=raw)
        program = compile_program(root, f"strict_full_{kind}", [op("SET_FIELD", key="input.raw", value=raw, text="seed"), operation, op("LIST_LENGTH", type="Float", source="out.values", key="out.count"), op("LIST_GET", type="Float", source="out.values", index="i:-1", key="out.item"), op("OUTPUT_FIELD", key="out.count"), op("OUTPUT_FIELD", key="out.item")])
        for backend in ["VM", "native"]:
            fields = outputs(run(program, backend), backend)
            assert fields["out.count"] == "65536"
            matching(fields["out.item"], -0., backend)
    print("PASS: exact/over raw-byte and item bounds, 1 MiB canonical results, exact 64 MiB shared List/Json return arena and 65,536-item strict arrays", flush=True)


def source_routes_and_files(root, rng):
    package = root / "source"
    package.mkdir()
    metadata = "# AIL Package\nname: float-list-oracle\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, functions, traces\nconformance: first-slice\n"
    (package / "ail-package.md").write_text(metadata)
    spec = '''The application FloatListOracle manages numerical reports.
A Data has:
- raw: Text
- document: Json
- path: Text
- values: List<Float>
- bias: Float
An Output has:
- values: List<Float>
- raw: Text
Function: shift.
The function needs:
- bias: Float
- values: List<Float>
The function produces:
- result: List<Float>
The function stores:
- count: Int
- index: Int
- item: Float
- result: List<Float>
The function executes:
- the function creates an empty list into local.result
- the function counts items in list local.values into local.count
- the function assigns 0 to local.index
- while local.index < local.count:
- the function gets item local.index from list local.values into local.item
- the function appends floating (local.item + local.bias) / 2 to list local.result into local.result
- the function computes local.index as local.index + 1
- end while
- the function returns local.result
Action: Run.
When Run happens:
- the system reads text from file resource input into data.raw
- the system parses JSON text data.raw into data.document
- the system assigns "/values" to data.path
- the system gets JSON value at path data.path from data.document into data.values
- the system calls shift with bias = data.bias; values = data.values into output.values
- the system assigns "/result" to data.path
- the system sets JSON value at path data.path in data.document to output.values into data.document
- the system formats data.document as JSON text into output.raw
- the system writes text output.raw to file resource report
- the system returns output.values
- the system records a trace event named Done
'''
    (package / "spec.ail-spec.md").write_text(spec)
    core = root / "saved.core"
    core.write_bytes(checked([AIL, "ail-core", package]))
    rendered = root / "rendered"
    rendered.mkdir()
    (rendered / "ail-package.md").write_text(metadata)
    (rendered / "spec.ail-spec.md").write_bytes(checked([AIL, "ail-spec", "--core-file", core]))
    routes = []
    for name, arguments in [("source", [package]), ("core", ["--core-file", core]), ("rendered", [rendered])]:
        bytecode = root / f"route-{name}.json"
        bytecode.write_bytes(checked([AIL, "ail-lower"] + arguments))
        ste = root / f"route-{name}.ail-ste"
        ste.write_bytes(checked([AIL, "ail-ste"] + arguments))
        for kind, artifact, compile_args in [("bytecode", bytecode, arguments), ("ste", ste, [ste])]:
            binary = root / f"route-{name}-{kind}.elf"
            checked([AIL, "ail-compile"] + compile_args + ["--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
            routes.append((artifact, binary))
    input_path, report_path = root / "input file=json", root / "report file=json"
    for n in range(36):
        values = [rng.choice([-0., 5e-324, rng.uniform(-1e10, 1e10)]) for _ in range(rng.randrange(12))]
        bias = rng.uniform(-100, 100)
        note = rng.choice(["\x00é🙂", "quotes\"slashes\\", "new\nline", ""])
        input_path.write_text('{"values":' + json.dumps(values) + ',"exact":1.2345678901234567890123456789,"note":' + json.dumps(note) + '}')
        expected = [(value + bias) / 2 for value in values]
        parity = []
        for route in routes:
            for backend in ["VM", "native"]:
                result = run(route, backend, [f"--read-file=input={input_path}", f"--write-file=report={report_path}", f"data.bias={bias!r}"])
                assert result.returncode == 0, (n, backend, result.stdout[:1000], result.stderr[-2000:])
                report = report_path.read_text()
                parsed = json.loads(report, parse_float=str, parse_int=str)
                assert parsed["exact"] == "1.2345678901234567890123456789" and parsed["note"] == note
                assert [bits(float(value)) for value in parsed["result"]] == [bits(value) for value in expected]
                assert [bits(float(value)) for value in parsed["values"]] == [bits(value) for value in values]
                parity.append(report)
        assert len(set(parity)) == 1, n
    print("PASS: 36 numerical file reports across source/saved Core/rendered source/STE × VM/native, local list maps, Unicode/NUL and lossless JSON neighbors", flush=True)


def main():
    rng = random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix="ail-float-list-oracle-") as directory:
        root = Path(directory)
        operations(root, rng)
        grammar(root, rng)
        boundaries(root)
        source_routes_and_files(root, rng)
    print(f"PASS: independent List<Float> public pipeline oracle, seed={SEED}", flush=True)


if __name__ == "__main__":
    main()
