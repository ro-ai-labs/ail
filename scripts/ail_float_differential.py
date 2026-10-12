#!/usr/bin/env python3
"""Independent binary64 public-pipeline oracle (Python standard library only).

Run cargo build first. Check binary64 result bits, not Python/Rust decimal
spelling. Fixed seeds reproduce failures; temporary artifacts are removed.
"""
import json
import math
from pathlib import Path
import random
import struct
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576
SEED = 0xF10A710


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
        "kind": "AIL-Bytecode", "package": "float-oracle", "version": "1",
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


def routine():
    return {"action": "blend", "instructions": [
        op("VALUE_FUNCTION", parameters="a:Float;b:Float", locals="result:Float", result="Float"),
        op("EVAL_FLOAT", key="local.result", expression="f:local.a f:local.b + d:2 /"),
        op("RETURN_VALUE", type="Float", expression="f:local.result"),
    ]}


def arithmetic(root, rng):
    instructions = []
    for name, symbol in [("add", "+"), ("sub", "-"), ("mul", "*"), ("div", "/"), ("mod", "%")]:
        instructions += [op("EVAL_FLOAT", key=f"out.{name}", expression=f"f:input.a f:input.b {symbol}"), op("OUTPUT_FIELD", key=f"out.{name}")]
    instructions += [op("CALL_VALUE", target="blend", arguments="a=f:input.a;b=f:input.b", key="out.call"), op("OUTPUT_FIELD", key="out.call")]
    for index, comparison in enumerate(["==", "!=", "<", "<=", ">", ">="]):
        key = f"out.c{index}"
        instructions += [op("EVAL_INT", key=key, expression="i:0"), op("BRANCH_FLOAT", left="f:input.a", comparison=comparison, right="f:input.b", label=f"yes{index}"), op("JUMP", label=f"done{index}"), op("LABEL", name=f"yes{index}"), op("EVAL_INT", key=key, expression="i:1"), op("LABEL", name=f"done{index}"), op("OUTPUT_FIELD", key=key)]
    program = compile_program(root, "arithmetic", instructions, [routine()])
    extremes = [0.0, -0.0, 5e-324, -5e-324, 2.2250738585072014e-308, -1.7976931348623157e308, 1.7976931348623157e308, math.inf, -math.inf, math.nan, 1., -1., 0.1, -0.1, float(2**53 + 1)]
    cases = [(a, b) for a in extremes for b in extremes if b != 0]
    for _ in range(1000):
        a, b = [struct.unpack(">d", struct.pack(">Q", rng.getrandbits(64)))[0] for _ in range(2)]
        if b != 0:
            cases.append((a, b))
    for index, (a, b) in enumerate(cases):
        expected = {"add": a + b, "sub": a - b, "mul": a * b, "div": a / b, "mod": a % b, "call": (a + b) / 2}
        comparisons = [a == b, a != b, a < b, a <= b, a > b, a >= b]
        parity = []
        for backend in ["VM", "native"]:
            fields = outputs(run(program, backend, [f"input.a={a!r}", f"input.b={b!r}"]), backend)
            for name, value in expected.items():
                matching(fields[f"out.{name}"], value, (index, backend, name, a, b))
            for n, value in enumerate(comparisons):
                assert fields[f"out.c{n}"] == str(int(value)), (index, backend, n, a, b, fields)
            parity.append(fields)
        assert parity[0] == parity[1], (index, parity)
    print(f"PASS: {len(cases)} seeded argument pairs × six arithmetic/function results and six IEEE comparisons, VM/native bit parity", flush=True)


