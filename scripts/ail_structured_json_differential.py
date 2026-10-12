#!/usr/bin/env python3
"""Independent lossless JSON and RFC 6901 oracle for AIL VM and native ELF.

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
TYPES = ["Json", "Int", "Bool", "Text", "List<Int>", "List<Text>"]


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


def pointer(value, path):
    if len(path.encode()) > LIMIT:
        raise OracleError("JSON-LIMIT")
    checked_string(path)
    if path == "":
        return value
    if not path.startswith("/"):
        raise OracleError("JSON-PATH")
    tokens = path[1:].split("/")
    if any(re.search(r"~(?![01])", token) for token in tokens):
        raise OracleError("JSON-PATH")
    # Decode once, after validating every segment, including unvisited ones.
    tokens = [token.replace("~1", "/").replace("~0", "~") for token in tokens]
    for token in tokens:
        if isinstance(value, Pairs):
            matches = [child for key, child in value if key == token]
            if len(matches) != 1:
                raise OracleError("JSON-PATH")
            value = matches[0]
        elif type(value) is list:
            if not re.fullmatch(r"0|[1-9][0-9]*", token) or len(token) > 20:
                raise OracleError("JSON-PATH")
            index = int(token)
            if index >= len(value):
                raise OracleError("JSON-PATH")
            value = value[index]
        else:
            raise OracleError("JSON-PATH")
    return value


def typed(value, type_name):
    if type_name == "Json":
        return canonical(value)
    if type_name == "Int":
        if not isinstance(value, Number) or not re.fullmatch(r"-?(0|[1-9][0-9]*)", value):
            raise OracleError("JSON")
        number = int(value)
        if not -(2**63) <= number < 2**63:
            raise OracleError("JSON")
        return str(number)
    if type_name == "Bool" and type(value) is bool:
        return "true" if value else "false"
    if type_name == "Text" and type(value) is str:
        return value
    if type_name.startswith("List") and type(value) is list:
        inner = type_name[5:-1]
        items = [typed(item, inner) for item in value]
        return "[" + ",".join(quoted(item) if inner == "Text" else item for item in items) + "]"
    raise OracleError("JSON")


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=30)


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions):
    source = root / f"{name}.json"
    source.write_text(json.dumps({
        "kind": "AIL-Bytecode", "package": "structured-oracle", "version": "1",
        "profile": "Application", "capability_grants": [], "target_support": {},
        "external_bindings": [], "failures": [],
        "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}],
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


def main():
    random.seed(953_721)
    characters = ["a", " ", "é", "🙂", "\U0010ffff", "\u0301", "\0", "\x01", "\b", "\f", "\n", "\r", "\t", "\x7f", "\x85", '"', "\\", "=", "[", "]", ",", "/", "~", "to data.document"]

    def text():
        return "".join(random.choices(characters, k=random.randrange(12)))

    def value(depth=0):
        kind = random.randrange(6 if depth < 4 else 4)
        if kind == 0:
            return None
        if kind == 1:
            return random.choice([True, False])
        if kind == 2:
            return Number(random.choice(["-0", "-0.000e+0", "1E-999", "1.2500e+0002", "9999999999999999999999999999999", str(random.randrange(-100_000, 100_000))]))
        if kind == 3:
            return text()
        if kind == 4:
            return [value(depth + 1) for _ in range(random.randrange(5))]
        return Pairs((text(), value(depth + 1)) for _ in range(random.randrange(5)))

    with tempfile.TemporaryDirectory(prefix="ail-structured-oracle-") as temporary:
        root = Path(temporary)
        roundtrip = compile_program(root, "roundtrip", [
            op("JSON_PARSE", key="data.document", source="input.raw", type="Json"),
            op("COPY_FIELD", key="out.copy", source="data.document", text="retain immutable document"),
            op("JSON_FORMAT", key="data.document", source="data.document", type="Json"),
            op("JSON_PARSE", key="out.value", source="data.document", type="Json"),
            op("OUTPUT_FIELD", key="out.copy"), op("OUTPUT_FIELD", key="out.value"),
        ])
        samples = [value() for _ in range(250)] + [
            Pairs([("a", Number("1")), ("a", Number("2"))]),
            Number("1e" + "9" * 1000), Number("9" * 1000),
        ]
        for idx, item in enumerate(samples):
            raw = " \t\r\n" + emit(item, idx % 2 == 0) + "\n "
            expected = canonical(decode(raw))
            for backend in ["VM", "native"]:
                succeeded(run(roundtrip, backend, ["input.raw=" + raw]), backend,
                          f"out.copy={expected}\nout.value={expected}\n".encode())
        print(f"PASS: {len(samples)} lossless recursive JSON, number tokens, canonical Unicode and alias round trips", flush=True)

        programs = {}
        for type_name in TYPES:
            programs[type_name] = compile_program(root, "get_" + type_name.replace("<", "_").replace(">", ""), [
                op("JSON_PARSE", key="input.path", source="path.raw", type="Text"),
                op("JSON_GET", key="out.value", source="input.raw", path="input.path", type=type_name),
                op("OUTPUT_FIELD", key="out.value"),
            ])
        lookups = []
        for _ in range(100):
            entries = Pairs([("n", Number(str(random.randrange(-(2**63), 2**63)))), ("s", text()),
                             ("b", random.choice([True, False])), ("j", value()),
                             ("is", [Number(str(random.randrange(-1000, 1000))) for _ in range(4)]),
                             ("ss", [text() for _ in range(4)])])
            root_value = Pairs([("records/~", [entries])])
            for key, type_name in [("n", "Int"), ("s", "Text"), ("b", "Bool"), ("j", "Json"), ("is", "List<Int>"), ("ss", "List<Text>")]:
                lookups.append((emit(root_value, True), "/records~1~0/0/" + key, type_name))
        tricky = '{"":7,"~1":8,"a/b":{"~k":["\\u0000",true,-0]},"\\u0000":9,"dup":0,"\\u0064up":1,"é":2,"e\\u0301":3,"00":4,"~":5}'
        lookups += [(tricky, p, t) for p, t in [
            ("", "Json"), ("/", "Int"), ("/~01", "Int"), ("/a~1b/~0k/0", "Text"),
            ("/a~1b/~0k/1", "Bool"), ("/a~1b/~0k/2", "Json"), ("/\0", "Int"),
            ("/dup", "Json"), ("/é", "Int"), ("/e\u0301", "Int"), ("/00", "Int"), ("/~0", "Int"),
            ("/missing/later~2", "Json"), ("#", "Json"), ("/a~1b/~0k/00", "Json"),
            ("/a~1b/~0k/+0", "Json"), ("/a~1b/~0k/-", "Json"), ("/a~1b/~0k/9", "Json"),
        ]]
        lookups += [(raw, "", t) for raw in ["1.0", "1e0", "9223372036854775808", "-9223372036854775809", "null", "true", "[]", "{}"] for t in TYPES]
        for raw, path, type_name in lookups:
            try:
                expected = typed(pointer(decode(raw), path), type_name)
                error = None
            except OracleError as problem:
                error = str(problem)
            for backend in ["VM", "native"]:
                result = run(programs[type_name], backend, ["input.raw=" + raw, "path.raw=" + json.dumps(path)])
                if error:
                    failed(result, backend, error)
                else:
                    succeeded(result, backend, f"out.value={expected}\n".encode())
        print(f"PASS: {len(lookups)} independent RFC 6901 lookups, duplicate ambiguity and six typed results", flush=True)

        count_program = compile_program(root, "count", [op("JSON_COUNT", key="out.value", source="input.raw"), op("OUTPUT_FIELD", key="out.value")])
        for item in samples[:100]:
            for backend in ["VM", "native"]:
                result = run(count_program, backend, ["input.raw=" + emit(item, True)])
                if isinstance(item, list):
                    succeeded(result, backend, f"out.value={len(item)}\n".encode())
                else:
                    failed(result, backend)
        print("PASS: 100 array/object counts, repeated members and scalar errors", flush=True)

        malformed = ["", "nullx", "true false", "[1,]", '{"a":1,}', '{1:true}', '{"a" 1}', "[[]", "+1", "01", "-01", "1.", ".1", "1e", "1e+", "NaN", "Infinity", "\x85null", '"\\ud800"', '"\\udc00"', '"\\ud800\\u0000"', '"raw\nline"', '{"good":1,"bad":"\\ud800"}', '{"good":1}junk', '"\\u+041"', '"\\u004"', '"\\x41"']
        formatting = compile_program(root, "format", [op("JSON_FORMAT", key="out.value", source="input.raw", type="Json"), op("OUTPUT_FIELD", key="out.value")])
        for raw in malformed:
            for program in [roundtrip, formatting, count_program, programs["Json"]]:
                for backend in ["VM", "native"]:
                    failed(run(program, backend, ["input.raw=" + raw, 'path.raw="/good"']), backend)
        for program in [roundtrip, formatting, count_program]:
            failed(run(program, "native", [b"input.raw=\xff"]), "native")
        print(f"PASS: {len(malformed)} malformed grammars across parse/format/get/count and native invalid UTF-8", flush=True)

        limits = [("[" * depth + "0" + "]" * depth, None if depth <= 64 else "JSON-LIMIT") for depth in [63, 64, 65]]
        limits += [("[" + ",".join("0" for _ in range(nodes - 1)) + "]", None if nodes <= 65_536 else "JSON-LIMIT") for nodes in [65_535, 65_536, 65_537]]
        limits += [('"' + "x" * (LIMIT - 2) + '"', None), ('"' + "x" * (LIMIT - 1) + '"', "JSON-LIMIT"),
                   ('{"a":0,"z":"' + "\x7f" * 180_000 + '"}', "JSON-LIMIT"),
                   ('{"a":0,"z":"' + "\x7f" * 180_000 + '","bad":}', "JSON")]
        for idx, (raw, error) in enumerate(limits):
            for opcode in ["JSON_PARSE", "JSON_FORMAT", "JSON_GET"]:
                instruction = op(opcode, key="out.value", source="input.raw", type="Json", **({"path": "input.path"} if opcode == "JSON_GET" else {}))
                program = compile_program(root, f"limit_{idx}_{opcode}", [
                    op("SET_FIELD", key="input.raw", value=raw, text="bounded JSON fixture"),
                    op("ASSIGN_LITERAL", key="input.path", type="Text", value='""'), instruction,
                    op("TEXT_LENGTH", key="out.count", source="out.value"), op("OUTPUT_FIELD", key="out.count"),
                ])
                for backend in ["VM", "native"]:
                    result = run(program, backend)
                    if error:
                        failed(result, backend, error)
                    else:
                        succeeded(result, backend, f"out.count={len(canonical(decode(raw)))}\n".encode())
        print("PASS: exact and exceeded depth/node/byte limits, full-document validation and expansion priority", flush=True)

        for extra in [False, True]:
            large = '"' + "x" * (LIMIT - 2) + '"'
            steps = [op("SET_FIELD", key="input.raw", value=large, text="budget fixture"), op("ASSIGN_LITERAL", key="input.path", type="Text", value='""')]
            steps += [op("JSON_GET", key="out.value", source="input.raw", path="input.path", type="Json") if index % 2 else op("JSON_PARSE", key="out.value", source="input.raw", type="Json") for index in range(64)]
            steps += [op("ASSIGN_LITERAL", key="small.raw", type="Json", value='{"n":8}'),
                      op("JSON_COUNT", key="out.count", source="small.raw"),
                      op("JSON_FORMAT", key="out.text", source="out.value", type="Json")]
            if extra:
                steps += [op("ASSIGN_LITERAL", key="small.list", type="Text", value='"[]"'), op("JSON_PARSE", key="out.list", source="small.list", type="List<Int>")]
            steps += [op("OUTPUT_FIELD", key="out.count")]
            program = compile_program(root, "budget", steps)
            for backend in ["VM", "native"]:
                result = run(program, backend)
                if extra:
                    failed(result, backend, "JSON-LIMIT")
                else:
                    succeeded(result, backend, b"out.count=1\n")
        print("PASS: shared 64 MiB Json/list result arena, free literals/count and independent text formatting", flush=True)

        dataset = compile_program(root, "dataset", [
            op("FILE_READ_TEXT", key="data.raw", resource="input"),
            op("JSON_PARSE", key="data.document", source="data.raw", type="Json"),
            op("ASSIGN_LITERAL", key="data.path", type="Text", value='"/records"'),
            op("JSON_GET", key="data.records", source="data.document", path="data.path", type="Json"),
            op("JSON_COUNT", key="out.count", source="data.records"),
            op("EVAL_INT", key="data.index", expression="i:0"), op("EVAL_INT", key="data.total", expression="i:0"),
            op("ASSIGN_LITERAL", key="data.slash", type="Text", value='"/"'),
            op("ASSIGN_LITERAL", key="data.value_path", type="Text", value='"/value"'),
            op("ASSIGN_LITERAL", key="data.name_path", type="Text", value='"/name"'),
            op("LABEL", name="again"), op("BRANCH_INT", left="f:data.index", right="f:out.count", comparison=">=", label="done"),
            op("JSON_FORMAT", key="data.digits", source="data.index", type="Int"),
            op("TEXT_JOIN", key="data.path", left="data.slash", right="data.digits"),
            op("JSON_GET", key="data.record", source="data.records", path="data.path", type="Json"),
            op("JSON_GET", key="data.value", source="data.record", path="data.value_path", type="Int"),
            op("JSON_GET", key="data.name", source="data.record", path="data.name_path", type="Text"),
            op("FILE_WRITE_TEXT", source="data.name", resource="name"),
            op("EVAL_INT", key="data.total", expression="f:data.total f:data.value f:data.value * +"),
            op("EVAL_INT", key="data.index", expression="f:data.index i:1 +"), op("JUMP", label="again"), op("LABEL", name="done"),
            op("JSON_FORMAT", key="data.report", source="data.total", type="Int"),
            op("FILE_WRITE_TEXT", source="data.report", resource="report"), op("OUTPUT_FIELD", key="out.count"),
        ])
        input_file, report_file, name_file = [root / name for name in ["input.json", "report.json", "name.txt"]]
        grants = [f"--read-file=input={input_file}", f"--write-file=report={report_file}", f"--write-file=name={name_file}"]
        for idx in range(40):
            records = [{"value": random.randrange(-1000, 1000), "name": text()} for _ in range(random.randrange(12))]
            input_file.write_text(json.dumps({"records": records}, ensure_ascii=idx % 2 == 0))
            expected = str(sum(item["value"] ** 2 for item in records)).encode()
            for backend in ["VM", "native"]:
                name_file.write_bytes(b"untouched")
                succeeded(run(dataset, backend, grants), backend, f"out.count={len(records)}\n".encode())
                assert report_file.read_bytes() == expected
                assert name_file.read_bytes() == (records[-1]["name"].encode() if records else b"untouched")
        print("PASS: 40 nested file datasets, dynamic pointers, integer loops and exact Unicode/NUL output files", flush=True)


if __name__ == "__main__":
    main()
