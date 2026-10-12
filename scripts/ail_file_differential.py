#!/usr/bin/env python3
"""Compare actual VM/native file effects with independent Python expectations.

Run cargo build first. All granted paths and artifacts live in a temporary
directory. This does not alter the frozen numeric capability ladder.
"""

import json
import os
from pathlib import Path
import random
import stat
import subprocess
import tempfile


REPO = Path(__file__).resolve().parent.parent
AIL = REPO / "target/debug/ail"
LIMIT = 1_048_576


def invoke(arguments):
    return subprocess.run(arguments, cwd=REPO, capture_output=True, timeout=15)


def op(opcode, **operands):
    return {"opcode": opcode, "operands": operands}


def compile_program(root, name, instructions):
    source = root / f"{name}.json"
    source.write_text(json.dumps({
        "kind": "AIL-Bytecode", "package": "file-oracle", "version": "1",
        "profile": "Application", "capability_grants": [], "target_support": {},
        "external_bindings": [], "failures": [],
        "actions": [{"action": "Run", "instructions": instructions + [op("RETURN_SUCCESS")]}],
    }))
    binary = root / f"{name}.elf"
    result = invoke([AIL, "ail-compile", source, "--action", "Run", "--target",
                     "linux-x86_64-elf", "--out", binary])
    assert result.returncode == 0, (name, result.stdout, result.stderr)
    return source, binary


def run(program, backend, arguments):
    source, binary = program
    command = [AIL, "ail-vm", source, "--action", "Run"] if backend == "VM" else [binary]
    return invoke(command + arguments)


def grant(direction, name, path):
    return f"--{direction}-file={name}={path}"


def succeeded(result, backend, expected):
    assert result.returncode == 0, (backend, result.stdout, result.stderr)
    if backend == "VM":
        assert result.stdout.startswith(b"ail-vm succeeded\n" + expected), (result.stdout, expected)
    else:
        assert result.stdout == expected, (result.stdout, expected)


def failed(result, backend, category=None):
    assert result.returncode != 0, (backend, result.stdout, result.stderr)
    if category:
        assert category.encode() in result.stdout + result.stderr, (backend, category, result.stdout, result.stderr)
    if backend == "native":
        assert not result.stdout, result.stdout