def conversion_and_grammar(root, rng):
    identity = compile_program(root, "identity", [op("CALL_VALUE", target="identity", arguments="value=f:input.value", key="out.value"), op("OUTPUT_FIELD", key="out.value")], [{"action": "identity", "instructions": [op("VALUE_FUNCTION", parameters="value:Float", locals="", result="Float"), op("RETURN_VALUE", type="Float", expression="f:local.value")]}])
    valid = ["+inf", "-inf", "inf", "nan", "+nan", "-nan", "0", "-0", "+0", ".1", "-.1", "1.", "01", "1.e2", "1E-99", "1e99999", "-1e-99999", "2.4703282292062327208828439643411068618252990130716238221279284125e-324", "1.00000000000000011102230246251565404236316680908203125"]
    valid += [f"{rng.randrange(-10**30, 10**30)}e{rng.randrange(-340,310)}" for _ in range(150)]
    for raw in valid:
        for backend in ["VM", "native"]:
            matching(outputs(run(identity, backend, [f"input.value={raw}"]), backend)["out.value"], float(raw), (backend, raw))
    invalid = ["", " ", " 1", "1 ", "1\t", "1\n", "NaN", "NAN", "Inf", "infinity", "0x1p0", "1_0", "1e", "1e+", "+", "-", ".", "1..", "--1", "1,2", "１", "١", "∞"]
    for raw in invalid:
        for backend in ["VM", "native"]:
            failed(run(identity, backend, [f"input.value={raw}"]), backend, "FLOAT")
    for backend in ["VM", "native"]:
        failed(run(identity, backend), backend, "FLOAT")
    for opcode in ["INT_TO_FLOAT", "FLOAT_TO_INT"]:
        program = compile_program(root, opcode.lower(), [op(opcode, source="input.value", key="out.value"), op("OUTPUT_FIELD", key="out.value")])
        values = [-(2**63), 2**63 - 1, 0, 2**53 + 1, -(2**53 + 1)] + [rng.randrange(-2**63, 2**63) for _ in range(100)] if opcode == "INT_TO_FLOAT" else [-(2.**63), 2.**63, math.nextafter(2.**63, 0), math.nextafter(-(2.**63), -math.inf), math.inf, -math.inf, math.nan, -0., -3.999] + [rng.uniform(-1e20,1e20) for _ in range(100)]
        for value in values:
            for backend in ["VM", "native"]:
                result = run(program, backend, [f"input.value={value!r}"])
                if opcode == "INT_TO_FLOAT":
                    matching(outputs(result, backend)["out.value"], float(value), (backend, value))
                elif math.isfinite(value) and -(2**63) <= value < 2**63:
                    assert outputs(result, backend)["out.value"] == str(math.trunc(value)), (backend, value)
                else:
                    failed(result, backend, "FLOAT")
    for length in [LIMIT, LIMIT + 1]:
        program = compile_program(root, f"limit{length}", [op("SET_FIELD", key="input.value", value="0" * (length-1) + "1", text="seed"), op("CALL_VALUE", target="identity", arguments="value=f:input.value", key="out.value"), op("OUTPUT_FIELD", key="out.value")], [{"action": "identity", "instructions": [op("VALUE_FUNCTION", parameters="value:Float", locals="", result="Float"), op("RETURN_VALUE", type="Float", expression="f:local.value")]}])
        for backend in ["VM", "native"]:
            result = run(program, backend)
            if length == LIMIT:
                matching(outputs(result, backend)["out.value"], 1.0, backend)
            else:
                failed(result, backend, "FLOAT-LIMIT")
    print(f"PASS: {len(valid)} decimal transports, {len(invalid)} malformed transports, 214 cast boundaries/random values, exact/over 1 MiB parameter bounds", flush=True)


