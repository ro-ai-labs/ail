# Conway Life library

## Purpose

`ail.std.life` 0.1.0 supplies reusable pure functions for finite Conway Life
grids. Import this Application-profile package as `Life`. Its state is an
explicit immutable `List<Int>` passed between calls; no host or global grid
stores the simulation. This is a function support package, listed in the
[support manifest](../support-packages.md), rather than a new historical
[replay catalog](../examples.md) entry.

## Concepts Taught

The public APIs are `step(width:Int,height:Int,cells:List<Int>)->List<Int>`,
`evolve(width:Int,height:Int,cells:List<Int>,generations:Int)->List<Int>`,
`population(cells:List<Int>)->Int`, and `runRequest(text:Text)->Json`.

Grids use positive signed-i64 width and height, exactly width × height bits in
row-major order, and dead exterior cells. Eight neighboring coordinates exclude
self. All cells read the same previous generation: live cells survive with two
or three neighbors and dead cells are born with three (B3/S23). Step and evolve
share one transition implementation. Validation checks the dimension/length
relationship without multiplying untrusted dimensions.

Results are immutable canonical lists; retained inputs and earlier results stay
available for independent calls. Integer -0 normalizes to 0. Evolve validates the
whole grid before the generation count; zero generations returns canonical input
and still rejects an invalid grid. Population validates only bits, accepts an
empty list and returns 0 for it. For a valid grid, composing step with four more
generations equals evolving the original grid five times.

## Files To Inspect

[ail-package.md](ail-package.md) declares the package identity, profile and
features. [spec.ail-spec.md](spec.ail-spec.md) defines the four APIs, full
validation, bounded neighbor counting, shared generation and narrow request
recovery. The [application specification](../conway-life/spec.ail-spec.md)
imports Life for Analyze and the raw-stdin Run action.

The [benchmark program](../../bench/R025-conway-life/program.ail/spec.ail-spec.md)
uses the same Analyze body. Its [intent](../../bench/R025-conway-life/intent.md),
[independent oracle](../../bench/R025-conway-life/reference.py) and
[frozen requests](../../bench/R025-conway-life/cases) define semantic comparisons.
The [API guide](../../docs/ail/65-conway-life.md) explains the complete contract
and composing retained states.

## Expected Replay Artifacts

Run an action caller, such as [Conway Life](../conway-life/README.md), rather than
enter a library function directly from the CLI. Analyze accepts data.request
Text, selects only `out.result=hex:...` Bytes and records Simulated. Decoding the
complete UTF-8 bytes yields exactly
`{"ok":true,"cells":C,"population":P,"error":""}` on success, in that field order.
For example, a live 1×1 cell after one step yields cells `[0]` and population 0.

The application can produce checked Core, rendered Spec, exported STE, saved
bytecode and native ELF through the normal compiler paths. Its
[STE export](../ste/conway-life.ail-ste) includes the library. Run reads one raw
UTF-8 request through EOF and writes one compact response plus LF with --stdio.
Direct caller reports can inspect step, evolve, population and runRequest results
alongside retained inputs. These describe expected artifacts and interfaces;
this guide does not certify completion of an ongoing acceptance matrix.

## Rejected Fixtures

Invalid well-typed dimensions, length, bits or negative direct generations raise
catchable VALUE. Malformed typed transport may fail before a function body.
Requests require one complete JSON object with one unambiguous decoded occurrence
of width, height, cells and generations. Selected numbers must be strict signed-i64
integers and cells a flat integer array. Boolean/fractional/nested cells and selected
duplicate names, including escaped aliases, fail REQUEST. Extra members and
unrelated duplicates are ignored only after whole-document syntax and Unicode
validation; ignored invalid strings still fail.

Request error priority is REQUEST, GRID, then GENERATIONS. Thus nonarray cells
with negative generations is REQUEST, a well-typed invalid grid is GRID, and a
valid grid with a negative count is GENERATIONS. Failure is exactly
`{"ok":false,"cells":[],"population":0,"error":CATEGORY}`. Zero generations cannot
bypass validation. JSON/JSON-PATH recovery covers request extraction and VALUE
recovery covers grid validation; simulation, population, construction and output
remain outside those catches. Fatal runtime bounds propagate.

## Next Example To Read

Read the [Conway Life application](../conway-life/README.md) for raw stdin,
complete Bytes transport and replay commands. Then compare the
[immutable LRU library](../ail_std_lru.ail/README.md) for another explicit-state
API and [List Statistics](../list-statistics/README.md) for reusable typed-list
transformations.

## v0.3 Learning Signal

A pure explicit-state API composes validation, immutable lists, integer arithmetic,
finite neighbor loops and narrow recovery using existing AIL features. The frozen
rung includes dead, all-live, blinker, glider and mixed 20×20 grids for 50 generations,
a mixed 32×32 grid for 10, narrow 1×128 and 128×1 grids for 40, and a 1×1 grid for
257. Those useful-size checks describe examples, not interface caps or an
unbounded performance guarantee.

The algorithm retains current and next grids, but immutable append copies and
reparses lists; overwritten allocations remain charged. Existing finite instruction
fuel, 65,536 list items, 1 MiB value transport, 64 active function frames and 64 MiB
Text and List/Json arenas apply. Large valid requests can exhaust these fatal
limits. The package adds no host Life primitive, compiler opcode, budget increase,
arbitrary simulation cap or cycle shortcut.