def main():
    random.seed(735_912)
    with tempfile.TemporaryDirectory(prefix="ail-file-oracle-") as temporary:
        root = Path(temporary)
        input_path = root / "input with = signs.txt"
        output_path = root / "output with = signs.txt"
        copy_path = root / "immutable copy.txt"
        sentinel = b"previous content must be truncated or preserved as specified"
        suffix = "\0🙂"
        pipeline = compile_program(root, "transform", [
            op("FILE_READ_TEXT", resource="input", key="data.text"),
            op("COPY_FIELD", source="data.text", key="data.copy", text="copy input"),
            op("ASSIGN_LITERAL", key="data.suffix", type="Text", value=json.dumps(suffix)),
            op("TEXT_JOIN", left="data.text", right="data.suffix", key="data.text"),
            op("FILE_WRITE_TEXT", resource="output", source="data.text"),
            op("FILE_WRITE_TEXT", resource="copy", source="data.copy"),
            op("FILE_READ_TEXT", resource="output", key="data.reloaded"),
            op("TEXT_LENGTH", source="data.reloaded", key="out.count"),
            op("OUTPUT_FIELD", key="out.count"),
        ])
        grants = [grant("read", "input", input_path), grant("read", "output", output_path),
                  grant("write", "output", output_path), grant("write", "copy", copy_path)]
        alphabet = ["a", " ", "é", "🙂", "\U0010ffff", "\u0301", "\0", "\n", "\r", "\t",
                    "\x01", "\x7f", "\x85", '"', "\\", "=", "[", "]", " to ", "data.text"]
        values = ["", "\0", "é\0🙂\n", "line one\r\nline two", "\U0010ffff"]
        values += ["".join(random.choices(alphabet, k=random.randrange(80))) for _ in range(195)]
        for value in values:
            input_path.write_bytes(value.encode())
            for backend in ["VM", "native"]:
                output_path.write_bytes(sentinel)
                copy_path.write_bytes(sentinel)
                result = run(pipeline, backend, grants)
                succeeded(result, backend, f"out.count={len(value + suffix)}\n".encode())
                assert output_path.read_bytes() == (value + suffix).encode()
                assert copy_path.read_bytes() == value.encode()
                assert input_path.read_bytes() == value.encode()
                assert str(input_path).encode() not in result.stdout + result.stderr
                assert str(output_path).encode() not in result.stdout + result.stderr
        print(f"PASS: {len(values)} Python/VM/native Unicode, NUL, empty-file transforms and immutable copies")

        # Both resources point at the same file: opening for each operation must
        # observe the preceding completed write and begin reading from byte zero.
        shared = root / "shared.txt"
        same_path_grants = [grant("read", "input", shared), grant("read", "output", shared),
                            grant("write", "output", shared), grant("write", "copy", copy_path)]
        for backend in ["VM", "native"]:
            shared.write_bytes("same\0é".encode())
            result = run(pipeline, backend, same_path_grants)
            succeeded(result, backend, b"out.count=8\n")
            assert shared.read_bytes() == ("same\0é" + suffix).encode()
            assert copy_path.read_bytes() == "same\0é".encode()

        reader = compile_program(root, "read_write", [
            op("FILE_READ_TEXT", resource="input", key="data.text"),
            op("FILE_WRITE_TEXT", resource="output", source="data.text"),
            op("TEXT_LENGTH", source="data.text", key="out.count"),
            op("OUTPUT_FIELD", key="out.count"),
        ])
        read_write_grants = [grant("read", "input", input_path), grant("write", "output", output_path)]
        for data, category in [
            (b"", None), (b"x" * LIMIT, None), ("🙂".encode() * (LIMIT // 4), None),
            (b"x" * (LIMIT + 1), "AIL-RUNTIME-TEXT-LIMIT"),
            (b"\xff" * (LIMIT + 1), "AIL-RUNTIME-TEXT-LIMIT"),
            (b"\x80", "AIL-RUNTIME-TEXT:"), (b"\xc0\xaf", "AIL-RUNTIME-TEXT:"),
            (b"\xe0\x80\xaf", "AIL-RUNTIME-TEXT:"), (b"\xed\xa0\x80", "AIL-RUNTIME-TEXT:"),
            (b"\xf4\x90\x80\x80", "AIL-RUNTIME-TEXT:"), (b"\xf0\x9f\x99", "AIL-RUNTIME-TEXT:"),
        ]:
            input_path.write_bytes(data)
            for backend in ["VM", "native"]:
                output_path.write_bytes(sentinel)
                result = run(reader, backend, read_write_grants)
                if category:
                    failed(result, backend, category)
                    assert output_path.read_bytes() == sentinel
                else:
                    succeeded(result, backend, f"out.count={len(data.decode('utf-8'))}\n".encode())
                    assert output_path.read_bytes() == data
        print("PASS: exact/over byte limits, size-before-UTF-8 priority, and invalid UTF-8 rejection")

        input_path.write_bytes(b"visible")
        writer = compile_program(root, "write_only", [op("FILE_WRITE_TEXT", resource="output", source="data.text")])
        for backend in ["VM", "native"]:
            for bindings in [[], [grant("write", "input", input_path)],
                             [f"input={input_path}", f"read-file.input={input_path}", f"resource.input={input_path}"]]:
                output_path.write_bytes(sentinel)
                result = run(reader, backend, bindings + [grant("write", "output", output_path)])
                failed(result, backend, "AIL-RUNTIME-CAPABILITY")
                assert output_path.read_bytes() == sentinel
            for bindings in [[], [grant("read", "output", output_path)],
                             [f"output={output_path}", f"write-file.output={output_path}"]]:
                output_path.write_bytes(sentinel)
                failed(run(writer, backend, bindings + ["data.text=changed"]), backend, "AIL-RUNTIME-CAPABILITY")
                assert output_path.read_bytes() == sentinel
            output_path.write_bytes(sentinel)
            failed(run(writer, backend, [grant("write", "output", output_path)]), backend, "AIL-RUNTIME-TEXT:")
            assert output_path.read_bytes() == sentinel
            failed(run(writer, backend, []), backend, "AIL-RUNTIME-CAPABILITY")
            output_path.write_bytes(sentinel)
            # Valid grant precedes an invalid byte sequence supplied directly by
            # execve; no Python/Rust string coercion can hide malformed UTF-8.
            if backend == "native":
                failed(run(writer, backend, [grant("write", "output", output_path), b"data.text=\xff"]), backend, "AIL-RUNTIME-TEXT:")
                assert output_path.read_bytes() == sentinel

        # Binding syntax is checked independently of whether grants are used;
        # malformed later flags must not allow earlier writes to happen.
        malformed = ["--read-file", "--read-file=", "--read-file=input", "--read-file==path",
                     "--read-file=input=", "--read-file=a..b=path", "--read-file=é=path",
                     "--write-file", "--write-file=", "--write-file=output", "--write-file==path",
                     "--write-file=output=", "--write-file=a-b=path"]
        for backend in ["VM", "native"]:
            for argument in malformed:
                output_path.write_bytes(sentinel)
                result = run(reader, backend, read_write_grants + [argument])
                failed(result, backend)
                assert output_path.read_bytes() == sentinel, argument
            decoy = root / "decoy.txt"
            decoy.write_bytes(b"decoy")
            output_path.write_bytes(sentinel)
            result = run(reader, backend, [grant("read", "input", decoy), grant("write", "output", decoy)] + read_write_grants)
            succeeded(result, backend, b"out.count=7\n")
            assert output_path.read_bytes() == b"visible"
            assert decoy.read_bytes() == b"decoy"
        print("PASS: denied/reversed/spoofed grants, malformed binding flags, and duplicate last-wins bindings")

        # With no selected output fields, grant flags must still stay outside
        # the runtime state emitted by the program.
        default_output = compile_program(root, "default_output", [op("FILE_READ_TEXT", resource="input", key="data.text")])
        for backend in ["VM", "native"]:
            result = run(default_output, backend, read_write_grants)
            succeeded(result, backend, b"data.text=visible\n")
            assert str(input_path).encode() not in result.stdout + result.stderr
            assert str(output_path).encode() not in result.stdout + result.stderr

        partial = compile_program(root, "later_error", [
            op("ASSIGN_LITERAL", key="data.text", type="Text", value='"committed"'),
            op("FILE_WRITE_TEXT", resource="output", source="data.text"),
            op("FILE_READ_TEXT", resource="denied", key="data.text"),
        ])
        for backend in ["VM", "native"]:
            output_path.write_bytes(sentinel)
            failed(run(partial, backend, [grant("write", "output", output_path)]), backend, "AIL-RUNTIME-CAPABILITY")
            assert output_path.read_bytes() == b"committed"

        directory = root / "directory"
        directory.mkdir()
        fifo = root / "fifo"
        os.mkfifo(fifo)
        socket_path = root / "socket"
        # A socket inode is sufficient for host open/type rejection; no network
        # service or socket permissions are needed by this file contract probe.
        os.mknod(socket_path, stat.S_IFSOCK | 0o600)
        for backend in ["VM", "native"]:
            for invalid in [directory, fifo, socket_path, Path("/dev/null"), root / "missing"]:
                failed(run(default_output, backend, [grant("read", "input", invalid)]), backend, "AIL-RUNTIME-IO")
            for invalid in [directory, fifo, socket_path, Path("/dev/null"), root / "missing-parent" / "file"]:
                failed(run(writer, backend, [grant("write", "output", invalid), "data.text=valid"]), backend, "AIL-RUNTIME-IO")
        assert not (root / "missing-parent").exists()
        link = root / "link"
        link.symlink_to(input_path)
        for backend in ["VM", "native"]:
            succeeded(run(default_output, backend, [grant("read", "input", link)]), backend, b"data.text=visible\n")
        print("PASS: default-state isolation, completed-write persistence, regular-file checks, nonblocking FIFOs, and host symlinks")


if __name__ == "__main__":
    main()
