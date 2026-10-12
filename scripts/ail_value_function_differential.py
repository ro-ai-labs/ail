#!/usr/bin/env python3
"""Independent typed value-function oracle for AIL VM and native ELF.

Run cargo build first. Uses Python JSON's number-token and object-pair hooks,
then separately checks return normalization and reusable record transformations.
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


def main():
    random.seed(724_913)
    alphabet = ["a", " ", "é", "🙂", "\U0010ffff", "\u0301", "\0", "\x01", "\b", "\f", "\n", "\r", "\t", "\x7f", "\x85", '"', "\\", "=", "[", "]", ",", "/", "~", "local.value"]

    def text():
        return "".join(random.choices(alphabet, k=random.randrange(15)))

    def nested(depth=0):
        kind = random.randrange(6 if depth < 3 else 4)
        if kind == 0:
            return None
        if kind == 1:
            return random.choice([True, False])
        if kind == 2:
            return Number(random.choice(["-0", "1e999", "-0.00E+09", "999999999999999999999999999", str(random.randrange(-1000, 1000))]))
        if kind == 3:
            return text()
        if kind == 4:
            return [nested(depth + 1) for _ in range(random.randrange(4))]
        return Pairs((text(), nested(depth + 1)) for _ in range(random.randrange(4)))

    with tempfile.TemporaryDirectory(prefix="ail-value-function-oracle-") as temporary:
        root = Path(temporary)
        cases = 0
        for type_name in TYPES:
            identity = routine("identity", "value:" + type_name, "", type_name, [returns(type_name, "f:local.value")])
            program = compile_program(root, "identity", [
                op("JSON_PARSE", key="input.value", source="value.raw", type="Text"),
                call("identity", "value=f:input.value", "out.first"),
                call("identity", "value=f:input.value", "input.value"),
                call("identity", "value=f:input.value", "out.second"),
                op("OUTPUT_FIELD", key="out.first"), op("OUTPUT_FIELD", key="out.second"),
            ], [identity])
            for _ in range(60):
                if type_name == "Int":
                    number = random.choice([-(2**63), 2**63 - 1, 0, random.randrange(-100000, 100000)])
                    raw = ("+000" if number >= 0 else "-000") + str(abs(number))
                    expected = str(number)
                elif type_name == "Bool":
                    raw = expected = random.choice(["true", "false"])
                elif type_name == "Text":
                    raw = expected = text()
                elif type_name == "List<Int>":
                    raw = "[ " + ", ".join(random.choice(["-0", str(random.randrange(-1000, 1000))]) for _ in range(random.randrange(7))) + " ]"
                    expected = typed(decode(raw), type_name)
                elif type_name == "List<Text>":
                    raw = json.dumps([text() for _ in range(random.randrange(7))])
                    expected = typed(decode(raw), type_name)
                else:
                    raw = emit(nested(), random.choice([True, False]))
                    expected = canonical(decode(raw))
                for backend in ["VM", "native"]:
                    succeeded(run(program, backend, ["value.raw=" + json.dumps(raw), "local.value=forged"]), backend,
                              f"out.first={expected}\nout.second={expected}\n".encode())
                cases += 1
        print(f"PASS: {cases} six-type returns, normalization, isolated frames and values surviving repeated calls", flush=True)

        # These helpers exist exclusively inside function bodies; root instruction
        # scanning alone must not omit the required native operation helpers.
        labels = routine("labels", "names:List<Text>", "first:Text;result:List<Text>", "List<Text>", [
            op("LIST_GET", key="local.first", source="local.names", index="i:0", type="Text"),
            op("TEXT_JOIN", key="local.first", left="local.first", right="local.first"),
            op("LIST_SET", key="local.result", source="local.names", index="i:0", value="local.first", type="Text"),
            op("LIST_APPEND", key="local.result", source="local.result", value="local.first", type="Text"),
            returns("List<Text>", "f:local.result"),
        ])
        label_program = compile_program(root, "labels", [call("labels", "names=f:input.names", "out.names"), op("OUTPUT_FIELD", key="out.names")], [labels])
        for _ in range(100):
            names = [text() for _ in range(random.randrange(1, 8))]
            expected = [names[0] * 2] + names[1:] + [names[0] * 2]
            for backend in ["VM", "native"]:
                succeeded(run(label_program, backend, ["input.names=" + json.dumps(names)]), backend, f"out.names={canonical(expected)}\n".encode())
        print("PASS: 100 Text/List transformations using helpers referenced only by function bodies", flush=True)

        recursive = routine("repeat", "n:Int;text:Text", "child:Text;result:Text", "Text", [
            op("BRANCH_INT", left="f:local.n", right="i:0", comparison=">", label="recurse"),
            returns("Text", "f:local.text"), op("LABEL", name="recurse"),
            call("repeat", "n=f:local.n i:1 -;text=f:local.text", "local.child"),
            op("TEXT_JOIN", key="local.result", left="local.text", right="local.child"), returns("Text", "f:local.result"),
            op("EVAL_INT", key="local.n", expression="i:1 i:0 /"),
        ])
        repeat_program = compile_program(root, "repeat", [op("JSON_PARSE", key="input.text", source="text.raw", type="Text"), call("repeat", "n=f:input.n;text=f:input.text", "out.text"), op("OUTPUT_FIELD", key="out.text")], [recursive])
        for n in list(range(10)) + [63, 64]:
            raw = text()
            for backend in ["VM", "native"]:
                result = run(repeat_program, backend, ["input.n=" + str(n), "text.raw=" + json.dumps(raw)])
                if n == 64:
                    failed(result, backend, "DEPTH")
                else:
                    succeeded(result, backend, f"out.text={raw * (n + 1)}\n".encode())
        print("PASS: recursive local values, early return and exact 64/65-frame depth", flush=True)

        # Invalid unused parameters must still fail at invocation, with the
        # established type error families and no disclosure of their contents.
        bad_values = {
            "Int": ("INT", ["private_bad_integer", "9223372036854775808", " 1"]),
            "Bool": ("JSON", ["True", "0", " false"]),
            "List<Int>": ("LIST", ["[1,true]", "[01]", "[1,]"]),
            "List<Text>": ("LIST", ['["\\ud800"]', "[1]", "null"]),
            "Json": ("JSON", ['{"x":}', '"\\ud800"', "01"]),
        }
        for type_name, (category, values) in bad_values.items():
            program = compile_program(root, "invalid", [call("ignore", "value=f:input.value", "out.value")], [routine("ignore", "value:" + type_name, "", "Int", [returns("Int", "i:7")])])
            for arguments in [[]] + [["input.value=" + raw] for raw in values]:
                for backend in ["VM", "native"]:
                    result = run(program, backend, arguments)
                    failed(result, backend, category)
                    assert b"private_bad_integer" not in result.stdout + result.stderr
        text_program = compile_program(root, "invalid_text", [call("ignore", "value=f:input.value", "out.value")], [routine("ignore", "value:Text", "", "Int", [returns("Int", "i:7")])])
        failed(run(text_program, "native", [b"input.value=\xff"]), "native", "TEXT")
        order = compile_program(root, "argument_order", [call("ignore", "a=f:input.text;z=f:input.number", "out.value")], [routine("ignore", "a:Text;z:Int", "", "Int", [returns("Int", "i:7")])])
        for backend in ["VM", "native"]:
            failed(run(order, backend, ["input.number=bad"]), backend, "TEXT")
        print("PASS: invalid unused arguments, missing operands, sorted validation and native UTF-8 errors", flush=True)

        expanding = '["' + "\x7f" * 180_000 + '"]'
        for type_name, returned, category in [("List<Text>", False, None), ("List<Text>", True, "LIST-LIMIT"), ("Json", False, "JSON-LIMIT")]:
            result_type = type_name if returned else "Int"
            program = compile_program(root, "borrow_limits", [seed("input.value", expanding), call("inspect", "value=f:input.value", "out.value"), op("OUTPUT_FIELD", key="out.value")], [routine("inspect", "value:" + type_name, "", result_type, [returns(result_type, "f:local.value" if returned else "i:7")])])
            for backend in ["VM", "native"]:
                result = run(program, backend)
                if category:
                    failed(result, backend, category)
                else:
                    succeeded(result, backend, b"out.value=7\n")
        print("PASS: borrowed list input may exceed canonical size until returned; Json parameters remain bounded", flush=True)

        for type_name, raw, category in [("Text", "x" * LIMIT, "TEXT-LIMIT"), ("List<Text>", '["' + "x" * (LIMIT - 4) + '"]', "LIST-LIMIT"), ("Json", '"' + "x" * (LIMIT - 2) + '"', "JSON-LIMIT")]:
            for extra in [False, True]:
                identity = routine("identity", "value:" + type_name, "", type_name, [returns(type_name, "f:local.value")])
                forward = routine("forward", "value:" + type_name, "result:" + type_name, type_name, [call("identity", "value=f:local.value", "local.result"), returns(type_name, "f:local.result")])
                steps = [seed("input.value", raw)] + [call("forward", "value=f:input.value", "out.value") for _ in range(32)]
                if extra:
                    steps += [call("identity", "value=f:input.value", "out.value")]
                steps += [op("TEXT_LENGTH", key="out.count", source="out.value"), op("OUTPUT_FIELD", key="out.count")]
                program = compile_program(root, "return_budget", steps, [identity, forward])
                for backend in ["VM", "native"]:
                    result = run(program, backend)
                    if extra:
                        failed(result, backend, category)
                    else:
                        succeeded(result, backend, b"out.count=1048576\n")
        print("PASS: Text/List/Json nested returns each copy exactly once into shared 64 MiB arenas", flush=True)

        square = {"action": "square", "instructions": [op("INTEGER_FUNCTION", parameters="n"), op("RETURN_INT", expression="f:local.n f:local.n *")]}
        transform = routine("transform", "document:Json", "count:Int;names:List<Text>;path:Text;raw:Text;result:Json;value:Int", "Json", [
            op("ASSIGN_LITERAL", key="local.path", type="Text", value='"/value"'), op("JSON_GET", key="local.value", source="local.document", path="local.path", type="Int"),
            op("CALL_INT", target="square", arguments="n=f:local.value", key="local.value"),
            op("JSON_SET", key="local.result", source="local.document", path="local.path", value="local.value", type="Int"),
            op("ASSIGN_LITERAL", key="local.path", type="Text", value='"/private"'), op("JSON_REMOVE", key="local.result", source="local.result", path="local.path"),
            op("ASSIGN_LITERAL", key="local.path", type="Text", value='"/names"'), op("JSON_GET", key="local.names", source="local.result", path="local.path", type="List<Text>"),
            call("labels", "names=f:local.names", "local.names"),
            op("JSON_SET", key="local.result", source="local.result", path="local.path", value="local.names", type="List<Text>"),
            op("JSON_COUNT", key="local.count", source="local.result"), op("JSON_FORMAT", key="local.raw", source="local.result", type="Json"), op("JSON_PARSE", key="local.result", source="local.raw", type="Json"), returns("Json", "f:local.result"),
        ])
        dataset = compile_program(root, "dataset", [
            op("FILE_READ_TEXT", key="data.raw", resource="input"), op("JSON_PARSE", key="data.document", source="data.raw", type="Json"), op("JSON_COUNT", key="out.count", source="data.document"),
            op("ASSIGN_LITERAL", key="out.document", type="Json", value="[]"), op("EVAL_INT", key="data.index", expression="i:0"),
            op("ASSIGN_LITERAL", key="data.slash", type="Text", value='"/"'), op("ASSIGN_LITERAL", key="data.append", type="Text", value='"/-"'),
            op("LABEL", name="again"), op("BRANCH_INT", left="f:data.index", right="f:out.count", comparison=">=", label="done"),
            op("JSON_FORMAT", key="data.digits", source="data.index", type="Int"), op("TEXT_JOIN", key="data.path", left="data.slash", right="data.digits"),
            op("JSON_GET", key="data.record", source="data.document", path="data.path", type="Json"), call("transform", "document=f:data.record", "data.record"),
            op("JSON_SET", key="out.document", source="out.document", path="data.append", value="data.record", type="Json"),
            op("EVAL_INT", key="data.index", expression="f:data.index i:1 +"), op("JUMP", label="again"), op("LABEL", name="done"),
            op("JSON_FORMAT", key="data.report", source="out.document", type="Json"), op("FILE_WRITE_TEXT", source="data.report", resource="report"), op("OUTPUT_FIELD", key="out.count"),
        ], [transform, labels, square])
        input_file, report_file = root / "input.json", root / "report.json"
        grants = [f"--read-file=input={input_file}", f"--write-file=report={report_file}"]
        for index in range(50):
            records = [Pairs([("value", Number(str(random.randrange(-1000, 1000)))), ("private", text()), ("names", [text() for _ in range(random.randrange(1, 5))]), ("extra", nested())]) for _ in range(random.randrange(10))]
            input_file.write_text(emit(records, index % 2 == 0))
            expected = []
            for record in records:
                fields = dict(record)
                names = fields["names"]
                expected.append(Pairs([("value", Number(str(int(fields["value"]) ** 2))), ("names", [names[0] * 2] + names[1:] + [names[0] * 2]), ("extra", fields["extra"])]))
            for backend in ["VM", "native"]:
                succeeded(run(dataset, backend, grants), backend, f"out.count={len(records)}\n".encode())
                assert report_file.read_bytes() == canonical(expected).encode()
        print("PASS: 50 file reports composed from Json/List/Text functions and a pure integer helper", flush=True)


if __name__ == "__main__":
    main()
