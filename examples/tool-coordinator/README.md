# Coordinate external tools with AIL

This program reads JSON task lines from stdin, runs a caller-bound executable,
and emits one JSON result per nonblank task. Each task supplies an `arguments`
array of strings, exact Text `input`, and a `timeout` in milliseconds. Results
contain `status`, `stdout`, `stderr`, and `error`. A launch/timeout/data error
uses a null status and a stable error category; a tool's nonzero exit is a normal
result. Successful tools and handled failures do not stop later tasks.

Use the existing AIL [JSON-lines filter](../json-lines-filter) as the worker:

```bash
cargo build
target/debug/ail ail-compile examples/json-lines-filter --action Filter \
  --target linux-x86_64-elf --out /tmp/ail-coordinator-worker
target/debug/ail ail-run examples/tool-coordinator --action Coordinate --stdio \
  --run-process=worker=/tmp/ail-coordinator-worker \
  < examples/tool-coordinator/input.jsonl

target/debug/ail ail-compile examples/tool-coordinator --action Coordinate \
  --target linux-x86_64-elf --out /tmp/ail-tool-coordinator
/tmp/ail-tool-coordinator --stdio \
  --run-process=worker=/tmp/ail-coordinator-worker \
  < examples/tool-coordinator/input.jsonl
```

Both coordinator and worker execute as standalone AIL binaries. Python and a
shell interpreter are not needed by their runtime. The sample includes accepted
and rejected worker rows, a worker option error, a recovered invalid timeout,
and another successful request after that failure. Export with `ail-ste` to
inspect the controlled-English intermediate.

The executable binding is caller authority. Argument strings never grant access
to another executable, although the chosen tool can use its ordinary host OS
permissions. Each child gets an empty environment, the inherited working
directory and private standard streams. This is synchronous tool execution;
see [process resources](../../docs/ail/53-process-resources.md) for cleanup,
signal, host and resource limits. Outer `--stdio` and the worker's `--stdio`
argument authorize their respective AIL programs independently.

Input and each captured stream are bounded to 1 MiB. The coordinator's encoded
JSON result must also fit the existing Json/Text limits; escaped large captures
can exceed that limit. A fatal limit or missing resource grant stops execution
and retains any earlier output records. The read consumes through EOF before
processing tasks, so this example is a bounded batch coordinator.
