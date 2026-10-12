# Conway Life

This application imports the pure [Life library](../ail_std_life.ail/README.md).
A grid is a row-major list of 0/1 integers with positive width and height and
exactly width × height cells. Exterior cells are dead. All eight neighbors read
the previous generation; the rule is B3/S23. Returned grids are immutable.

Run reads one raw UTF-8 JSON request through EOF and writes one compact response
plus one LF. `--stdio` grants standard input/output and suppresses runtime reports.
Run from the repository root:

```sh
target/debug/ail ail-run examples/conway-life --action Run --stdio < examples/conway-life/input.json
```

The [sample](input.json) is a horizontal 3×3 blinker after one generation. Its
[exact response](expected.json) is the vertical blinker with population 3.
Analyze returns only complete UTF-8 `out.result` Bytes and records Simulated:

```sh
target/debug/ail ail-run examples/conway-life --action Analyze 'data.request={"width":1,"height":1,"cells":[1],"generations":1}'
target/debug/ail ail-vm examples/ste/conway-life.ail-ste --action Run --stdio < examples/conway-life/input.json
target/debug/ail ail-compile examples/conway-life --action Run --target linux-x86_64-elf --out /tmp/conway-life
/tmp/conway-life --stdio < examples/conway-life/input.json
```

Success is `{"ok":true,"cells":C,"population":P,"error":""}`. Failure is
`{"ok":false,"cells":[],"population":0,"error":CATEGORY}`. Validation priority
is REQUEST (complete syntax/root/selected names/types), GRID (dimensions, exact
length and bits), then GENERATIONS (negative count). Integer -0 normalizes to 0.
Unrelated members and duplicates are ignored after complete Unicode validation.
Selected duplicates, including escaped aliases, are REQUEST. Zero generations
still validates the entire grid. Structured errors exit zero with empty stderr.
Missing grants, invalid UTF-8 and fatal runtime limits exit nonzero before output.

Import `ail.std.life` 0.1.0 as `Life` to call step, evolve, population and runRequest.
Direct domain errors use VALUE. The [API reference](../../docs/ail/65-conway-life.md)
explains composition, exact contracts and bounded immutable-list costs.
The [STE export](../ste/conway-life.ail-ste) includes the library.

Independent acceptance uses a frozen Python oracle and fourteen app plus fourteen
direct routes. Copy the harness to an immutable attempt-specific file and use a
fresh or empty artifact directory:

```sh
cp scripts/ail_life_differential.py /tmp/life-acceptance-attempt.py
AIL_LIFE_REPO="$PWD" python3 /tmp/life-acceptance-attempt.py --artifact-dir /tmp/life-acceptance-artifacts
```

Occupied or concurrently reserved artifact directories reject without modifying
their bytes. Finite subprocess/supervisor deadlines retain complete streams,
identities and failure records. Generated native programs need no Python.
