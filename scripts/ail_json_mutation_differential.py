#!/usr/bin/env python3
"""Independent lossless JSON mutation oracle for AIL VM and native ELF.

Run cargo build first. Uses Python JSON's number-token and object-pair hooks,
then separately implements AIL canonical strings and immutable record mutation.
No production parser, emitter, third-party dependency or benchmark is used.
"""
import copy
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
        "kind": "AIL-Bytecode", "package": "mutation-oracle", "version": "1",
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


def path_tokens(path):
    if len(path.encode()) > LIMIT:
        raise OracleError("JSON-LIMIT")
    checked_string(path)
    if path == "":
        return []
    if not path.startswith("/"):
        raise OracleError("JSON-PATH")
    tokens = path[1:].split("/")
    if any(re.search(r"~(?![01])", token) for token in tokens):
        raise OracleError("JSON-PATH")
    return [token.replace("~1", "/").replace("~0", "~") for token in tokens]


def array_index(token, length):
    if not re.fullmatch(r"0|[1-9][0-9]*", token) or len(token) > 20:
        raise OracleError("JSON-PATH")
    index = int(token)
    if index >= length:
        raise OracleError("JSON-PATH")
    return index


def replacement(raw, type_name):
    if len(raw.encode()) > LIMIT:
        raise OracleError("JSON-LIMIT")
    if type_name == "Json":
        return decode(raw)
    if type_name == "Text":
        checked_string(raw)
        result = raw
    elif type_name == "Int":
        if not re.fullmatch(r"[+-]?[0-9]+", raw):
            raise OracleError("JSON")
        number = int(raw)
        if not -(2**63) <= number < 2**63:
            raise OracleError("JSON")
        result = Number(str(number))
    elif type_name == "Bool":
        if raw not in ("true", "false"):
            raise OracleError("JSON")
        result = raw == "true"
    else:
        result = decode(typed(decode(raw), type_name))
    if len(canonical(result).encode()) > LIMIT:
        raise OracleError("JSON-LIMIT")
    return result


def mutate(raw, path, type_name=None, operand=None):
    document = decode(raw)
    tokens = path_tokens(path)
    if not tokens:
        if type_name is None:
            raise OracleError("JSON-PATH")
        return canonical(replacement(operand, type_name))
    parent = document
    for token in tokens[:-1]:
        if isinstance(parent, Pairs):
            matches = [item for key, item in parent if key == token]
            if len(matches) != 1:
                raise OracleError("JSON-PATH")
            parent = matches[0]
        elif type(parent) is list:
            parent = parent[array_index(token, len(parent))]
        else:
            raise OracleError("JSON-PATH")
    last = tokens[-1]
    if isinstance(parent, Pairs):
        matches = [index for index, (key, _) in enumerate(parent) if key == last]
        if len(matches) > 1 or (not matches and type_name is None):
            raise OracleError("JSON-PATH")
        index = matches[0] if matches else len(parent)
    elif type(parent) is list:
        index = len(parent) if last == "-" and type_name is not None else array_index(last, len(parent))
    else:
        raise OracleError("JSON-PATH")
    # Target validity above is deliberately resolved before decoding operand.
    if type_name is None:
        parent.pop(index)
    else:
        value = replacement(operand, type_name)
        pair = (last, value) if isinstance(parent, Pairs) else value
        if index == len(parent):
            parent.append(pair)
        else:
            parent[index] = pair
    return canonical(decode(canonical(document)))


