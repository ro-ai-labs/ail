#!/usr/bin/env python3
"""Independent Python oracle for typed JSON conversion. Run cargo build first.

Checks canonical bytes, strict grammar, aliases, size limits and allocation
accounting against VM and directly emitted ELF without changing frozen benches.
"""
import json
from pathlib import Path
import random
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576
TYPES = ["Int", "Bool", "Text", "List<Int>", "List<Text>"]


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=30)


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def quoted(value):
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + "".join(escapes[c] if c in escapes else f"\\u{ord(c):04x}" if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value) + '"'


def canonical(value):
    if type(value) is str:
        return quoted(value)
    if type(value) is bool:
        return "true" if value else "false"
    if type(value) is int:
        return str(value)
    return "[" + ",".join(canonical(item) for item in value) + "]"


def transport(value):
    return value if type(value) is str else canonical(value)


def compile_program(root, name, instructions):
    source = root / f"{name}.json"
    source.write_text(json.dumps({
        "kind": "AIL-Bytecode", "package": "json-oracle", "version": "1",
        "profile": "Application", "capability_grants": [], "target_support": {},
        "external_bindings": [], "failures": [],
        "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}],
    }))
    binary = root / f"{name}.elf"
    result = invoke([AIL, "ail-compile", source, "--action", "Run", "--target", "linux-x86_64-elf", "--out", binary])
    assert result.returncode == 0, (name, result.stdout[:1000], result.stderr[:2000])
    return source, binary


def run(program, backend, arguments=()):
    source, binary = program
    command = [AIL, "ail-vm", source, "--action", "Run"] if backend == "VM" else [binary]
    return invoke(command + list(arguments))


def succeeded(result, backend, expected):
    assert result.returncode == 0, (backend, result.stdout[:1000], result.stderr[-2000:])
    if backend == "VM":
        assert result.stdout.startswith(b"ail-vm succeeded\n" + expected), (backend, result.stdout[:1000], expected[:1000])
    else:
        assert result.stdout == expected, (backend, result.stdout[:1000], expected[:1000])


def failed(result, backend, category="AIL-RUNTIME-JSON:"):
    assert result.returncode != 0, (backend, result.stdout[:1000], result.stderr[-1000:])
    assert category.encode() in result.stdout + result.stderr, (backend, category, result.stdout[:1000], result.stderr[-2000:])
    if backend == "native":
        assert not result.stdout, result.stdout[:1000]