def source_routes(root, rng):
    package = root / "source"
    package.mkdir()
    metadata = "# AIL Package\n\nname: float-oracle\nversion: 1.0.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, expressions, functions, traces\nconformance: first-slice\n"
    (package / "ail-package.md").write_text(metadata)
    values = [-0., 5e-324, -1.7976931348623157e308] + [rng.uniform(-1e50,1e50) for _ in range(50)]
    declarations = "\n".join(f"- v{n}: Float" for n in range(len(values)))
    statements = []
    expected = []
    for n, value in enumerate(values):
        statements += [f"- the system assigns {value!r} to output.v{n}", f"- the system calls transform with value = output.v{n} into output.v{n}", f"- the system returns output.v{n}"]
        expected.append(-(value / 2.0))
    spec = "The application Oracle manages floating values.\nAn Output has:\n" + declarations + "\n\nFunction: transform.\nThe function needs:\n- value: Float\nThe function produces:\n- result: Float\nThe function stores:\n- result: Float\nThe function executes:\n- the function computes floating local.result as -(local.value / 2)\n- the function returns local.result\n\nAction: Run.\nWhen Run happens:\n" + "\n".join(statements) + "\n- the system records a trace event named Done\n"
    (package / "spec.ail-spec.md").write_text(spec)
    core = root / "saved.core"
    core.write_bytes(checked([AIL, "ail-core", package]))
    rendered = root / "rendered"
    rendered.mkdir()
    (rendered / "ail-package.md").write_text(metadata)
    (rendered / "spec.ail-spec.md").write_bytes(checked([AIL, "ail-spec", "--core-file", core]))
    for name, arguments in [("source", [package]), ("core", ["--core-file", core]), ("rendered", [rendered])]:
        bytecode = root / f"route-{name}.json"
        bytecode.write_bytes(checked([AIL, "ail-lower"] + arguments))
        ste = root / f"route-{name}.ail-ste"
        ste.write_bytes(checked([AIL, "ail-ste"] + arguments))
        binaries = []
        for kind, compile_args in [("source", arguments), ("ste", [ste])]:
            binary = root / f"route-{name}-{kind}.elf"
            checked([AIL, "ail-compile"] + compile_args + ["--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
            binaries.append(binary)
        results = [checked([AIL, "ail-vm", artifact, "--action", "Run"]) for artifact in [bytecode, ste]] + [checked([binary]) for binary in binaries]
        for raw in results:
            actual = dict(line.split("=", 1) for line in raw.decode().splitlines() if line.startswith("output.v"))
            assert len(actual) == len(expected), (name, len(actual))
            for n, value in enumerate(expected):
                matching(actual[f"output.v{n}"], value, (name, n))
    print(f"PASS: {len(values)} source literals and aliasing Float function returns across source/saved Core/rendered source/STE × VM/native", flush=True)


def json_and_files(root, rng):
    instructions = [op("FILE_READ_TEXT", key="data.raw", resource="input"), op("JSON_PARSE", key="data.document", source="data.raw", type="Json"), op("ASSIGN_LITERAL", key="data.path", type="Text", value='"/value"'), op("JSON_GET", key="data.value", source="data.document", path="data.path", type="Float"), op("EVAL_FLOAT", key="data.value", expression="f:data.value d:9 * d:5 / d:32 +"), op("JSON_SET", key="data.document", source="data.document", path="data.path", type="Float", value="data.value"), op("JSON_FORMAT", key="data.report", source="data.document", type="Json"), op("FILE_WRITE_TEXT", source="data.report", resource="report")]
    program = compile_program(root, "file-report", instructions)
    input_path, report_path = root / "input file=json", root / "output file=json"
    for n in range(60):
        value = rng.uniform(-1e6,1e6)
        note = rng.choice(["", "é🙂", "\x00with newline\n", "quotes\"slashes\\"])
        input_path.write_text('{"value":' + repr(value) + ',"exact":1.2345678901234567890123456789,"note":' + json.dumps(note) + '}')
        expected = value * 9 / 5 + 32
        parity = []
        for backend in ["VM", "native"]:
            result = run(program, backend, [f"--read-file=input={input_path}", f"--write-file=report={report_path}"])
            assert result.returncode == 0, (n, backend, result.stdout[:1000], result.stderr[-2000:])
            encoded = report_path.read_text()
            parsed = json.loads(encoded, parse_float=str)
            matching(parsed["value"], expected, (n, backend))
            assert parsed["exact"] == "1.2345678901234567890123456789" and parsed["note"] == note
            parity.append(encoded)
        assert parity[0] == parity[1]
    parse = compile_program(root, "json-parse", [op("JSON_PARSE", key="out.value", source="input.raw", type="Float"), op("OUTPUT_FIELD", key="out.value")])
    format_ = compile_program(root, "json-format", [op("JSON_FORMAT", key="out.value", source="input.raw", type="Float"), op("OUTPUT_FIELD", key="out.value")])
    for raw in ["-0", " 1.250e-4 ", "1e-99999", "1.7976931348623157e308", "4.9406564584124654e-324"]:
        for backend in ["VM", "native"]:
            matching(outputs(run(parse, backend, [f"input.raw={raw}"]), backend)["out.value"], float(raw), (backend, raw))
    for program_, values in [(parse, ["nan", "inf", "1e99999", "+1", "01", ".1", "1.", "true", '"1"']), (format_, ["nan", "+nan", "inf", "-inf", "1e99999", "garbage"])]:
        for raw in values:
            for backend in ["VM", "native"]:
                failed(run(program_, backend, [f"input.raw={raw}"]), backend, "JSON")
    print("PASS: 60 real Unicode/NUL file transformations with lossless JSON neighbors; strict Float JSON parse/format rejection", flush=True)


def main():
    rng = random.Random(SEED)
    with tempfile.TemporaryDirectory(prefix="ail-float-oracle-") as directory:
        root = Path(directory)
        arithmetic(root, rng)
        conversion_and_grammar(root, rng)
        source_routes(root, rng)
        json_and_files(root, rng)
    print(f"PASS: independent Float public pipeline oracle, seed={SEED}", flush=True)


if __name__ == "__main__":
    main()