def main():
    random.seed(419_873)
    alphabet = ["a", " ", "é", "🙂", "\U0010ffff", "\u0301", "\0", "\x01", "\b", "\f", "\n", "\r", "\t", "\x7f", "\x85", '"', "\\", "=", "[", "]", ",", "/", "~", "to data.item"]

    def text():
        return "".join(random.choices(alphabet, k=random.randrange(10)))

    def value(depth=0):
        kind = random.randrange(6 if depth < 3 else 4)
        if kind == 0:
            return None
        if kind == 1:
            return random.choice([True, False])
        if kind == 2:
            return Number(random.choice(["-0", "-0.000e+0", "1E-999", "1.2500e+0002", "99999999999999999999999999999", str(random.randrange(-100_000, 100_000))]))
        if kind == 3:
            return text()
        if kind == 4:
            return [value(depth + 1) for _ in range(random.randrange(4))]
        return Pairs((text(), value(depth + 1)) for _ in range(random.randrange(4)))

    def operand(type_name):
        if type_name == "Int":
            return random.choice(["+00042", "-000", str(-(2**63)), str(2**63 - 1), str(random.randrange(-1_000_000, 1_000_000))])
        if type_name == "Bool":
            return random.choice(["true", "false"])
        if type_name == "Text":
            return text()
        if type_name == "List<Int>":
            return "[" + ",".join(str(random.randrange(-1000, 1000)) for _ in range(random.randrange(8))) + "]"
        if type_name == "List<Text>":
            return json.dumps([text() for _ in range(random.randrange(8))])
        return emit(value(), random.choice([True, False]))

    with tempfile.TemporaryDirectory(prefix="ail-mutation-oracle-") as temporary:
        root = Path(temporary)
        programs = {}
        for type_name in TYPES + [None]:
            mutation = op("JSON_REMOVE", key="out.document", source="input.document", path="input.path") if type_name is None else op("JSON_SET", key="out.document", source="input.document", path="input.path", value="input.value", type=type_name)
            programs[type_name] = compile_program(root, "mutate_" + str(type_name).replace("<", "_").replace(">", ""), [
                op("JSON_PARSE", key="input.path", source="path.raw", type="Text"),
                op("JSON_PARSE", key="input.value", source="value.raw", type="Text"),
                mutation, op("OUTPUT_FIELD", key="out.document"),
            ])
        cases = []
        for type_name in TYPES:
            for _ in range(70):
                document = Pairs([("before", Number("-0")), ("records/~", [value(), value(), value()]), ("after", Number("1e+999"))])
                path = random.choice(["", "/before", "/new", "/records~1~0/0", "/records~1~0/2", "/records~1~0/-"])
                cases.append((emit(document, True), path, type_name, operand(type_name)))
        for _ in range(100):
            document = Pairs([("before", Number("-0")), ("records/~", [value(), value(), value()]), ("after", Number("1e+999"))])
            cases.append((emit(document, True), random.choice(["/before", "/after", "/records~1~0/0", "/records~1~0/2"]), None, ""))
        tricky = '{"":0,"a/b":{"~key":[0,1,2]},"dup":0,"\\u0064up":1,"~1":5,"\\u0000":6,"é":7,"e\\u0301":8}'
        for path in ["", "/", "/new", "/dup", "/~01", "/\0", "/é", "/e\u0301", "/a~1b/~0key/0", "/a~1b/~0key/-", "/a~1b/~0key/3", "/a~1b/~0key/00", "/a~1b/~0key/+1", "/a~1b/~0key/-1", "/missing/x", "/missing/later~3", "#/dup", "/~", "/a~1b/~0key/-/x"]:
            cases.extend([(tricky, path, "Json", "{\"n\":1e+009}"), (tricky, path, None, "")])
        for raw, path, type_name, raw_value in cases:
            try:
                expected = mutate(raw, path, type_name, raw_value)
                error = None
            except OracleError as problem:
                error = str(problem)
            arguments = ["input.document=" + raw, "path.raw=" + json.dumps(path), "value.raw=" + json.dumps(raw_value)]
            for backend in ["VM", "native"]:
                result = run(programs[type_name], backend, arguments)
                if error:
                    failed(result, backend, error)
                else:
                    succeeded(result, backend, f"out.document={expected}\n".encode())
        print(f"PASS: {len(cases)} independent immutable mutations, six value types, pair/number order, append/remove and escaped paths", flush=True)

        # Raw operands bypass the preparatory Text decoders to test mutation's
        # own error precedence, missing fields and native invalid UTF-8.
        bad = compile_program(root, "priority", [op("JSON_SET", key="out.document", source="input.document", path="input.path", value="input.value", type="Json"), op("OUTPUT_FIELD", key="out.document")])
        priority = [("bad", "/~3", "bad", "JSON"), ("{}", "/absent/child", "bad", "JSON-PATH"), ("{}", "/~3", "bad", "JSON-PATH"), ("[]", "/0", "bad", "JSON-PATH"), ('{"a":0,"\\u0061":1}', "/a", "bad", "JSON-PATH"), ("{}", "/new", "bad", "JSON"), ("null", "", "bad", "JSON"), ("null", "/x", "bad", "JSON-PATH")]
        for raw, path, raw_value, error in priority:
            for backend in ["VM", "native"]:
                failed(run(bad, backend, ["input.document=" + raw, "input.path=" + path, "input.value=" + raw_value]), backend, error)
        for arguments, error in [([], "JSON"), (["input.document={}"], "JSON"), (["input.document={}", "input.path=/valid"], "JSON"), (["input.document={}", "input.path=/absent/child"], "JSON-PATH")]:
            for backend in ["VM", "native"]:
                failed(run(bad, backend, arguments), backend, error)
        for arguments, error in [([b"input.document=\xff", "input.path=/~3", "input.value=bad"], "JSON"), (["input.document={}", b"input.path=/\xff", "input.value=bad"], "JSON"), (["input.document={}", "input.path=/valid", b"input.value=\xff"], "JSON"), (["input.document={}", "input.path=/absent/child", b"input.value=\xff"], "JSON-PATH")]:
            failed(run(bad, "native", arguments), "native", error)
        print("PASS: source/path/target/value error order, missing operands and native invalid UTF-8", flush=True)

        alias_cases = 0
        for destination in ["input.document", "input.path", "input.value"]:
            for type_name in TYPES:
                mutation = op("JSON_SET", key=destination, source="input.document", path="input.path", value="input.value", type=type_name)
                program = compile_program(root, "alias", [
                    op("JSON_PARSE", key="input.value", source="value.raw", type="Text"),
                    op("COPY_FIELD", key="out.copy", source=destination, text="immutable alias"), mutation,
                    op("COPY_FIELD", key="out.result", source=destination, text="mutation result"),
                    op("OUTPUT_FIELD", key="out.copy"), op("OUTPUT_FIELD", key="out.result"),
                ])
                raw_value = operand(type_name)
                raw = '{"x":0}'
                path = "/x"
                previous = raw if destination == "input.document" else path if destination == "input.path" else raw_value
                expected = mutate(raw, path, type_name, raw_value)
                for backend in ["VM", "native"]:
                    succeeded(run(program, backend, ["input.document=" + raw, "input.path=" + path, "value.raw=" + json.dumps(raw_value)]), backend, f"out.copy={previous}\nout.result={expected}\n".encode())
                alias_cases += 1
        same_source = compile_program(root, "source_is_value", [op("JSON_SET", key="input.document", source="input.document", path="input.path", value="input.document", type="Json"), op("OUTPUT_FIELD", key="input.document")])
        for backend in ["VM", "native"]:
            succeeded(run(same_source, backend, ["input.document={\"a\":1e0}", "input.path=/child"]), backend, b'input.document={"a":1e0,"child":{"a":1e0}}\n')
        print(f"PASS: {alias_cases} source/path/value destination aliases and source-as-value snapshot", flush=True)

        full_nodes = "[" + ",".join("0" for _ in range(65_535)) + "]"
        nested = lambda depth: "[" * depth + "0" + "]" * depth
        exact = '"' + "x" * (LIMIT - 2) + '"'
        boundaries = [("null", "", nested(64), None), ("[]", "/-", nested(63), None), ("[]", "/-", nested(64), "JSON-LIMIT"), (nested(64), "/0", nested(63), None), (nested(65), "", "0", "JSON-LIMIT"), ("null", "", nested(65), "JSON-LIMIT"), (full_nodes, "/0", "0", None), (full_nodes, "/-", "0", "JSON-LIMIT"), ("null", "", exact, None), ("[]", "/-", exact, "JSON-LIMIT"), (exact, "", "0", None), ('"' + "x" * (LIMIT - 1) + '"', "", "0", "JSON-LIMIT"), ("{}", "/bad/child", "x" * (LIMIT + 1), "JSON-PATH"), ("{}", "/new", "x" * (LIMIT + 1), "JSON-LIMIT"), ("{}", "x" * (LIMIT + 1), "bad", "JSON-LIMIT")]
        for index, (raw, path, raw_value, error) in enumerate(boundaries):
            program = compile_program(root, f"boundary{index}", [
                op("SET_FIELD", key="input.document", value=raw, text="source fixture"), op("SET_FIELD", key="input.path", value=path, text="path fixture"), op("SET_FIELD", key="input.value", value=raw_value, text="value fixture"),
                op("JSON_SET", key="out.document", source="input.document", path="input.path", value="input.value", type="Json"),
                op("TEXT_LENGTH", key="out.count", source="out.document"), op("OUTPUT_FIELD", key="out.count"),
            ])
            for backend in ["VM", "native"]:
                result = run(program, backend)
                if error:
                    failed(result, backend, error)
                else:
                    succeeded(result, backend, f"out.count={len(mutate(raw, path, 'Json', raw_value))}\n".encode())
        print(f"PASS: {len(boundaries)} source/replacement/final depth, node and byte limits; replaced subtrees excluded", flush=True)

        for extra in [False, True]:
            steps = [op("ASSIGN_LITERAL", key="input.document", type="Json", value="null"), op("ASSIGN_LITERAL", key="input.path", type="Text", value='""'), op("SET_FIELD", key="input.value", value="x" * (LIMIT - 2), text="text value fixture")]
            steps += [op("JSON_SET", key="out.document", source="input.document", path="input.path", value="input.value", type="Text") for _ in range(64)]
            steps += [op("JSON_FORMAT", key="out.text", source="input.value", type="Text"), op("ASSIGN_LITERAL", key="small.json", type="Json", value="{}")]
            if extra:
                steps += [op("JSON_PARSE", key="extra.json", source="small.json", type="Json")]
            steps += [op("TEXT_LENGTH", key="out.count", source="out.document"), op("OUTPUT_FIELD", key="out.count")]
            program = compile_program(root, "budget", steps)
            for backend in ["VM", "native"]:
                result = run(program, backend)
                if extra:
                    failed(result, backend, "JSON-LIMIT")
                else:
                    succeeded(result, backend, b"out.count=1048576\n")
        print("PASS: only final Json bytes charged, shared 64 MiB arena and independent text arena", flush=True)

        dataset = compile_program(root, "dataset", [
            op("FILE_READ_TEXT", key="data.raw", resource="input"), op("JSON_PARSE", key="data.document", source="data.raw", type="Json"),
            op("ASSIGN_LITERAL", key="data.path", type="Text", value='"/records"'), op("JSON_GET", key="data.records", source="data.document", path="data.path", type="Json"),
            op("JSON_COUNT", key="out.count", source="data.records"), op("ASSIGN_LITERAL", key="out.document", type="Json", value="[]"),
            op("EVAL_INT", key="data.index", expression="i:0"),
            op("ASSIGN_LITERAL", key="data.slash", type="Text", value='"/"'), op("ASSIGN_LITERAL", key="data.value_path", type="Text", value='"/value"'),
            op("ASSIGN_LITERAL", key="data.remove_path", type="Text", value='"/private"'), op("ASSIGN_LITERAL", key="data.append_path", type="Text", value='"/-"'),
            op("LABEL", name="again"), op("BRANCH_INT", left="f:data.index", right="f:out.count", comparison=">=", label="done"),
            op("JSON_FORMAT", key="data.digits", source="data.index", type="Int"), op("TEXT_JOIN", key="data.path", left="data.slash", right="data.digits"),
            op("JSON_GET", key="data.record", source="data.records", path="data.path", type="Json"),
            op("JSON_REMOVE", key="data.record", source="data.record", path="data.remove_path"),
            op("JSON_GET", key="data.value", source="data.record", path="data.value_path", type="Int"), op("EVAL_INT", key="data.value", expression="f:data.value f:data.value *"),
            op("JSON_SET", key="data.record", source="data.record", path="data.value_path", value="data.value", type="Int"),
            op("JSON_SET", key="out.document", source="out.document", path="data.append_path", value="data.record", type="Json"),
            op("EVAL_INT", key="data.index", expression="f:data.index i:1 +"), op("JUMP", label="again"), op("LABEL", name="done"),
            op("JSON_FORMAT", key="data.report", source="out.document", type="Json"), op("FILE_WRITE_TEXT", source="data.report", resource="report"), op("OUTPUT_FIELD", key="out.count"),
        ])
        input_file, report_file = root / "input.json", root / "report.json"
        grants = [f"--read-file=input={input_file}", f"--write-file=report={report_file}"]
        for index in range(50):
            records = [Pairs([("value", Number(str(random.randrange(-1000, 1000)))), ("private", text()), ("name", text()), ("unchanged", value())]) for _ in range(random.randrange(10))]
            input_file.write_text(emit(Pairs([("records", records)]), index % 2 == 0))
            expected = copy.deepcopy(records)
            for record in expected:
                record.pop(1)
                record[0] = ("value", Number(str(int(record[0][1]) ** 2)))
            for backend in ["VM", "native"]:
                succeeded(run(dataset, backend, grants), backend, f"out.count={len(records)}\n".encode())
                assert report_file.read_bytes() == canonical(expected).encode()
        print("PASS: 50 nested file reports with field removal, computed replacements, array append and preserved arbitrary values", flush=True)


if __name__ == "__main__":
    main()