def main():
    random.seed(827_351)
    with tempfile.TemporaryDirectory(prefix="ail-json-oracle-") as temporary:
        root = Path(temporary)
        characters = ["a", " ", "é", "🙂", "\U0010ffff", "\u0301", "\0", "\x01", "\b", "\f", "\n", "\r", "\t", "\x7f", "\x85", '"', "\\", "=", "[", "]", ",", " to ", "data.value"]
        def text():
            return "".join(random.choices(characters, k=random.randrange(20)))
        def integer():
            return random.choice([-(2**63), 2**63 - 1, 0, random.randrange(-1_000_000, 1_000_000)])
        count = 0
        for type_name in TYPES:
            program = compile_program(root, "roundtrip_" + type_name.replace("<", "_").replace(">", ""), [
                op("JSON_PARSE", key="data.value", source="input.raw", type=type_name),
                op("COPY_FIELD", key="out.copy", source="data.value", text="retain decoded value"),
                op("JSON_FORMAT", key="data.value", source="data.value", type=type_name),
                op("COPY_FIELD", key="out.json", source="data.value", text="retain canonical JSON"),
                op("JSON_PARSE", key="out.again", source="data.value", type=type_name),
                op("OUTPUT_FIELD", key="out.again"), op("OUTPUT_FIELD", key="out.copy"), op("OUTPUT_FIELD", key="out.json"),
            ])
            for idx in range(100):
                value = integer() if type_name == "Int" else random.choice([True, False]) if type_name == "Bool" else text() if type_name == "Text" else [integer() if type_name == "List<Int>" else text() for _ in range(random.randrange(8))]
                raw = json.dumps(value, ensure_ascii=idx % 2 == 0, separators=(",", ":") if idx % 3 else (", ", ":"))
                if idx % 4 == 0:
                    raw = " \t\r\n" + raw + "\r\n "
                if type_name == "Int" and value == 0 and idx % 2:
                    raw = " -0 "
                expected = f"out.again={transport(value)}\nout.copy={transport(value)}\nout.json={canonical(value)}\n".encode()
                for backend in ["VM", "native"]:
                    succeeded(run(program, backend, ["input.raw=" + raw]), backend, expected)
                count += 1
        print(f"PASS: {count} independent Python/VM/native typed JSON round trips, aliases and immutable copies")

        malformed = {
            "Int": ["", "null", "true", '"1"', "[]", "{}", "+1", "01", "-01", "1.0", "1e0", "NaN", "Infinity", "9223372036854775808", "-9223372036854775809", "\x851", "1\x85", "1 2", "--1", "-"],
            "Bool": ["", "0", "1", '"true"', "True", "FALSE", "null", "[]", "{}", "true false", "\vtrue"],
            "Text": ["", "null", "[]", "{}", "true", "1", '"unterminated', '"\\ud800"', '"\\udc00"', '"\\ud800\\u0000"', '"\\u+041"', '"\\u004"', '"\\x00"', '"raw\nline"', '"ok" trailing', '\x85"ok"', '"ok"\x85'],
            "List<Int>": ["", "null", "{}", "[[1]]", "[true]", "[null]", "[,]", "[1,]", "[1] trailing", "[+1]", "[01]", "[-01]", "[1.]", "[1.0]", "[1e0]", "[NaN]", "[Infinity]", "[9223372036854775808]", "[-9223372036854775809]", '[1,"wrong"]', "[\v1]", "[\x851]"],
            "List<Text>": ["[1]", '["ok",1]', '["\\ud800"]', '["\\udc00"]', '["\\ud800\\u0000"]', '["\\u+041"]', '["\\u004"]', '["\\x00"]', '["raw\nline"]', '["unterminated]', '[["x"]]', '["x",]', '["x"] junk', '\x85["x"]'],
        }
        invalids = 0
        for type_name in TYPES:
            for opcode in ["JSON_PARSE", "JSON_FORMAT"]:
                program = compile_program(root, "invalid", [op(opcode, key="out.value", source="input.raw", type=type_name), op("OUTPUT_FIELD", key="out.value")])
                bad = malformed[type_name] if opcode == "JSON_PARSE" or type_name.startswith("List") else ["", " 1", "1 ", "1.0", "1e0", "9223372036854775808", "confidential_bad_value"] if type_name == "Int" else ["", "True", "FALSE", "0", " true"] if type_name == "Bool" else []
                for raw in bad:
                    for backend in ["VM", "native"]:
                        result = run(program, backend, ["input.raw=" + raw, "out.value=old"])
                        failed(result, backend)
                        assert b"confidential_bad_value" not in result.stdout + result.stderr
                    invalids += 1
                for backend in ["VM", "native"]:
                    failed(run(program, backend), backend)
                for invalid in [b"\x80", b"\xc0\xaf", b"\xed\xa0\x80", b"\xf4\x90\x80\x80", b"\xff"]:
                    failed(run(program, "native", [b"input.raw=" + invalid]), "native")
        for raw, expected in [("+00042", "42"), ("-000", "0"), ("-09223372036854775808", "-9223372036854775808"), ("+09223372036854775807", "9223372036854775807")]:
            program = compile_program(root, "integer_transport", [op("JSON_FORMAT", key="out.value", source="input.raw", type="Int"), op("OUTPUT_FIELD", key="out.value")])
            for backend in ["VM", "native"]:
                succeeded(run(program, backend, ["input.raw=" + raw]), backend, f"out.value={expected}\n".encode())
        print(f"PASS: {invalids} strict malformed parse/format cases, missing fields, native invalid UTF-8 and signed integer transport rules")

        # This independent dataset path uses real resource-bound files, indexed
        # list mutation and arithmetic, beyond isolated conversion instructions.
        dataset = compile_program(root, "dataset", [
            op("FILE_READ_TEXT", resource="input", key="data.raw"),
            op("JSON_PARSE", key="data.values", source="data.raw", type="List<Int>"),
            op("LIST_LENGTH", key="out.count", source="data.values", type="Int"),
            op("EVAL_INT", key="data.index", expression="i:0"),
            op("EVAL_INT", key="data.total", expression="i:0"),
            op("LABEL", name="again"),
            op("BRANCH_INT", left="f:data.index", right="f:out.count", comparison=">=", label="done"),
            op("LIST_GET", key="data.item", source="data.values", index="f:data.index", type="Int"),
            op("EVAL_INT", key="data.item", expression="f:data.item f:data.item *"),
            op("LIST_SET", key="data.values", source="data.values", index="f:data.index", value="f:data.item", type="Int"),
            op("EVAL_INT", key="data.total", expression="f:data.total f:data.item +"),
            op("EVAL_INT", key="data.index", expression="f:data.index i:1 +"),
            op("JUMP", label="again"),
            op("LABEL", name="done"),
            op("JSON_FORMAT", key="data.squared_json", source="data.values", type="List<Int>"),
            op("JSON_FORMAT", key="data.total_json", source="data.total", type="Int"),
            op("FILE_WRITE_TEXT", resource="squared", source="data.squared_json"),
            op("FILE_WRITE_TEXT", resource="total", source="data.total_json"),
            op("OUTPUT_FIELD", key="out.count"),
        ])
        input_path, squared_path, total_path = [root / name for name in ["input.json", "squared.json", "total.json"]]
        grants = [f"--read-file=input={input_path}", f"--write-file=squared={squared_path}", f"--write-file=total={total_path}"]
        for dataset_index in range(60):
            values = [] if dataset_index == 0 else [random.randrange(-1_000_000, 1_000_000) for _ in range(random.randrange(30))]
            input_path.write_text(" \n" + json.dumps(values) + "\t")
            expected = [value * value for value in values]
            for backend in ["VM", "native"]:
                squared_path.write_bytes(b"old bytes must disappear")
                total_path.write_bytes(b"old bytes must disappear")
                succeeded(run(dataset, backend, grants), backend, f"out.count={len(values)}\n".encode())
                assert squared_path.read_bytes() == canonical(expected).encode()
                assert total_path.read_bytes() == str(sum(expected)).encode()
        print("PASS: 60 Python/VM/native file datasets parse, square, aggregate, format and write exact JSON")

        boundaries = [
            ("JSON_PARSE", "Text", '"' + "x" * (LIMIT - 2) + '"', LIMIT - 2, None),
            ("JSON_PARSE", "Text", '"' + "x" * (LIMIT - 1) + '"', None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_FORMAT", "Text", "x" * (LIMIT - 2), LIMIT, None),
            ("JSON_FORMAT", "Text", "x" * (LIMIT - 1), None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_FORMAT", "Text", "\x7f" * 174762, 1048574, None),
            ("JSON_FORMAT", "Text", "\x7f" * 174763, None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_PARSE", "Int", "x" * (LIMIT + 1), None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_FORMAT", "Bool", "x" * (LIMIT + 1), None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_PARSE", "List<Int>", "[" + ",".join(["0"] * 65536) + "]", 131073, None),
            ("JSON_PARSE", "List<Int>", "[" + ",".join(["0"] * 65537) + "]", None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_PARSE", "List<Text>", '["' + "x" * (LIMIT - 4) + '"]', LIMIT, None),
            ("JSON_FORMAT", "List<Text>", '["' + "\x7f" * 200000 + '"]', None, "AIL-RUNTIME-JSON-LIMIT"),
            ("JSON_PARSE", "List<Text>", '["' + "\x7f" * 200000 + '",1]', None, "AIL-RUNTIME-JSON:"),
        ]
        for idx, (opcode, type_name, raw, count, category) in enumerate(boundaries):
            program = compile_program(root, f"boundary_{idx}", [
                op("SET_FIELD", key="input.raw", value=raw, text="runtime input fixture"),
                op(opcode, key="data.value", source="input.raw", type=type_name),
                op("TEXT_LENGTH", key="out.count", source="data.value"), op("OUTPUT_FIELD", key="out.count"),
            ])
            for backend in ["VM", "native"]:
                result = run(program, backend)
                if category:
                    failed(result, backend, category)
                else:
                    succeeded(result, backend, f"out.count={count}\n".encode())
        print(f"PASS: {len(boundaries)} exact/overflow input, result, canonical expansion and item-count boundaries")

        large_list = '["' + "x" * (LIMIT - 4) + '"]'
        for opcode, type_name, raw in [("JSON_PARSE", "List<Text>", large_list), ("JSON_FORMAT", "List<Text>", large_list), ("JSON_FORMAT", "Text", "x" * (LIMIT - 2))]:
            for extra in [False, True]:
                instruction = op(opcode, key="data.value", source="input.raw", type=type_name)
                steps = [
                    op("SET_FIELD", key="input.raw", value=raw, text="runtime fixture"),
                    op("SET_FIELD", key="input.list", value=large_list, text="runtime fixture"),
                    op("SET_FIELD", key="input.integer", value="-0", text="runtime fixture"),
                    op("SET_FIELD", key="input.boolean", value="true", text="runtime fixture"),
                    op("EVAL_INT", key="out.count", expression="i:0"), op("LABEL", name="again"),
                    instruction, op("EVAL_INT", key="out.count", expression="f:out.count i:1 +"),
                    op("BRANCH_INT", left="f:out.count", right="i:64", comparison="<", label="again"),
                ]
                if extra:
                    steps.append(instruction)
                steps.extend([
                    op("JSON_PARSE", key="data.integer", source="input.integer", type="Int"),
                    op("JSON_PARSE", key="data.boolean", source="input.boolean", type="Bool"),
                ])
                # Filling one arena must leave the other available. Formatting
                # a list cannot charge a hidden list-producing temporary.
                steps.append(op("JSON_FORMAT", key="data.other", source="data.value", type="List<Text>") if opcode == "JSON_PARSE" else op("JSON_PARSE", key="data.other", source="input.list", type="List<Text>"))
                steps.append(op("OUTPUT_FIELD", key="out.count"))
                program = compile_program(root, "budget", steps)
                for backend in ["VM", "native"]:
                    result = run(program, backend)
                    if extra:
                        failed(result, backend, "AIL-RUNTIME-JSON-LIMIT")
                    else:
                        succeeded(result, backend, b"out.count=64\n")
        print("PASS: 64/65-result shared arena bounds and absence of intermediate text/list charges")


if __name__ == "__main__":
    main()
